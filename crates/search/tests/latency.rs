//! Latencia de búsqueda con el tamaño objetivo del plan (~50.000 fragmentos).
//!
//! Tarda en construir el índice, así que no corre con `cargo test` normal:
//! `cargo test -p sift-search --release --test latency -- --ignored --nocapture`

use std::time::Instant;

use sift_indexer::{IndexSink, IndexedFile};
use sift_search::{Lexical, TextIndex, search};

/// Vocabulario de apuntes de la carrera, para generar texto con aspecto realista.
const WORDS: &[&str] = &[
    "normalización",
    "tabla",
    "clave",
    "primaria",
    "foránea",
    "dependencia",
    "funcional",
    "transacción",
    "concurrencia",
    "bloqueo",
    "índice",
    "consulta",
    "relación",
    "atributo",
    "protocolo",
    "red",
    "paquete",
    "enrutamiento",
    "capa",
    "transporte",
    "algoritmo",
    "grafo",
    "árbol",
    "complejidad",
    "recursión",
    "memoria",
    "proceso",
    "hilo",
    "sistema",
    "operativo",
    "compilador",
    "gramática",
    "autómata",
    "lenguaje",
    "función",
    "variable",
    "objeto",
    "clase",
    "herencia",
    "interfaz",
    "prueba",
    "requisito",
    "diseño",
    "arquitectura",
    "servidor",
    "cliente",
    "seguridad",
    "cifrado",
    "certificado",
    "examen",
    "práctica",
    "entrega",
];

/// Generador pseudoaleatorio simple y determinista (sin dependencias).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as usize
    }
}

fn percentile(sorted: &[f64], p: usize) -> f64 {
    sorted[((p * sorted.len()).div_ceil(100)).max(1) - 1]
}

#[test]
#[ignore = "lento: ejecutar a mano en release"]
fn p95_under_50ms_with_50k_chunks() {
    const FILES: i64 = 5_000;
    const CHUNKS_PER_FILE: i64 = 10;
    let mut rng = Lcg(42);
    let mut idx = TextIndex::in_memory().unwrap();
    let started = Instant::now();
    let mut chunk_id = 0;
    for file_id in 1..=FILES {
        let chunks = (0..CHUNKS_PER_FILE)
            .map(|_| {
                chunk_id += 1;
                let text: Vec<&str> = (0..180).map(|_| WORDS[rng.next() % WORDS.len()]).collect();
                (chunk_id, text.join(" "))
            })
            .collect();
        idx.upsert(&IndexedFile {
            file_id,
            path: format!(r"C:\Uni\apuntes\tema{file_id}.md"),
            name: format!("tema{file_id}.md"),
            ext: "md".into(),
            chunks,
        })
        .unwrap();
    }
    idx.commit().unwrap();
    println!(
        "índice: {chunk_id} fragmentos en {:.1} s",
        started.elapsed().as_secs_f64()
    );

    // Consultas variadas: palabras completas, prefijos (escribiendo) y erratas.
    let queries = [
        "normalización ",
        "clave primaria ",
        "transacción concurrencia ",
        "normali",
        "protoc",
        "arbol ",
        "complejidad algoritmo ",
        "dependensia funcional ",
        "enrutamento ",
        "cifr",
    ];
    let lexical = Lexical(&idx);
    let mut times = Vec::new();
    for _ in 0..20 {
        for q in queries {
            let t = Instant::now();
            let r = search(&[&lexical], q, 8).unwrap();
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
