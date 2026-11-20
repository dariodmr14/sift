//! La descarga del modelo de búsqueda inteligente, en **su propio hilo** (auditoría 1.0).
//!
//! Antes se descargaba dentro del hilo del indexado: mientras bajaban los ~135 MB (minutos
//! con una conexión lenta, o para siempre si se colgaba), los archivos nuevos o cambiados no
//! se indexaban. Ahora el indexado sigue a lo suyo; al acabar la descarga, este hilo avisa a
//! la cola (`Job::Smart`) y el indexado carga el modelo.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tauri_specta::Event;

use super::Job;
use super::status::{IndexProgress, IndexStatus};
use crate::models::DownloadError;

/// Tras una descarga fallida (con sus reintentos), cuánto se espera antes de volver a
/// probar por su cuenta: así, sin conexión, cada archivo cambiado no lanza otra descarga.
const RETRY_LATER: Duration = Duration::from_secs(5 * 60);
/// Cada cuánto se avisa a la interfaz del progreso, como mucho.
const EMIT_EVERY: Duration = Duration::from_millis(250);

/// En qué punto está la descarga.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// El modelo está completo y verificado.
    Ready,
    /// Se está descargando (en otro hilo).
    Running,
    /// Falló: el motivo en palabras. Se volverá a intentar más tarde.
    Failed(String),
}

#[derive(Default)]
pub(super) struct ModelDownload {
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    /// Lo que dejó la última descarga al acabar (lo recoge el indexado).
    finished: Arc<Mutex<Option<Result<(), String>>>>,
    /// Tras un fallo: el motivo y desde cuándo se puede volver a intentar.
    failed: Option<(String, Instant)>,
}

/// Lo que necesita el hilo de la descarga.
pub(super) struct Context {
    pub app: AppHandle,
    pub status: Arc<Mutex<IndexStatus>>,
    pub jobs: Sender<Job>,
    pub dir: PathBuf,
    pub card: &'static sift_embeddings::ModelCard,
}

impl ModelDownload {
    /// Dónde está la descarga, y si hace falta, la empieza.
    pub(super) fn step(&mut self, ready: bool, start: impl FnOnce() -> Context) -> Step {
        if ready {
            self.failed = None;
            return Step::Ready;
        }
        if self.running.load(Ordering::SeqCst) {
            return Step::Running;
        }
        let finished = self
            .finished
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(Err(note)) = finished {
            self.failed = Some((note, Instant::now() + RETRY_LATER));
        }
        if let Some((note, until)) = &self.failed
            && Instant::now() < *until
        {
            return Step::Failed(note.clone());
        }
        self.spawn(start())
    }

    /// Para la descarga en curso (p. ej. al desactivar la búsqueda inteligente).
    pub(super) fn cancel(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.failed = None;
    }

    fn spawn(&mut self, ctx: Context) -> Step {
        self.cancel = Arc::new(AtomicBool::new(false));
        self.running.store(true, Ordering::SeqCst);
        let (running, cancel, finished) = (
            self.running.clone(),
            self.cancel.clone(),
            self.finished.clone(),
        );
        let spawned = std::thread::Builder::new()
            .name("sift-model-download".into())
            .spawn(move || {
                let started = Instant::now();
                // Un fallo inesperado aquí no puede dejar «descargando» para siempre.
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    download(&ctx, &cancel)
                }))
                .unwrap_or_else(|_| {
                    Err("Fallo inesperado al descargar: se volverá a intentar.".into())
                });
                match &outcome {
                    Ok(()) => tracing::info!(
                        model = ctx.card.id,
                        secs = started.elapsed().as_secs_f64(),
                        "modelo de búsqueda inteligente descargado"
                    ),
                    Err(note) => tracing::warn!(%note, "descarga del modelo no terminada"),
                }
                *finished.lock().unwrap_or_else(|p| p.into_inner()) = Some(outcome);
                running.store(false, Ordering::SeqCst);
                // Que el indexado lo recoja (cargar el modelo o decir qué pasó).
                let _ = ctx.jobs.send(Job::Smart);
            });
        match spawned {
            Ok(_) => Step::Running,
            Err(e) => {
                self.running.store(false, Ordering::SeqCst);
                tracing::error!(error = %e, "no se pudo arrancar la descarga");
                let note = "No se pudo empezar la descarga: se volverá a intentar.".to_owned();
                self.failed = Some((note.clone(), Instant::now() + RETRY_LATER));
                Step::Failed(note)
            }
        }
    }
}

/// Descarga con reintentos, avisando del progreso. `Err` con el motivo en palabras.
fn download(ctx: &Context, cancel: &AtomicBool) -> Result<(), String> {
    let mut last_emit = Instant::now();
    let result = crate::models::with_retries(&crate::models::RETRY_WAITS, cancel, || {
        crate::models::download(&ctx.dir, ctx.card, cancel, |done, _| {
            if last_emit.elapsed() >= EMIT_EVERY {
                last_emit = Instant::now();
                let snapshot = {
                    let mut s = ctx.status.lock().unwrap_or_else(|p| p.into_inner());
                    s.smart.downloaded_mb = (done / 1_000_000) as u32;
                    s.clone()
                };
                let _ = IndexProgress(snapshot).emit(&ctx.app);
            }
        })
    });
    result.map_err(|e| {
        // El detalle técnico, al log; a la persona, qué pasa y qué hará Sift.
        tracing::warn!(error = %e, "no se pudo descargar la búsqueda inteligente");
        note_for(&e)
    })
}

fn note_for(e: &DownloadError) -> String {
    match e {
        DownloadError::Network(_) => "Sin conexión: se descargará cuando la haya.",
        DownloadError::Corrupt(_) => "La descarga llegó dañada: se volverá a intentar.",
        DownloadError::Io(_) => "No se pudo guardar la descarga. ¿Queda espacio en el disco?",
        DownloadError::Cancelled => "Descarga cancelada.",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never() -> Context {
        unreachable!("no tendría que empezar una descarga")
    }

    #[test]
    fn a_ready_model_needs_no_download_and_clears_old_failures() {
        let mut d = ModelDownload {
            failed: Some(("x".into(), Instant::now() + RETRY_LATER)),
            ..ModelDownload::default()
        };
        assert_eq!(d.step(true, never), Step::Ready);
        assert!(d.failed.is_none());
    }

    #[test]
    fn while_running_it_does_not_start_another() {
        let mut d = ModelDownload::default();
        d.running.store(true, Ordering::SeqCst);
        assert_eq!(d.step(false, never), Step::Running);
    }

    #[test]
    fn after_a_failure_it_waits_before_trying_again() {
        let mut d = ModelDownload::default();
        *d.finished.lock().unwrap() = Some(Err("Sin conexión".into()));
        assert_eq!(d.step(false, never), Step::Failed("Sin conexión".into()));
        // Otra vez enseguida: sigue esperando, sin lanzar otra descarga.
        assert_eq!(d.step(false, never), Step::Failed("Sin conexión".into()));
        // Al desactivar y volver a activar se olvida la espera.
        d.cancel();
        assert!(d.failed.is_none());
    }

    #[test]
    fn notes_say_what_happens_in_plain_words() {
        assert!(note_for(&DownloadError::Network("x".into())).contains("Sin conexión"));
        assert!(note_for(&DownloadError::Io(std::io::Error::other("x"))).contains("disco"));
    }
}
