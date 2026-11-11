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
    Frecency, Intent, Lexical, ParsedQuery, Retriever, Semantic, TextIndex, VectorIndex, search,
    search_with,
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
        min_similarity: std::env::var("SIFT_MIN_SIMILARITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(card.min_similarity),
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

/// Diagnóstico de una consulta que falla: dónde quedan los fragmentos de los archivos
/// esperados. Similitud **exacta** contra todos los vectores (no solo los 40 vecinos),
/// su puesto en HNSW, qué contienen y dónde los deja la fusión.
/// `SIFT_EVAL_QUERY="…" SIFT_EVAL_EXPECT="patrón,patrón" cargo test … -- --ignored diagnose --nocapture`
#[test]
#[ignore = "necesita la base de datos real y los modelos: ejecutar a mano en release"]
fn diagnose() {
    let query = std::env::var("SIFT_EVAL_QUERY")
        .unwrap_or_else(|_| "cómo evitar redundancia en tablas".into());
    let expect = std::env::var("SIFT_EVAL_EXPECT")
        .unwrap_or_else(|_| "%apuntes_bd_completos%,%Apuntes_TEST_B_Bases_de_Datos%".into());
    let db = snapshot();
    let card = sift_embeddings::MODELS[0];
    let embedder = OnnxEmbedder::load(card, &app_data().join("models").join(card.id), 4).unwrap();
    let q = embedder
        .embed(&[query.as_str()], sift_embeddings::Purpose::Query)
        .unwrap()
        .remove(0);

    // Todas las similitudes (exactas), para saber el puesto real de cada texto.
    let mut all: Vec<(i64, f32)> = Vec::new();
    db.for_each_embedding(card.id, |h, v| all.push((h, sift_embeddings::dot(&q, &v))))
        .unwrap();
    all.sort_by(|a, b| b.1.total_cmp(&a.1));
    let rank: HashMap<i64, (usize, f32)> = all
        .iter()
        .enumerate()
        .map(|(i, (h, s))| (*h, (i + 1, *s)))
        .collect();
    eprintln!("consulta: «{query}» · {} textos con vector", all.len());
    eprintln!(
        "parecido de los puestos 1, 10, 40, 100, 400: {:.3} {:.3} {:.3} {:.3} {:.3}",
        all[0].1, all[9].1, all[39].1, all[99].1, all[399].1
    );

    // Puesto en HNSW (lo que de verdad ve la búsqueda).
    let vectors = vector_index(&db, card);
    let hnsw: HashMap<i64, usize> = vectors
        .nearest(&q, 400)
        .unwrap()
        .into_iter()
        .enumerate()
        .map(|(i, (h, _))| (h, i + 1))
        .collect();

    // Los fragmentos de los archivos esperados.
    let copy = repo().join("target").join("eval").join("sift-eval.db");
    let conn =
        rusqlite::Connection::open_with_flags(&copy, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    for pattern in expect.split(',') {
        let mut stmt = conn
            .prepare(
                "SELECT f.path, c.ordinal, c.text_hash, c.text FROM chunks c JOIN files f ON f.id = c.file_id
                 WHERE f.path LIKE ?1 ORDER BY f.path, c.ordinal",
            )
            .unwrap();
        let rows: Vec<(String, i64, i64, String)> = stmt
            .query_map([pattern], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let files: std::collections::BTreeSet<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        eprintln!(
            "\n== {pattern}: {} archivos, {} fragmentos",
            files.len(),
            rows.len()
        );
        type Row = (String, i64, i64, String);
        let mut scored: Vec<(&Row, Option<(usize, f32)>)> =
            rows.iter().map(|r| (r, rank.get(&r.2).copied())).collect();
        let without = scored.iter().filter(|(_, r)| r.is_none()).count();
        eprintln!("   sin vector: {without}");
        scored.sort_by_key(|(_, r)| r.map_or(usize::MAX, |(p, _)| p));
        for (row, r) in scored.iter().take(6) {
            let (path, ordinal, hash, text) = row;
            let name = path.rsplit('\\').next().unwrap_or(path);
            let words: Vec<&str> = ["normaliz", "redund", "forma normal", "dependencia", "tabla"]
                .into_iter()
                .filter(|w| text.to_lowercase().contains(w))
                .collect();
            let snippet: String = text
                .chars()
                .take(140)
                .collect::<String>()
                .replace(['\n', '\r'], " ");
            match r {
                Some((p, sim)) => eprintln!(
                    "   puesto exacto {p:>5} · parecido {sim:.3} · HNSW {} · {name} #{ordinal} · {words:?}\n      «{snippet}»",
                    hnsw.get(hash).map_or("—".to_string(), |x| x.to_string())
                ),
                None => eprintln!("   sin vector · {name} #{ordinal} · {words:?}"),
            }
        }
        // ¿Algún fragmento habla de normalización? ¿Y dónde queda?
        let normal: Vec<_> = scored
            .iter()
            .filter(|(row, _)| row.3.to_lowercase().contains("normaliz"))
            .collect();
        eprintln!("   fragmentos que dicen «normaliz…»: {}", normal.len());
        if let Some((row, r)) = normal.first() {
            eprintln!(
                "   el mejor de ellos: puesto {:?} · {} chars · «{}»",
                r,
                row.3.chars().count(),
                row.3
                    .chars()
                    .take(300)
                    .collect::<String>()
                    .replace(['\n', '\r'], " ")
            );
        }
    }

    // Dónde deja la fusión a los archivos esperados (de los 50 primeros).
    let text = lexical_index(&db);
    let semantic = Semantic {
        vectors: &vectors,
        text: &text,
        embedder: &embedder,
        min_similarity: card.min_similarity,
    };
    let lexical = Lexical(&text);
    // Paso a paso: hashes vecinos → fragmentos (con sus copias) → archivos.
    let nearest = vectors.nearest(&q, 40).unwrap();
    let copies: Vec<usize> = {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM chunks WHERE text_hash = ?1")
            .unwrap();
        nearest
            .iter()
            .map(|(h, _)| stmt.query_row([h], |r| r.get::<_, i64>(0)).unwrap() as usize)
            .collect()
    };
    eprintln!(
        "\ncopias de cada uno de los 40 textos vecinos: {copies:?} · fragmentos en total: {}",
        copies.iter().sum::<usize>()
    );
    let target: Vec<i64> = {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT c.text_hash FROM chunks c JOIN files f ON f.id = c.file_id
                 WHERE f.path LIKE ?1",
            )
            .unwrap();
        let first = expect.split(',').next().unwrap_or_default();
        stmt.query_map([first], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let positions: Vec<usize> = nearest
        .iter()
        .enumerate()
        .filter(|(_, (h, _))| target.contains(h))
        .map(|(i, _)| i + 1)
        .collect();
    eprintln!("textos del primer archivo esperado entre los 40 vecinos: puestos {positions:?}");
    let candidates = semantic.retrieve(&ParsedQuery::parse(&query), 40).unwrap();
    let keys: Vec<(i64, i64)> = candidates.iter().map(|c| (c.file_id, c.chunk_id)).collect();
    let fetched = text.fetch(&keys).unwrap();
    let mut seen = std::collections::HashSet::new();
    let files: Vec<&str> = fetched
        .iter()
        .map(|h| h.name.as_str())
        .filter(|n| seen.insert(*n))
        .collect();
    eprintln!(
        "la semántica entrega {} fragmentos de {} archivos distintos: {:?}",
        candidates.len(),
        files.len(),
        files
    );
    let hits = search(&text, &[&lexical, &semantic], &query, 50)
        .unwrap()
        .hits;
    let words = search(&text, &[&lexical], &query, 50).unwrap().hits;
    for (label, list) in [("palabras", &words), ("fusión", &hits)] {
        eprintln!("\n{label} (top 12):");
        for (i, h) in list.iter().take(12).enumerate() {
            eprintln!("   {:>2}. {}", i + 1, h.name);
        }
        for pattern in expect.split(',') {
            let needle = pattern.trim_matches('%').to_lowercase();
            let pos = list
                .iter()
                .position(|h| h.name.to_lowercase().contains(&needle));
            eprintln!("   {pattern}: {:?}", pos.map(|p| p + 1));
        }
    }
    eprintln!("\nfusión (top 50):");
    for pattern in expect.split(',') {
        let needle = pattern.trim_matches('%').to_lowercase();
        let pos = hits
            .iter()
            .position(|h| h.name.to_lowercase().contains(&needle));
        eprintln!(
            "   {pattern}: {}",
            pos.map_or("fuera del top 50".to_string(), |p| format!(
                "puesto {}",
                p + 1
            ))
        );
    }
}

/// ¿Cuánto se equivoca HNSW? recall@40 frente a la búsqueda exacta, de media en todas las
/// consultas de `eval/`, y lo que tarda, para varios esfuerzos de búsqueda (`ef`).
/// `cargo test -p sift-search --release --test eval -- --ignored hnsw_recall --nocapture`
#[test]
#[ignore = "necesita la base de datos real y los modelos: ejecutar a mano en release"]
fn hnsw_recall() {
    let queries: QuerySet =
        serde_json::from_str(&std::fs::read_to_string(repo().join("eval/queries.json")).unwrap())
            .unwrap();
    let db = snapshot();
    let card = sift_embeddings::MODELS[0];
    let embedder = OnnxEmbedder::load(card, &app_data().join("models").join(card.id), 4).unwrap();
    let mut all: Vec<(i64, Vec<f32>)> = Vec::new();
    db.for_each_embedding(card.id, |h, v| all.push((h, v)))
        .unwrap();
    let vectors = vector_index(&db, card);
    let texts: Vec<&str> = queries.queries.iter().map(|q| q.query.as_str()).collect();
    let qs = embedder
        .embed(&texts, sift_embeddings::Purpose::Query)
        .unwrap();
    // Los 40 de verdad (exactos) de cada consulta.
    let exact: Vec<std::collections::HashSet<i64>> = qs
        .iter()
        .map(|q| {
            let mut s: Vec<(i64, f32)> = all
                .iter()
                .map(|(h, v)| (*h, sift_embeddings::dot(q, v)))
                .collect();
            s.sort_by(|a, b| b.1.total_cmp(&a.1));
            s.into_iter().take(40).map(|(h, _)| h).collect()
        })
        .collect();
    eprintln!(
        "
| ef | recall@40 de HNSW | tiempo medio por búsqueda |"
    );
    for ef in [64usize, 128, 256, 512, 1024] {
        vectors.set_expansion_search(ef);
        let started = Instant::now();
        let mut hit = 0usize;
        for (q, truth) in qs.iter().zip(&exact) {
            let got = vectors.nearest(q, 40).unwrap();
            hit += got.iter().filter(|(h, _)| truth.contains(h)).count();
        }
        let ms = started.elapsed().as_secs_f64() * 1000.0 / qs.len() as f64;
        eprintln!(
            "| {ef} | {:.2} | {ms:.2} ms |",
            hit as f64 / (40 * qs.len()) as f64
        );
    }
    let started = Instant::now();
    for q in &qs {
        let mut s: Vec<(i64, f32)> = all
            .iter()
            .map(|(h, v)| (*h, sift_embeddings::dot(q, v)))
            .collect();
        s.select_nth_unstable_by(39, |a, b| b.1.total_cmp(&a.1));
    }
    eprintln!(
        "| exacta (comparar con todos) | 1.00 | {:.2} ms |",
        started.elapsed().as_secs_f64() * 1000.0 / qs.len() as f64
    );
}

/// Calibra el peso de la frecencia. No hay aperturas reales suficientes para medirla (la
/// app lleva pocos días apuntándolas), así que se **simula** sobre `eval/`, con dos casos
/// que tiran en sentidos opuestos:
///
/// - **ayuda**: abriste hace poco uno de los archivos esperados (3 veces en la última
///   semana). Debería subir: mejora el MRR.
/// - **ruido**: abriste mucho otros archivos que salen en la búsqueda pero no son los
///   que buscas (los 5 primeros no esperados entre los puestos 6 y 30, 5 veces cada uno
///   esta semana). No debería echar a los buenos del top 10: baja el recall.
///
/// Se elige el peso que más ayuda sin que el ruido quite recall.
/// `cargo test -p sift-search --release --test eval -- --ignored calibrate_frecency --nocapture`
#[test]
#[ignore = "necesita la base de datos real y los modelos: ejecutar a mano en release"]
fn calibrate_frecency() {
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
    let retrievers: [&dyn Retriever; 2] = [&lexical, &semantic];
    let relative = |p: &str| {
        p.strip_prefix(root.as_str())
            .unwrap_or(p)
            .trim_start_matches('\\')
            .replace('\\', "/")
    };
    // Lo que sale sin frecencia (30 puestos) y qué se «abre» en cada caso.
    let mut cases = Vec::new();
    for (q, globs) in &m.0 {
        let base = search(&text, &retrievers, &q.query, 30).unwrap().hits;
        let expected =
            |h: &sift_search::FileHit| globs.iter().any(|g| g.is_match(relative(&h.path)));
        let helps: Vec<(i64, f64)> = base
            .iter()
            .find(|h| expected(h))
            .map(|h| [1.0, 3.0, 7.0].map(|age| (h.file_id, age)).to_vec())
            .unwrap_or_default();
        let noise: Vec<(i64, f64)> = base
            .iter()
            .skip(5)
            .filter(|h| !expected(h))
            .take(5)
            .flat_map(|h| [0.5, 1.0, 2.0, 4.0, 6.0].map(|age| (h.file_id, age)))
            .collect();
        cases.push((q, globs, helps, noise));
    }
    eprintln!("\n| peso | sin aperturas | ayuda | ruido |");
    eprintln!("|---|---|---|---|");
    for weight in [0.0, 0.25, 0.5, 0.75, 1.0, 2.0] {
        let mut out = [Score::default(); 3];
        for (q, globs, helps, noise) in &cases {
            for (i, opens) in [&Vec::new(), helps, noise].into_iter().enumerate() {
                let f = Frecency::from_opens(opens.iter().copied()).with_weight(weight);
                let hits = search_with(&text, &retrievers, &q.query, 10, &f)
                    .unwrap()
                    .hits;
                let found: Vec<String> = hits.iter().map(|h| relative(&h.path)).collect();
                let (recall, mrr) = grade(&found, globs);
                out[i].add(recall, mrr);
            }
        }
        eprintln!(
            "| {weight} | {} | {} | {} |",
            out[0].show(),
            out[1].show(),
            out[2].show()
        );
    }
}
