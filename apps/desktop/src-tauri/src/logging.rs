//! Logs con `tracing`: a la consola y a un fichero diario en la carpeta de logs de la app
//! (`%LOCALAPPDATA%\dev.sift.app\logs\sift.AAAA-MM-DD.log`). Se guardan 7 días.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Inicia los logs. El nivel se cambia con la variable `RUST_LOG` (p. ej. `RUST_LOG=debug`).
///
/// Devuelve un *guard*: el fichero se escribe en un hilo aparte (para no bloquear la UI) y
/// el guard vacía lo pendiente al destruirse, así que hay que mantenerlo vivo toda la app.
pub fn init(log_dir: &Path) -> anyhow::Result<WorkerGuard> {
    // La primera vez la carpeta no existe, y `max_log_files` necesita leerla para podar.
    std::fs::create_dir_all(log_dir)?;
    let file = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("sift")
        .filename_suffix("log")
        .max_log_files(7)
        .build(log_dir)?;
    let (file_writer, guard) = tracing_appender::non_blocking(file);

    // ONNX Runtime cuenta en INFO cada reserva de memoria: solo sus avisos.
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,ort=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .with(fmt::layer().with_ansi(false).with_writer(file_writer))
        .try_init()?;
    Ok(guard)
}
