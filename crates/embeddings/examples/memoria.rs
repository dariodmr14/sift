//! Memoria que ocupa el modelo cargado como lo carga Sift (dos veces: indexado y consultas),
//! antes y después de vectorizar lotes de fragmentos largos (auditoría 1.0).
//! `cargo run -p sift-embeddings --release --example memoria -- <carpeta de modelos>`

use std::process::Command;

use sift_embeddings::{Embedder, MODELS, OnnxEmbedder, Purpose};

/// Memoria privada y conjunto de trabajo de este proceso, en MB.
fn memory() -> String {
    let pid = std::process::id();
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "$p = Get-Process -Id {pid}; '{{0}} MB privada · {{1}} MB en uso' -f [math]::Round($p.PrivateMemorySize64/1MB), [math]::Round($p.WorkingSet64/1MB)"
            ),
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("falta la carpeta")?);
    let card = MODELS[0];
    println!("antes de cargar: {}", memory());
    if std::env::var_os("SOLO_TOKENIZADOR").is_some() {
        let t = tokenizers::Tokenizer::from_file(dir.join(card.id).join(card.tokenizer.name))
            .map_err(|e| e.to_string())?;
        println!("con un tokenizador: {}", memory());
        let copies: Vec<_> = (0..3).map(|_| t.clone()).collect();
        println!("con cuatro tokenizadores: {}", memory());
        drop(copies);
        return Ok(());
    }
    let background = OnnxEmbedder::load(card, &dir.join(card.id), 2)?;
    let query = OnnxEmbedder::load(card, &dir.join(card.id), 2)?;
    println!("con los dos cargados: {}", memory());
    let long =
        "La normalización de bases de datos organiza las tablas para reducir la redundancia. "
            .repeat(40);
    let texts: Vec<String> = (0..8).map(|i| format!("{i} {long}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let started = std::time::Instant::now();
    for round in 1..=5 {
        background.embed(&refs, Purpose::Document)?;
        println!("tras {round} lotes de 8 × 512 tokens: {}", memory());
    }
    let docs = started.elapsed();
    let started = std::time::Instant::now();
    for _ in 0..20 {
        query.embed(&["cómo evitar redundancia en tablas"], Purpose::Query)?;
    }
    let queries = started.elapsed();
    println!("tras 20 consultas: {}", memory());
    println!(
        "lote de 8: {:.0} ms (con la medida de memoria) · consulta: {:.1} ms",
        docs.as_secs_f64() * 1000.0 / 5.0,
        queries.as_secs_f64() * 1000.0 / 20.0
    );
    Ok(())
}
