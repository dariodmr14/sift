//! Benchmark de latencia de la búsqueda, reproducible: corpus sintético y determinista
//! (`corpus.rs`) del tamaño de la carpeta real (~100.000 fragmentos, 30.000 con vector).
//!
//! ```text
//! cargo bench -p sift-search --bench search
//! ```
//!
//! Mide cada pieza con `criterion` (mediana con intervalo de confianza; compara con la
//! ejecución anterior y avisa si algo empeora) y al final imprime p50/p95 de la búsqueda
//! completa, que es el objetivo del plan (< 50 ms p95).
//!
//! - **palabras**: solo Tantivy (búsqueda exacta, prefijos, erratas, frases relajadas).
//! - **vectores**: los 100 vecinos en HNSW.
//! - **consulta → vector**: el modelo de verdad (e5-small), si está descargado en
//!   `%APPDATA%\dev.sift.app\models` (lo baja la app la primera vez). Si no, se salta.
//! - **híbrida**: todo junto, con el modelo de verdad o, si no está, con un vector
//!   fijo por consulta (así se mide el resto aunque no haya modelo).

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "support/corpus.rs"]
mod corpus;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion};
use sift_embeddings::{EmbedError, Embedder, ModelCard, OnnxEmbedder, Purpose};
use sift_search::{Lexical, ParsedQuery, Retriever, Semantic, TextIndex, VectorIndex, search};

const FILES: i64 = 10_000;
const CHUNKS_PER_FILE: i64 = 10;
const VECTORS_PER_FILE: usize = 3;
/// Resultados que pide la barra.
const LIMIT: usize = 12;

/// Sin modelo: un vector fijo por consulta (determinista), sin coste de cálculo.
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn card(&self) -> &'static ModelCard {
        sift_embeddings::MODELS[0]
    }
    fn embed(&self, texts: &[&str], _: Purpose) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| corpus::Lcg(sift_storage::text_hash(t) as u64).unit_vector(384))
            .collect())
    }
}

fn real_embedder() -> Option<OnnxEmbedder> {
    let card = sift_embeddings::MODELS[0];
    let dir = PathBuf::from(std::env::var_os("APPDATA")?)
        .join("dev.sift.app")
        .join("models")
        .join(card.id);
    // Dos hilos, como las consultas en la app.
    OnnxEmbedder::load(card, &dir, 2).ok()
}

struct Setup {
    text: TextIndex,
    vectors: VectorIndex,
    real: Option<OnnxEmbedder>,
}

fn setup() -> Setup {
    let started = Instant::now();
    let (text, hashes) = corpus::text_index(FILES, CHUNKS_PER_FILE);
    let vectors = corpus::vector_index(&hashes, VECTORS_PER_FILE, 384);
    let real = real_embedder();
    eprintln!(
        "corpus: {} fragmentos, {} con vector, en {:.1} s · modelo: {}",
        FILES * CHUNKS_PER_FILE,
        vectors.len(),
        started.elapsed().as_secs_f64(),
        if real.is_some() {
            "e5-small"
        } else {
            "no está (vector fijo)"
        }
    );
    Setup {
        text,
        vectors,
        real,
    }
}

fn benches(c: &mut Criterion, s: &Setup) {
    let lexical = Lexical(&s.text);
    let mut words = c.benchmark_group("palabras");
    for q in [
        "normalización ",
        "normali",
        "dependensia funcional ",
        "cómo evitar redundancia en tablas",
    ] {
        let parsed = ParsedQuery::parse(q);
        words.bench_with_input(BenchmarkId::from_parameter(q.trim()), &parsed, |b, p| {
            b.iter(|| lexical.retrieve(p, 100).unwrap())
        });
    }
    words.finish();

    // Lo último de una búsqueda: cargar el texto de los que se muestran (el doble del
    // límite, para reordenar) y resaltar lo que encaja.
    let parsed = ParsedQuery::parse("normalización ");
    let keys: Vec<(i64, i64)> = lexical
        .retrieve(&parsed, 100)
        .unwrap()
        .iter()
        .map(|c| (c.file_id, c.chunk_id))
        .take(LIMIT * 2)
        .collect();
    c.bench_function("resultados/cargar 24", |b| {
        b.iter(|| s.text.fetch(&keys).unwrap())
    });
    let loaded = s.text.fetch(&keys).unwrap();
    c.bench_function("resultados/resaltar 24", |b| {
        b.iter(|| {
            loaded
                .iter()
                .map(|h| sift_search::snippet::highlight(&h.body, &parsed, 160))
                .collect::<Vec<_>>()
        })
    });

    let query = corpus::Lcg(1).unit_vector(384);
    c.bench_function("vectores/100 vecinos", |b| {
        b.iter(|| s.vectors.nearest(&query, 100).unwrap())
    });

    if let Some(real) = &s.real {
        c.bench_function("consulta → vector/e5-small", |b| {
            b.iter(|| {
                real.embed(&["cómo evitar redundancia en tablas"], Purpose::Query)
                    .unwrap()
            })
        });
    }

    let embedder: &dyn Embedder = match &s.real {
        Some(r) => r,
        None => &FixedEmbedder,
    };
    let semantic = Semantic {
        vectors: &s.vectors,
        text: &s.text,
        embedder,
        min_similarity: 0.0,
    };
    let mut hybrid = c.benchmark_group("híbrida");
    for q in ["normalización ", "cómo evitar redundancia en tablas"] {
        hybrid.bench_with_input(BenchmarkId::from_parameter(q.trim()), q, |b, q| {
            b.iter(|| search(&s.text, &[&lexical, &semantic], q, LIMIT).unwrap())
        });
    }
    hybrid.finish();
}

/// p50/p95 de la búsqueda completa con todas las consultas del corpus.
fn percentiles(s: &Setup) {
    let lexical = Lexical(&s.text);
    let embedder: &dyn Embedder = match &s.real {
        Some(r) => r,
        None => &FixedEmbedder,
    };
    let semantic = Semantic {
        vectors: &s.vectors,
        text: &s.text,
        embedder,
        min_similarity: 0.0,
    };
    let only_words: [&dyn Retriever; 1] = [&lexical];
    let both: [&dyn Retriever; 2] = [&lexical, &semantic];
    eprintln!("\n| búsqueda completa | p50 | p95 |\n|---|---|---|");
    for (label, retrievers) in [("solo palabras", &only_words[..]), ("híbrida", &both[..])] {
        let mut times: Vec<Duration> = Vec::new();
        for round in 0..21 {
            for q in corpus::QUERIES {
                let t = Instant::now();
                std::hint::black_box(search(&s.text, retrievers, q, LIMIT).unwrap());
                // La primera ronda calienta cachés: no cuenta.
                if round > 0 {
                    times.push(t.elapsed());
                }
            }
        }
        times.sort();
        let at = |p: f64| times[((times.len() - 1) as f64 * p).round() as usize];
        eprintln!(
            "| {label} | {:.1} ms | {:.1} ms |",
            at(0.50).as_secs_f64() * 1000.0,
            at(0.95).as_secs_f64() * 1000.0
        );
    }
}

fn main() {
    let s = setup();
    let mut c = Criterion::default()
        .configure_from_args()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));
    benches(&mut c, &s);
    c.final_summary();
    percentiles(&s);
}
