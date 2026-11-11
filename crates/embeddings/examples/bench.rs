//! Velocidad de cada modelo en este PC (ADR 0009, «medir antes de construir»).
//! `cargo run -p sift-embeddings --release --example bench`

use std::time::Instant;

use sift_embeddings::{Embedder, MODELS, OnnxEmbedder, Purpose};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Texto de apuntes de ~1.000 caracteres (el tamaño medio de un fragmento).
    let base = "La normalización de bases de datos es el proceso de organizar las tablas y sus columnas para reducir la redundancia y mejorar la integridad de los datos. La primera forma normal exige que cada atributo contenga valores atómicos; la segunda, que todo atributo no clave dependa de la clave completa; y la tercera, que no existan dependencias transitivas entre atributos no clave. ";
    let long: String = base.repeat(3).chars().take(1000).collect();
    let texts: Vec<String> = (0..64).map(|i| format!("{i} {long}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let dir = std::path::PathBuf::from(std::env::var_os("APPDATA").ok_or("falta APPDATA")?)
        .join("dev.sift.app/models");
    for card in MODELS {
        for threads in [2, 4, 8] {
            let e = OnnxEmbedder::load(card, &dir.join(card.id), threads)?;
            println!("{} · {} tokens por texto", card.id, e.count_tokens(&long));
            e.embed(&refs[..4], Purpose::Document)?; // calentar
            for batch in [1, 8, 32] {
                let t = Instant::now();
                for chunk in refs.chunks(batch) {
                    e.embed(chunk, Purpose::Document)?;
                }
                let rate = refs.len() as f64 / t.elapsed().as_secs_f64();
                println!("  {threads} hilos · lotes de {batch:>2}: {rate:5.1} textos/s");
            }
            let t = Instant::now();
            for _ in 0..20 {
                e.embed(&["cómo evitar redundancia en tablas"], Purpose::Query)?;
            }
            println!(
                "  consulta corta: {:.1} ms",
                t.elapsed().as_secs_f64() * 1000.0 / 20.0
            );
        }
    }
    Ok(())
}
