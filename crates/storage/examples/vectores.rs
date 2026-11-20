//! Cuánto tardan las consultas del vectorizado con una carpeta grande (auditoría 1.0): con
//! ellas, el indexado tiene el candado de la base de datos y la barra espera.
//! `cargo run -p sift-storage --release --example vectores -- <carpeta temporal> [archivos]`

use std::path::{Path, PathBuf};
use std::time::Instant;

use sift_storage::{Database, EmbedPolicy, NewChunk, NewFile, text_hash};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(std::env::args().nth(1).ok_or("falta la carpeta")?);
    let files: usize = std::env::args().nth(2).map_or(Ok(100_000), |n| n.parse())?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("vectores.db");
    let fresh = !path.exists();
    let mut db = Database::open(&path)?;
    if fresh {
        let started = Instant::now();
        let root = db.add_root(Path::new(r"C:\Prueba"))?.id;
        let exts = ["md", "txt", "rs", "pdf"];
        for n in 0..files {
            let ext = exts[n % exts.len()];
            let chunks: Vec<NewChunk> = (0..(1 + n % 6))
                .map(|i| NewChunk {
                    page: None,
                    ordinal: i as i64,
                    start: 0,
                    text: format!("fragmento {n}-{i} ").repeat(60),
                })
                .collect();
            db.upsert_file(
                &NewFile {
                    root_id: root,
                    path: PathBuf::from(format!(r"C:\Prueba\a{}\f{n}.{ext}", n / 100)),
                    name: format!("f{n}.{ext}"),
                    ext: ext.into(),
                    size: 1,
                    mtime: n as i64,
                    content_hash: Some("h".into()),
                    error: None,
                    error_kind: None,
                },
                &chunks,
            )?;
        }
        println!("creada en {:.0} s", started.elapsed().as_secs_f64());
    }
    let policy = EmbedPolicy::standard();
    let time = |what: &str, f: &mut dyn FnMut()| {
        let started = Instant::now();
        f();
        println!("{what}: {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
    };
    let mut pending = Vec::new();
    time("pending_texts(64)", &mut || {
        pending = db.pending_texts("m", &policy, 64).unwrap_or_default();
    });
    time("embedding_progress", &mut || {
        let _ = db.embedding_progress("m", &policy);
    });
    let rows: Vec<(i64, Vec<f32>)> = pending.iter().map(|(h, _)| (*h, vec![0.1; 384])).collect();
    time("save_embeddings(64)", &mut || {
        let _ = db.save_embeddings("m", &rows);
    });
    time("pending_texts(64), otra vez", &mut || {
        let _ = db.pending_texts("m", &policy, 64);
    });
    time("orphan_embeddings", &mut || {
        let _ = db.orphan_embeddings("m", &policy);
    });
    let reader = Database::open_reader(&path)?;
    time("pending_texts(512) por el lector", &mut || {
        let _ = reader.pending_texts("m", &policy, 512);
    });
    let _ = text_hash("x");
    Ok(())
}
