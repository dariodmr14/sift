//! Servicio de indexado: un hilo propio que mantiene SQLite y el índice de búsqueda al
//! día, sin bloquear la UI ni las búsquedas.
//!
//! - Al arrancar: si el índice de Tantivy es nuevo (o de otro formato), se reconstruye
//!   desde SQLite; después se sincronizan las carpetas.
//! - Se le piden sincronizaciones por un canal. Si llega una mientras ya sincroniza, se
//!   apunta y se repite una vez al acabar (no se encolan diez).
//! - El progreso se publica como evento para la UI, como mucho cada 120 ms.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sift_indexer::{IgnoreRules, IndexedFile, SyncError, sync_root};
use sift_search::TextIndex;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::spaces_commands::Db;

/// Estado del indexado, para Ajustes → Carpetas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub running: bool,
    /// Carpeta que se está recorriendo.
    pub current_root: Option<String>,
    /// Archivos vistos en la pasada actual.
    pub processed: u32,
    /// Resumen de la última pasada completa.
    pub last: Option<LastRun>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub removed: u32,
    pub errors: u32,
    pub seconds: f64,
}

/// Evento: el estado del indexado ha cambiado.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct IndexProgress(pub IndexStatus);

enum Job {
    Sync,
}

/// Lo que guarda Tauri: el índice compartido y el canal hacia el hilo del indexado.
pub struct Indexing {
    pub index: Arc<TextIndex>,
    jobs: Sender<Job>,
    status: Arc<Mutex<IndexStatus>>,
    /// Para parar la pasada en curso (p. ej. al quitar una carpeta).
    cancel: Arc<AtomicBool>,
}

impl Indexing {
    pub fn start(app: &AppHandle, index: Arc<TextIndex>) -> Indexing {
        let (jobs, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(IndexStatus::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let worker = Worker {
            app: app.clone(),
            index: index.clone(),
            status: status.clone(),
            cancel: cancel.clone(),
        };
        let spawned = std::thread::Builder::new()
            .name("sift-indexing".into())
            .spawn(move || worker.run(rx));
        if let Err(e) = spawned {
            tracing::error!(error = %e, "no se pudo arrancar el hilo de indexado");
        }
        let service = Indexing {
            index,
            jobs,
            status,
            cancel,
        };
        service.sync();
        service
    }

    /// Pide una sincronización (incremental) de todas las carpetas.
    pub fn sync(&self) {
        let _ = self.jobs.send(Job::Sync);
    }

    /// Para la pasada en curso; la siguiente `sync` vuelve a empezar.
    pub fn cancel_current(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn status(&self) -> IndexStatus {
        self.status
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

struct Worker {
    app: AppHandle,
    index: Arc<TextIndex>,
    status: Arc<Mutex<IndexStatus>>,
    cancel: Arc<AtomicBool>,
}

impl Worker {
    fn run(self, rx: Receiver<Job>) {
        if self.index.needs_rebuild {
            self.rebuild_from_db();
        }
        while let Ok(Job::Sync) = rx.recv() {
            // Varias peticiones seguidas = una sola pasada.
            while rx.try_recv().is_ok() {}
            self.cancel.store(false, Ordering::Relaxed);
            self.sync_all();
        }
    }

    fn db(&self) -> tauri::State<'_, Db> {
        self.app.state::<Db>()
    }

    fn update(&self, f: impl FnOnce(&mut IndexStatus)) {
        let snapshot = {
            let mut s = self.status.lock().unwrap_or_else(|p| p.into_inner());
            f(&mut s);
            s.clone()
        };
        let _ = IndexProgress(snapshot).emit(&self.app);
    }

    /// El índice es nuevo o de otro formato: se rellena desde SQLite (la fuente de verdad).
    fn rebuild_from_db(&self) {
        let started = Instant::now();
        let mut sink: &TextIndex = &self.index;
        let result = self
            .db()
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .for_each_file_with_chunks(|f, chunks| {
                let file = IndexedFile {
                    file_id: f.id,
                    path: f.path.clone(),
                    name: f.name.clone(),
                    ext: f.ext.clone(),
                    chunks,
                };
                if let Err(e) = sift_indexer::IndexSink::upsert(&mut sink, &file) {
                    tracing::warn!(error = %e, path = %f.path, "no se pudo reindexar");
                }
            });
        let committed = self.index.commit_and_reload();
        match (result, committed) {
            (Ok(n), Ok(())) => tracing::info!(
                files = n,
                secs = started.elapsed().as_secs_f64(),
                "índice reconstruido desde SQLite"
            ),
            (Err(e), _) => tracing::error!(error = %e, "no se pudo reconstruir el índice"),
            (_, Err(e)) => {
                tracing::error!(error = %e, "no se pudo confirmar el índice reconstruido")
            }
        }
    }

    fn sync_all(&self) {
        let started = Instant::now();
        let (roots, patterns) = {
            let db = self.db();
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            (db.list_roots(), db.ignore_patterns())
        };
        let (roots, patterns) = match (roots, patterns) {
            (Ok(r), Ok(p)) => (r, p),
            (Err(e), _) | (_, Err(e)) => {
                self.update(|s| s.error = Some(e.to_string()));
                return;
            }
        };
        let rules = match IgnoreRules::new(&patterns) {
            Ok(r) => r,
            Err(e) => {
                self.update(|s| s.error = Some(e.to_string()));
                return;
            }
        };
        self.update(|s| {
            s.running = true;
            s.processed = 0;
            s.error = None;
        });
        let mut total = LastRun {
            added: 0,
            updated: 0,
            unchanged: 0,
            removed: 0,
            errors: 0,
            seconds: 0.0,
        };
        let mut offset = 0;
        let mut last_emit = Instant::now();
        for root in &roots {
            if !root.path.is_dir() {
                tracing::warn!(root = %root.path.display(), "la carpeta no existe (¿disco desconectado?); se salta");
                continue;
            }
            self.update(|s| s.current_root = Some(root.path.display().to_string()));
            let mut sink: &TextIndex = &self.index;
            let result = sync_root(
                &self.db().0,
                root,
                &rules,
                &mut sink,
                &self.cancel,
                |n, _| {
                    if last_emit.elapsed() >= Duration::from_millis(120) {
                        last_emit = Instant::now();
                        self.update(|s| s.processed = (offset + n) as u32);
                    }
                },
            );
            match result {
                Ok(r) => {
                    offset += r.added + r.updated + r.unchanged;
                    total.added += r.added as u32;
                    total.updated += r.updated as u32;
                    total.unchanged += r.unchanged as u32;
                    total.removed += r.removed as u32;
                    total.errors += r.errors as u32;
                }
                Err(SyncError::Cancelled) => {
                    tracing::info!("indexado cancelado");
                    self.update(|s| {
                        s.running = false;
                        s.current_root = None;
                    });
                    return;
                }
                Err(e) => {
                    tracing::error!(root = %root.path.display(), error = %e, "fallo al indexar");
                    self.update(|s| s.error = Some(e.to_string()));
                }
            }
        }
        total.seconds = started.elapsed().as_secs_f64();
        tracing::info!(?total.added, ?total.updated, ?total.unchanged, ?total.removed, ?total.errors, secs = total.seconds, "indexado terminado");
        self.update(|s| {
            s.running = false;
            s.current_root = None;
            s.processed = offset as u32;
            s.last = Some(total);
        });
    }
}
