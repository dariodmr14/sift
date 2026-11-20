//! Descarga del modelo de búsqueda inteligente: **la única conexión a internet de Sift**
//! (CLAUDE.md: «nada de red salvo descargar el modelo de embeddings la primera vez»).
//!
//! - Solo se bajan las URLs de la ficha del modelo (`sift_embeddings::ModelCard`), y cada
//!   archivo se comprueba con su SHA-256 antes de usarlo. Si no coincide, se borra.
//! - Se baja a `<archivo>.part` y se renombra al acabar: un archivo con su nombre final
//!   siempre está completo y verificado.
//! - Si se corta (cerrar Sift, perder la conexión), la siguiente vez sigue donde iba
//!   (cabecera `Range`).

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

/// Tiempos máximos de la descarga (auditoría 1.0): sin ellos, una conexión colgada dejaba
/// el indexador parado hasta reiniciar. El cuerpo tiene margen para conexiones lentas
/// (~135 MB); si se corta, la próxima vez se sigue donde se quedó (`Range`).
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20 * 60);
use sift_embeddings::{ModelCard, ModelFile};

/// Carpeta de un modelo dentro de la de datos de la app.
pub fn model_dir(app_data: &Path, card: &ModelCard) -> PathBuf {
    app_data.join("models").join(card.id)
}

/// ¿Están ya todos los archivos del modelo? (Con su nombre final = verificados.)
pub fn is_ready(dir: &Path, card: &ModelCard) -> bool {
    card.files().iter().all(|f| dir.join(f.name).is_file())
}

#[derive(Debug)]
pub enum DownloadError {
    Network(String),
    /// El archivo descargado no es el esperado (SHA-256 distinto).
    Corrupt(String),
    Io(std::io::Error),
    Cancelled,
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Network(e) => write!(f, "sin conexión o el servidor no responde: {e}"),
            DownloadError::Corrupt(name) => {
                write!(f, "{name} no es el archivo esperado (SHA-256 distinto)")
            }
            DownloadError::Io(e) => write!(f, "no se pudo escribir en disco: {e}"),
            DownloadError::Cancelled => write!(f, "cancelado"),
        }
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        DownloadError::Io(e)
    }
}

/// Descarga lo que falte del modelo. `progress(bytes descargados, bytes totales)`.
pub fn download(
    dir: &Path,
    card: &ModelCard,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), DownloadError> {
    std::fs::create_dir_all(dir)?;
    let total = card.download_bytes();
    let mut before = 0;
    for file in card.files() {
        let target = dir.join(file.name);
        if !target.is_file() {
            fetch(&target, file, cancel, |n| progress(before + n, total))?;
        }
        before += file.bytes;
        progress(before, total);
    }
    Ok(())
}

fn fetch(
    target: &Path,
    file: &ModelFile,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<(), DownloadError> {
    let part = target.with_extension(format!(
        "{}.part",
        target.extension().map_or("", |e| e.to_str().unwrap_or(""))
    ));
    let have = std::fs::metadata(&part).map_or(0, |m| m.len());
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_recv_response(Some(RESPONSE_TIMEOUT))
        .timeout_recv_body(Some(BODY_TIMEOUT))
        .build()
        .into();
    let mut request = agent.get(file.url).header("User-Agent", "Sift");
    if have > 0 && have < file.bytes {
        request = request.header("Range", format!("bytes={have}-"));
    }
    let response = request
        .call()
        .map_err(|e| DownloadError::Network(e.to_string()))?;
    // 206 = el servidor acepta seguir; si no, se empieza de cero.
    let resumed = response.status() == 206 && have > 0;
    let mut out = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&part)?;
    let mut written = if resumed { have } else { 0 };
    let mut body = response.into_body().into_reader();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let n = body
            .read(&mut buf)
            .map_err(|e| DownloadError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        written += n as u64;
        progress(written);
    }
    out.flush()?;
    drop(out);
    verify_and_finish(&part, target, file)
}

/// Comprueba el SHA-256 de lo descargado y, si es el bueno, le pone su nombre final.
fn verify_and_finish(part: &Path, target: &Path, file: &ModelFile) -> Result<(), DownloadError> {
    let mut hasher = Sha256::new();
    std::io::copy(&mut std::fs::File::open(part)?, &mut hasher)?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != file.sha256 {
        let _ = std::fs::remove_file(part);
        return Err(DownloadError::Corrupt(file.name.to_owned()));
    }
    std::fs::rename(part, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_with(content: &[u8]) -> ModelFile {
        let sha: &'static str =
            Box::leak(format!("{:x}", Sha256::digest(content)).into_boxed_str());
        ModelFile {
            name: "model.onnx",
            url: "https://example.invalid/model.onnx",
            sha256: sha,
            bytes: content.len() as u64,
        }
    }

    #[test]
    fn a_good_download_gets_its_final_name() {
        let dir = tempfile::tempdir().unwrap();
        let (part, target) = (
            dir.path().join("model.onnx.part"),
            dir.path().join("model.onnx"),
        );
        std::fs::write(&part, b"modelo").unwrap();
        verify_and_finish(&part, &target, &file_with(b"modelo")).unwrap();
        assert!(target.is_file() && !part.exists());
    }

    #[test]
    fn a_corrupt_download_is_deleted_and_never_used() {
        let dir = tempfile::tempdir().unwrap();
        let (part, target) = (
            dir.path().join("model.onnx.part"),
            dir.path().join("model.onnx"),
        );
        std::fs::write(&part, b"otra cosa").unwrap();
        let e = verify_and_finish(&part, &target, &file_with(b"modelo")).unwrap_err();
        assert!(matches!(e, DownloadError::Corrupt(_)));
        assert!(!target.exists() && !part.exists());
    }

    #[test]
    fn a_model_is_ready_only_with_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let card = sift_embeddings::MODELS[0];
        assert!(!is_ready(dir.path(), card));
        std::fs::write(dir.path().join(card.model.name), b"x").unwrap();
        assert!(!is_ready(dir.path(), card));
        std::fs::write(dir.path().join(card.tokenizer.name), b"x").unwrap();
        assert!(is_ready(dir.path(), card));
    }
}
