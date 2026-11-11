//! Latencia de búsqueda con el tamaño objetivo del plan (~50.000 fragmentos).
//!
//! Tarda en construir el índice, así que no corre con `cargo test` normal:
//! `cargo test -p sift-search --release --test latency -- --ignored --nocapture`

use std::time::Instant;

#[path = "../benches/support/corpus.rs"]
mod corpus;

use sift_search::{Lexical, search};

fn percentile(sorted: &[f64], p: usize) -> f64 {
    sorted[((p * sorted.len()).div_ceil(100)).max(1) - 1]
}

#[test]
#[ignore = "lento: ejecutar a mano en release"]
fn p95_under_50ms_with_50k_chunks() {
    const FILES: i64 = 5_000;
    const CHUNKS_PER_FILE: i64 = 10;
    let started = Instant::now();
    let (idx, _) = corpus::text_index(FILES, CHUNKS_PER_FILE);
    println!(
        "índice: {} fragmentos en {:.1} s",
        FILES * CHUNKS_PER_FILE,
        started.elapsed().as_secs_f64()
    );
    // Palabras, prefijos y erratas (las frases del final buscan palabras que el
    // vocabulario sintético no tiene: son para la híbrida del benchmark).
    let queries = corpus::QUERIES
        .iter()
        .filter(|q| !q.starts_with("cómo") && !q.starts_with("conectar"));
    let lexical = Lexical(&idx);
    let mut times = Vec::new();
    for _ in 0..20 {
        for q in queries.clone() {
            let t = Instant::now();
            let r = search(&idx, &[&lexical], q, 8).unwrap();
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            assert!(!r.hits.is_empty(), "«{q}» no encontró nada");
        }
    }
    times.sort_by(f64::total_cmp);
    let (p50, p95) = (percentile(&times, 50), percentile(&times, 95));
    println!(
        "búsquedas: {} · p50 {p50:.1} ms · p95 {p95:.1} ms · máx {:.1} ms",
        times.len(),
        times[times.len() - 1]
    );
    assert!(p95 < 50.0, "p95 = {p95:.1} ms (objetivo < 50 ms)");
}
