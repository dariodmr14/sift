//! El hilo del indexado: el trabajador, su bucle de trabajos y los ayudantes generales
//! (estado, descanso, cancelación y vaciado de la cola).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sift_embeddings::OnnxEmbedder;
use sift_extract::Extractor;
use sift_indexer::{IgnoreRules, Pace, Throttle};
use sift_search::TextIndex;
use sift_storage::Root;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use super::status::{IndexPhase, IndexProgress, IndexStatus, PANIC_NOTE};
use super::{Job, PAUSE_CHECK};
use crate::load::SystemLoad;
use crate::smart::SmartSearch;
use crate::spaces_commands::Db;
use crate::watcher::FolderWatcher;

/// Cada cuánto se avisa a la UI del progreso, como mucho.
const EMIT_EVERY: Duration = Duration::from_millis(120);

pub(super) struct Worker {
    pub(super) app: AppHandle,
    pub(super) index: Arc<TextIndex>,
    pub(super) status: Arc<Mutex<IndexStatus>>,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) watcher: Option<FolderWatcher>,
    pub(super) throttle: Throttle<SystemLoad>,
    pub(super) last_emit: Instant,
    pub(super) smart: Arc<SmartSearch>,
    /// El embedder del indexado y el índice de vectores que va llenando, del modelo
    /// actual (`MODELS[0]`). Las consultas van aparte, en `smart`: mientras se cambia de
    /// modelo, siguen usando el anterior.
    pub(super) embedder: Option<OnnxEmbedder>,
    pub(super) target_vectors: Option<Arc<sift_search::VectorIndex>>,
    /// Vectores añadidos al índice HNSW desde la última vez que se guardó.
    pub(super) unsaved_vectors: usize,
    /// La cola del propio indexado: la descarga del modelo avisa por aquí al acabar.
    pub(super) jobs: mpsc::Sender<Job>,
    /// La descarga del modelo, que va en su propio hilo (ver `download.rs`).
    pub(super) download: super::download::ModelDownload,
}

/// Lo que una pasada necesita y no cambia mientras dura.
pub(super) struct Pass {
    pub(super) roots: Vec<Root>,
    pub(super) rules: IgnoreRules,
}

impl Worker {
    pub(super) fn run(mut self, rx: Receiver<Job>, pdfium_dir: Option<PathBuf>) {
        crate::load::enter_background_mode();
        let extractor = match pdfium_dir.as_deref().map(Extractor::with_pdfium) {
            Some((extractor, None)) => extractor,
            Some((extractor, Some(reason))) => {
                tracing::warn!(%reason, "sin PDFium: los PDF se indexan solo por nombre");
                extractor
            }
            None => Extractor::new(),
        };
        let (watching, reads_pdf) = (self.watcher.is_some(), extractor.reads_pdf());
        self.update(|s| {
            s.watching = watching;
            s.reads_pdf = reads_pdf;
        });

        if self.index.needs_rebuild {
            self.rebuild_from_db();
        } else {
            self.recover();
        }

        let mut sync_pending = false;
        // ¿Queda trabajo de búsqueda inteligente? Se hace solo con la cola vacía.
        let mut smart_pending = true;
        loop {
            // Lo reciente primero: si hay cambios en cola, antes que otra revisión; y
            // vectorizar, solo cuando no hay nada más.
            let job = if sync_pending {
                Job::Sync
            } else if smart_pending {
                match rx.try_recv() {
                    Ok(job) => job,
                    Err(mpsc::TryRecvError::Empty) => {
                        let outcome =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                self.smart_work(&extractor, &rx)
                            }));
                        match outcome {
                            Ok(outcome) => {
                                smart_pending = outcome.more;
                                sync_pending |= outcome.sync;
                            }
                            Err(_) => {
                                tracing::error!(
                                    "fallo inesperado al preparar la búsqueda inteligente"
                                );
                                smart_pending = false;
                                self.update(|s| s.error = Some(PANIC_NOTE.into()));
                            }
                        }
                        continue;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            } else {
                match rx.recv() {
                    Ok(job) => job,
                    Err(_) => return, // la app se está cerrando
                }
            };
            // Un fallo inesperado en un trabajo no para el indexado: se apunta, se avisa en
            // Ajustes y se sigue con el siguiente (auditoría, A2).
            let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match job {
                    Job::Paths(paths) => {
                        let (paths, sync_too) = drain(&rx, paths);
                        sync_pending |= sync_too;
                        if self.changed(&extractor, paths).is_err() {
                            sync_pending = true; // demasiados cambios o un fallo: revisión completa
                        }
                    }
                    Job::Sync => {
                        // Varias peticiones seguidas = una sola pasada, que además cubre los
                        // cambios sueltos que hubiera en cola.
                        let _ = drain(&rx, Vec::new());
                        self.cancel.store(false, Ordering::Relaxed);
                        sync_pending = self.sync_all(&extractor, &rx);
                    }
                    Job::Smart => {}
                }
            }));
            if handled.is_err() {
                tracing::error!("fallo inesperado en el indexado; se sigue con lo siguiente");
                // No se reintenta en bucle: la próxima revisión será con el siguiente cambio
                // o al volver a abrir Sift.
                sync_pending = false;
                self.update(|s| {
                    s.running = false;
                    s.phase = IndexPhase::Idle;
                    s.error = Some(PANIC_NOTE.into());
                });
            }
            // Tras cualquier cambio puede haber textos nuevos que vectorizar.
            smart_pending = true;
        }
    }

    pub(super) fn db(&self) -> tauri::State<'_, Db> {
        self.app.state::<Db>()
    }

    pub(super) fn update(&self, f: impl FnOnce(&mut IndexStatus)) {
        let snapshot = {
            let mut s = self.status.lock().unwrap_or_else(|p| p.into_inner());
            f(&mut s);
            s.clone()
        };
        let _ = IndexProgress(snapshot).emit(&self.app);
    }

    /// Como `update`, pero como mucho cada `EMIT_EVERY` (el progreso archivo a archivo).
    pub(super) fn update_throttled(&mut self, f: impl FnOnce(&mut IndexStatus)) {
        if self.last_emit.elapsed() >= EMIT_EVERY {
            self.last_emit = Instant::now();
            self.update(f);
        }
    }

    /// Descanso tras un trabajo y pausa si el PC está ocupado. Devuelve `true` si,
    /// durante la pausa, se ha pedido parar.
    pub(super) fn pace(&mut self, worked: Duration) -> bool {
        let mut worked = worked;
        loop {
            match self.throttle.after_work(worked) {
                Pace::Go { rest } => {
                    std::thread::sleep(rest);
                    return false;
                }
                Pace::Pause => {
                    worked = Duration::ZERO;
                    let until = Instant::now() + PAUSE_CHECK;
                    while Instant::now() < until {
                        if self.cancel.load(Ordering::Relaxed) {
                            return true;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }

    pub(super) fn cancelled(&self) -> bool {
        tracing::info!("indexado cancelado");
        self.update(|s| {
            s.running = false;
            s.phase = IndexPhase::Idle;
            s.current_root = None;
        });
        false
    }
}

/// Vacía la cola sin esperar: junta todas las rutas pendientes (con las que ya se traían)
/// y dice si entre medias había una petición de revisión completa (o de la búsqueda
/// inteligente, que se atiende igual: rehaciendo lo que haga falta).
pub(super) fn drain(rx: &Receiver<Job>, mut paths: Vec<PathBuf>) -> (Vec<PathBuf>, bool) {
    let mut sync = false;
    while let Ok(job) = rx.try_recv() {
        match job {
            Job::Sync => sync = true,
            Job::Paths(more) => paths.extend(more),
            Job::Smart => {}
        }
    }
    (paths, sync)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draining_merges_recent_changes_and_remembers_a_sync_request() {
        let (tx, rx) = mpsc::channel();
        tx.send(Job::Paths(vec!["b.md".into()])).unwrap();
        tx.send(Job::Sync).unwrap();
        tx.send(Job::Paths(vec!["c.md".into()])).unwrap();
        let (paths, sync) = drain(&rx, vec!["a.md".into()]);
        assert_eq!(paths, [PathBuf::from("a.md"), "b.md".into(), "c.md".into()]);
        assert!(sync);
        assert_eq!(drain(&rx, Vec::new()), (Vec::new(), false));
    }
}
