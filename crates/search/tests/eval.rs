//! Evaluación de la búsqueda con consultas reales (`eval/queries.json`, ver eval/README.md).
//!
//! Compara «solo palabras», «solo semántica» e «híbrida» con cada modelo descargado y
//! mide recall@10, MRR y latencia. Necesita la base de datos real de Sift y los modelos,
//! así que no corre con `cargo test` normal:
//!
//! ```text
//! cargo test -p sift-search --release --test eval -- --ignored --nocapture
//! ```
//!
//! No toca la base de datos real: trabaja sobre una copia en `target/eval/` (hecha con
//! `VACUUM INTO`, que da una foto coherente aunque la app esté abierta). Los vectores que
//! calcula se quedan en esa copia, así que la segunda vez no se recalculan.
//! Variables: `SIFT_EVAL_MODELS` (ids separados por comas), `SIFT_EVAL_REFRESH=1` (rehacer
//! la copia), `SIFT_EVAL_THREADS` (hilos para calcular, 4 por defecto).

// Es un test: si algo falla, que falle ruidosamente (como `allow-unwrap-in-tests`, que no
// cubre las funciones auxiliares de un archivo de tests de integración).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use sift_embeddings::{Embedder, ModelCard, OnnxEmbedder, Purpose};
use sift_indexer::{IndexSink, IndexedFile};
use sift_search::{
    Intent, Lexical, ParsedQuery, Retriever, Semantic, TextIndex, VectorIndex, search,
};
use sift_storage::{Database, EmbedPolicy};

#[derive(Deserialize)]
struct QuerySet {
    queries: Vec<EvalQuery>,
}

#[derive(Deserialize, Clone)]
struct EvalQuery {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    query: String,
    expect: Vec<String>,
}

fn app_data() -> PathBuf {
    PathBuf::from(std::env::var_os("APPDATA").expect("APPDATA")).join("dev.sift.app")
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Copia coherente de la base de datos real, abierta con Sift (migraciones incluidas).
fn snapshot() -> Database {
    let dir = repo().join("target").join("eval");
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("sift-eval.db");
    let refresh = std::env::var("SIFT_EVAL_REFRESH").is_ok_and(|v| v == "1");
    if refresh || !copy.is_file() {
        let _ = std::fs::remove_file(&copy);
        let source = rusqlite::Connection::open_with_flags(
            app_data().join("sift.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("no se pudo abrir la base de datos real");
        source
            .execute("VACUUM INTO ?1", [copy.display().to_string()])
            .unwrap();
        eprintln!("copia de la base de datos hecha en {}", copy.display());
    }
    Database::open(&copy).unwrap()
}

/// Índice de palabras en memoria, con todo lo de la copia.
fn lexical_index(db: &Database) -> TextIndex {
    let started = Instant::now();
    let mut index = TextIndex::in_memory().unwrap();
    db.for_each_file_with_chunks(|f, chunks| {
        index
            .upsert(&IndexedFile {
                file_id: f.id,
                path: f.path.clone(),
                name: f.name.clone(),
                ext: f.ext.clone(),
                chunks,
            })
            .unwrap();
    })
    .unwrap();
    index.commit().unwrap();
    eprintln!(
        "índice de palabras: {} documentos en {:.1} s",
        index.num_docs(),
        started.elapsed().as_secs_f64()
    );
    index
}

/// Qué se vectoriza en la evaluación: por defecto, lo mismo que la app (documentos,
/// apuntes, código y `.txt` con su tope), para que las cifras sean las de verdad.
/// `SIFT_EVAL_POLICY=docs` se queda con documentos y apuntes: sirvió para comparar modelos
/// sin pasar una hora vectorizando código.
fn eval_policy() -> EmbedPolicy {
    let mut policy = EmbedPolicy::standard();
    if std::env::var("SIFT_EVAL_POLICY").as_deref() == Ok("docs") {
        policy.tiers.truncate(2);
    }
    policy
}

/// ¿Cuántos fragmentos pasan del máximo de tokens del modelo? (se cortan al vectorizar)
fn token_stats(db: &Database, embedder: &OnnxEmbedder, card: &ModelCard) {
    let standard = EmbedPolicy::standard();
    for (i, (exts, _)) in standard.tiers.iter().enumerate() {
        let tier = EmbedPolicy {
            tiers: vec![standard.tiers[i].clone()],
        };
        let sample = db.pending_texts("__muestra__", &tier, 1500).unwrap();
        let counts: Vec<usize> = sample
            .iter()
            .map(|(_, t)| embedder.count_tokens(t))
            .collect();
        let over = counts.iter().filter(|&&n| n > card.max_tokens).count();
        let mean = counts.iter().sum::<usize>() as f64 / counts.len().max(1) as f64;
        eprintln!(
            "  tokens ({}…): media {mean:.0}, pasan de {}: {:.1} %",
            exts[0],
            card.max_tokens,
            100.0 * over as f64 / counts.len().max(1) as f64
        );
    }
}

/// Calcula los vectores que falten en la copia, con progreso y velocidad.
fn embed_all(db: &mut Database, embedder: &OnnxEmbedder, card: &ModelCard) {
    const BATCH: usize = 16;
    let policy = eval_policy();
    let (done, total) = db.embedding_progress(card.id, &policy).unwrap();
    if done == total {
        eprintln!("{}: {total} vectores ya calculados", card.id);
        return;
    }
    eprintln!("{}: faltan {} de {total} vectores", card.id, total - done);
    let started = Instant::now();
    let mut made = 0usize;
    loop {
        let mut pending = db.pending_texts(card.id, &policy, 512).unwrap();
        if pending.is_empty() {
            break;
        }
        // Textos de longitud parecida en el mismo lote: cada lote se rellena hasta el más
        // largo, y el relleno también cuesta cálculo.
        pending.sort_by_key(|(_, t)| t.len());
        for batch in pending.chunks(BATCH) {
            let texts: Vec<&str> = batch.iter().map(|(_, t)| t.as_str()).collect();
            let vectors = embedder.embed(&texts, Purpose::Document).unwrap();
            let rows: Vec<(i64, Vec<f32>)> = batch.iter().map(|(h, _)| *h).zip(vectors).collect();
            db.save_embeddings(card.id, &rows).unwrap();
            made += rows.len();
        }
        let secs = started.elapsed().as_secs_f64();
        let rate = made as f64 / secs;
        let left = (total - done) as f64 - made as f64;
        eprintln!(
            "  {made} vectores en {secs:.0} s · {rate:.1}/s · quedan ~{:.0} min",
            left / rate / 60.0
        );
    }
}

fn vector_index(db: &Database, card: &ModelCard) -> VectorIndex {
    let started = Instant::now();
    let index = VectorIndex::new(card.dims).unwrap();
    let mut batch = Vec::new();
    db.for_each_embedding(card.id, |h, v| batch.push((h, v)))
        .unwrap();
    index.add(&batch).unwrap();
    eprintln!(
        "{}: índice de vectores con {} en {:.1} s",
        card.id,
        index.len(),
        started.elapsed().as_secs_f64()
    );
    index
}

/// Hace que un recuperador pese siempre lo mismo (para «solo semántica», que si no se
/// apaga sola en las búsquedas exactas).
struct Always<'a>(&'a dyn Retriever);

impl Retriever for Always<'_> {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn weight(&self, _: Intent) -> f64 {
        1.0
    }
    fn by_meaning(&self) -> bool {
        self.0.by_meaning()
    }
    fn retrieve(
        &self,
        q: &ParsedQuery,
        k: usize,
    ) -> Result<Vec<sift_search::Candidate>, sift_search::IndexError> {
        self.0.retrieve(q, k)
    }
}

#[derive(Default, Clone, Copy)]
struct Score {
    recall: f64,
    mrr: f64,
    n: usize,
}

impl Score {
    fn add(&mut self, recall: f64, mrr: f64) {
        self.recall += recall;
        self.mrr += mrr;
        self.n += 1;
    }
    fn show(&self) -> String {
        if self.n == 0 {
            return "—".into();
        }
        format!(
            "{:.2} ({:.2})",
            self.recall / self.n as f64,
            self.mrr / self.n as f64
        )
    }
}

struct Matchers(Vec<(EvalQuery, Vec<GlobMatcher>)>);

fn matchers(queries: &[EvalQuery]) -> Matchers {
    Matchers(
        queries
            .iter()
            .map(|q| {
                let globs = q
                    .expect
                    .iter()
                    .map(|p| {
                        GlobBuilder::new(p)
                            .case_insensitive(true)
                            .literal_separator(false)
                            .build()
                            .unwrap()
                            .compile_matcher()
                    })
                    .collect();
                (q.clone(), globs)
            })
            .collect(),
    )
}

/// (recall@10, MRR) de una lista de rutas relativas.
fn grade(found: &[String], globs: &[GlobMatcher]) -> (f64, f64) {
    let met = globs
        .iter()
        .filter(|g| found.iter().any(|p| g.is_match(p)))
        .count();
    let first = found
        .iter()
        .position(|p| globs.iter().any(|g| g.is_match(p)));
    (
        met as f64 / globs.len() as f64,
        first.map_or(0.0, |i| 1.0 / (i as f64 + 1.0)),
    )
}

fn run(
    label: &str,
    text: &TextIndex,
    retrievers: &[&dyn Retriever],
    m: &Matchers,
    root: &str,
    misses: bool,
) -> (HashMap<String, Score>, Vec<f64>) {
    let mut scores: HashMap<String, Score> = HashMap::new();
    let mut times = Vec::new();
    for (q, globs) in &m.0 {
        let started = Instant::now();
        let hits = search(text, retrievers, &q.query, 10).unwrap().hits;
        times.push(started.elapsed().as_secs_f64() * 1000.0);
        let found: Vec<String> = hits
            .iter()
            .map(|h| {
                h.path
                    .strip_prefix(root)
                    .unwrap_or(&h.path)
                    .trim_start_matches('\\')
                    .replace('\\', "/")
            })
            .collect();
        let (recall, mrr) = grade(&found, globs);
        scores.entry(q.kind.clone()).or_default().add(recall, mrr);
        scores.entry("total".into()).or_default().add(recall, mrr);
        if misses && recall < 1.0 {
            let top: Vec<&str> = found
                .iter()
                .take(3)
                .map(|s| s.rsplit('/').next().unwrap_or(s))
                .collect();
            eprintln!(
                "    [{label}] {:<18} {recall:.2} · {}",
                q.id,
                top.join(" | ")
            );
        }
    }
    (scores, times)
}

fn percentile(times: &mut [f64], p: f64) -> f64 {
    times.sort_by(f64::total_cmp);
    times[((times.len() - 1) as f64 * p).round() as usize]
}

fn row(label: &str, s: &HashMap<String, Score>) {
    let get = |k: &str| s.get(k).copied().unwrap_or_default().show();
    eprintln!(
        "| {label:<34} | {:<11} | {:<11} | {:<11} | {:<11} |",
        get("concept"),
        get("topic"),
        get("exact"),
        get("total")
    );
}

#[test]
#[ignore = "necesita la base de datos real y los modelos: ejecutar a mano en release"]
fn recall_at_10_words_meaning_and_hybrid() {
    let queries: QuerySet =
        serde_json::from_str(&std::fs::read_to_string(repo().join("eval/queries.json")).unwrap())
            .unwrap();
    let m = matchers(&queries.queries);
    let mut db = snapshot();
    let root = db.list_roots().unwrap()[0].path.display().to_string();
    let text = lexical_index(&db);

    eprintln!(
        "\n| configuración                      | concept     | topic       | exact       | total       |"
    );
    eprintln!(
        "|------------------------------------|-------------|-------------|-------------|-------------|"
    );
    let lexical = Lexical(&text);
    let (words, _) = run("palabras", &text, &[&lexical], &m, &root, false);
    row("solo palabras", &words);

    let wanted = std::env::var("SIFT_EVAL_MODELS").unwrap_or_else(|_| "e5-small".into());
    let threads = std::env::var("SIFT_EVAL_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let mut report = Vec::new();
    for id in wanted.split(',') {
        let card = sift_embeddings::model(id.trim()).expect("modelo desconocido");
        let dir = app_data().join("models").join(card.id);
        let embedder = match OnnxEmbedder::load(card, &dir, threads) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("se salta {}: {e}", card.id);
                continue;
            }
        };
        if std::env::var("SIFT_EVAL_TOKENS").is_ok_and(|v| v == "1") {
            token_stats(&db, &embedder, card);
        }
        // Fuera lo que la política ya no vectoriza (p. ej. lo de una regla anterior).
        let pruned = db.prune_embeddings(card.id, &eval_policy()).unwrap();
        if !pruned.is_empty() {
            eprintln!(
                "{}: {} vectores fuera de la política, quitados",
                card.id,
                pruned.len()
            );
        }
        embed_all(&mut db, &embedder, card);
        let vectors = vector_index(&db, card);

        // Umbral de parecido: se prueban varios y se queda el mejor híbrido que no
        // empeore las búsquedas exactas respecto a solo palabras.
        let mut best: Option<(f32, HashMap<String, Score>)> = None;
        for threshold in [0.0f32, 0.3, 0.4, 0.5, 0.6, 0.7, 0.75, 0.8, 0.85] {
            let semantic = Semantic {
                vectors: &vectors,
                text: &text,
                embedder: &embedder,
                min_similarity: threshold,
            };
            let (hybrid, _) = run("híbrida", &text, &[&lexical, &semantic], &m, &root, false);
            let total = |s: &HashMap<String, Score>| s["total"].recall / s["total"].n as f64;
            let exact =
                |s: &HashMap<String, Score>| s.get("exact").map_or(0.0, |x| x.recall / x.n as f64);
            if exact(&hybrid) + 1e-9 >= exact(&words)
                && best
                    .as_ref()
                    .is_none_or(|(_, b)| total(&hybrid) > total(b) + 1e-9)
            {
                best = Some((threshold, hybrid));
            }
        }
        let (threshold, hybrid) = best.expect("ningún umbral");
        let semantic = Semantic {
            vectors: &vectors,
            text: &text,
            embedder: &embedder,
            min_similarity: threshold,
        };
        let only = Always(&semantic);
        let (meaning, _) = run(card.id, &text, &[&only], &m, &root, false);
        row(&format!("solo semántica · {}", card.id), &meaning);
        row(
            &format!("híbrida · {} (umbral {threshold})", card.id),
            &hybrid,
        );

        // Latencia de la híbrida: cada consulta 5 veces, con 2 hilos (como en la app).
        let app_embedder = OnnxEmbedder::load(card, &dir, 2).unwrap();
        let semantic = Semantic {
            vectors: &vectors,
            text: &text,
            embedder: &app_embedder,
            min_similarity: threshold,
        };
        let full: Vec<&dyn Retriever> = vec![&lexical, &semantic];
        let mut times = Vec::new();
        for _ in 0..5 {
            let (_, t) = run("lat", &text, &full, &m, &root, false);
            times.extend(t);
        }
        let (p50, p95) = (percentile(&mut times, 0.5), percentile(&mut times, 0.95));
        // Lo que tarda en salir lo primero: solo palabras (lo que el plan B enseña ya).
        let mut first = Vec::new();
        for _ in 0..5 {
            let (_, t) = run("lat", &text, &[&lexical], &m, &root, false);
            first.extend(t);
        }
        report.push(format!(
            "{}: híbrida p50 {p50:.1} ms · p95 {p95:.1} ms · solo palabras p50 {:.1} ms · p95 {:.1} ms",
            card.id,
            percentile(&mut first, 0.5),
            percentile(&mut first, 0.95)
        ));
        eprintln!("  fallos de la híbrida con {}:", card.id);
        run(card.id, &text, &full, &m, &root, true);
    }
    eprintln!("\nrecall@10 (MRR entre paréntesis)");
    for line in report {
        eprintln!("{line}");
    }
}

/// Los 10 primeros de una consulta, con su fragmento: para ver por qué falla una.
/// `SIFT_EVAL_QUERY="…" cargo test -p sift-search --release --test eval -- --ignored show_one --nocapture`
#[test]
#[ignore = "necesita la base de datos real y los modelos: ejecutar a mano en release"]
fn show_one() {
    let Ok(query) = std::env::var("SIFT_EVAL_QUERY") else {
        return;
    };
    let db = snapshot();
    let text = lexical_index(&db);
    let card = sift_embeddings::MODELS[0];
    let embedder = OnnxEmbedder::load(card, &app_data().join("models").join(card.id), 2).unwrap();
    let vectors = vector_index(&db, card);
    let semantic = Semantic {
        vectors: &vectors,
        text: &text,
        embedder: &embedder,
        min_similarity: card.min_similarity,
    };
    let lexical = Lexical(&text);
    // Lo que da la semántica por separado.
    let parsed = ParsedQuery::parse(&query);
    {
        let keys: Vec<(i64, i64)> = semantic
            .retrieve(&parsed, 40)
            .unwrap()
            .iter()
            .map(|c| (c.file_id, c.chunk_id))
            .collect();
        let names: Vec<String> = text
            .fetch(&keys)
            .unwrap()
            .into_iter()
            .map(|h| h.name)
            .collect();
        let mut seen = std::collections::HashSet::new();
        let unique: Vec<&String> = names
            .iter()
            .filter(|n| seen.insert(n.as_str()))
            .take(8)
            .collect();
        eprintln!("[semántica] {unique:?}");
    }
    for h in search(&text, &[&lexical, &semantic], &query, 10)
        .unwrap()
        .hits
    {
        let snippet: String = h.snippet.iter().map(|s| s.text.as_str()).collect();
        let snippet: String = snippet.chars().take(110).collect();
        eprintln!(
            "{} {} — {}",
            if h.by_meaning { "≈" } else { "=" },
            h.name,
            snippet.replace('\n', " ")
        );
    }
}

/// Un recuperador con otro peso (solo donde el original pesa algo): para calibrar.
struct Weighted<'a>(&'a dyn Retriever, f64);

impl Retriever for Weighted<'_> {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn weight(&self, intent: Intent) -> f64 {
        if self.0.weight(intent) > 0.0 {
            self.1
        } else {
            0.0
        }
    }
    fn by_meaning(&self) -> bool {
        self.0.by_meaning()
    }
    fn retrieve(
        &self,
        q: &ParsedQuery,
        k: usize,
    ) -> Result<Vec<sift_search::Candidate>, sift_search::IndexError> {
        self.0.retrieve(q, k)
    }
}

/// Calibra el peso de la semántica en la fusión con `eval/` (las palabras pesan 1).
/// `cargo test -p sift-search --release --test eval -- --ignored calibrate --nocapture`
///
/// Se probó también un tercer recuperador, «alguna de las palabras» (BM25 con cualquiera
/// de los términos), con pesos de 0 a 1: no mejoraba nada y se descartó (ADR 0009).
#[test]
#[ignore = "necesita la base de datos real y los modelos: ejecutar a mano en release"]
fn calibrate_weights() {
    let queries: QuerySet =
        serde_json::from_str(&std::fs::read_to_string(repo().join("eval/queries.json")).unwrap())
            .unwrap();
    let m = matchers(&queries.queries);
    let db = snapshot();
    let root = db.list_roots().unwrap()[0].path.display().to_string();
    let text = lexical_index(&db);
    let card = sift_embeddings::MODELS[0];
    let embedder = OnnxEmbedder::load(card, &app_data().join("models").join(card.id), 4).unwrap();
    let vectors = vector_index(&db, card);
    let semantic = Semantic {
        vectors: &vectors,
        text: &text,
        embedder: &embedder,
        min_similarity: card.min_similarity,
    };
    let lexical = Lexical(&text);
    eprintln!(
        "
| semántica | concept | topic | exact | total |"
    );
    for sem_w in [0.3, 0.5, 0.7, 1.0] {
        let b = Weighted(&semantic, sem_w);
        let (s, _) = run("cal", &text, &[&lexical, &b], &m, &root, false);
        let get = |k: &str| s.get(k).copied().unwrap_or_default().show();
        eprintln!(
            "| {sem_w} | {} | {} | {} | {} |",
            get("concept"),
            get("topic"),
            get("exact"),
            get("total")
        );
    }
}
