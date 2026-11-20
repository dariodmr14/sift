//! Servicio de indexado: un hilo propio que mantiene SQLite y el índice de búsqueda al
//! día, sin bloquear la UI ni las búsquedas, y **sin que se note**.
//!
//! Es una cola de trabajos con dos prioridades:
//!
//! - **Cambios recientes** (`Job::Paths`): lo que el vigilante de carpetas acaba de ver.
//!   Van primero, e incluso se cuelan entre dos archivos de una revisión completa: lo
//!   que acabas de guardar aparece en segundos aunque haya miles de archivos pendientes.
//! - **Revisión completa** (`Job::Sync`): al arrancar y al cambiar las carpetas. En dos
//!   fases, *buscar cambios* y *leer*, de lo más reciente a lo más antiguo.
//!
//! Para no molestar: el hilo va en el modo «segundo plano» de Windows (menos prioridad de
//! CPU y de disco), descansa tras cada archivo (límite de CPU) y se pausa si el PC está
//! ocupado (`sift_indexer::throttle`). Los cambios recientes no se frenan: son pocos.
//!
//! Al arrancar: si el índice de Tantivy es nuevo (o de otro formato), se reconstruye
//! desde SQLite; si la app se cerró a mitad, se recupera lo que el índice no confirmó.
//!
//! Con la cola vacía, el último trabajo: **vectorizar** para la búsqueda inteligente
//! (Fase 4, ADR 0009). Descarga el modelo la primera vez y calcula los vectores de lo que
//! falte, en lotes pequeños, con el mismo descanso y la misma pausa, y cediendo el paso a
//! cualquier cambio reciente o revisión que llegue. La búsqueda por significado funciona
//! desde el primer vector y mejora según avanza.
//!
//! El progreso se publica como evento para la UI, como mucho cada 120 ms.

mod download;
mod status;
mod sync;
mod vectorize;
mod worker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sift_indexer::{Throttle, ThrottleConfig};
use sift_search::TextIndex;
use tauri::AppHandle;

use crate::load::SystemLoad;
use crate::smart::SmartSearch;
use crate::watcher::FolderWatcher;

use status::STOPPED_NOTE;
pub use status::{IndexProgress, IndexStatus};
use worker::Worker;

/// Cada cuánto se vuelve a mirar si el PC sigue ocupado, estando en pausa.
const PAUSE_CHECK: Duration = Duration::from_secs(2);

enum Job {
    /// Revisión completa de todas las carpetas.
    Sync,
    /// Rutas que acaban de cambiar (del vigilante).
    Paths(Vec<PathBuf>),
    /// Se ha activado o desactivado la búsqueda inteligente.
    Smart,
}

/// Lo que guarda Tauri: el índice compartido y el canal hacia el hilo del indexado.
pub struct Indexing {
    pub index: Arc<TextIndex>,
    /// Búsqueda por significado: lo que la búsqueda necesita, cuando está lista.
    pub smart: Arc<SmartSearch>,
    jobs: Sender<Job>,
    status: Arc<Mutex<IndexStatus>>,
    /// Para parar la pasada en curso (p. ej. al quitar una carpeta).
    cancel: Arc<AtomicBool>,
}

impl Indexing {
    /// `pdfium_dir` es la carpeta donde debería estar `pdfium.dll` (ver ADR 0008).
    pub fn start(app: &AppHandle, index: Arc<TextIndex>, pdfium_dir: Option<PathBuf>) -> Indexing {
        let (jobs, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(IndexStatus::default()));
        let smart = Arc::new(SmartSearch::default());
        let cancel = Arc::new(AtomicBool::new(false));

        // El vigilante manda sus cambios a la misma cola.
        let to_queue = jobs.clone();
        let watcher = FolderWatcher::start(move |paths| {
            let _ = to_queue.send(Job::Paths(paths));
        })
        .map_err(|e| tracing::error!(error = %e, "sin vigilancia de carpetas en tiempo real"))
        .ok();

        let worker = Worker {
            app: app.clone(),
            index: index.clone(),
            status: status.clone(),
            cancel: cancel.clone(),
            watcher,
            throttle: Throttle::new(SystemLoad::default(), ThrottleConfig::default()),
            last_emit: Instant::now(),
            smart: smart.clone(),
            embedder: None,
            target_vectors: None,
            unsaved_vectors: 0,
            jobs: jobs.clone(),
            download: download::ModelDownload::default(),
            reader: None,
        };
        let crashed = status.clone();
        let spawned = std::thread::Builder::new()
            .name("sift-indexing".into())
            .spawn(move || {
                // Lo que no atrapa cada trabajo (p. ej. al arrancar): queda dicho en el estado.
                let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker.run(rx, pdfium_dir)
                }));
                if run.is_err() {
                    tracing::error!("el hilo de indexado se ha parado por un fallo inesperado");
                    let mut s = crashed.lock().unwrap_or_else(|p| p.into_inner());
                    s.running = false;
                    s.error = Some(STOPPED_NOTE.into());
                }
            });
        if let Err(e) = spawned {
            tracing::error!(error = %e, "no se pudo arrancar el hilo de indexado");
        }
        let service = Indexing {
            index,
            smart,
            jobs,
            status,
            cancel,
        };
        service.sync();
        service
    }

    /// Pide una revisión (incremental) de todas las carpetas.
    pub fn sync(&self) {
        self.send(Job::Sync);
    }

    /// Se ha cambiado la opción de búsqueda inteligente: se carga o se descarga.
    pub fn smart_changed(&self) {
        self.send(Job::Smart);
    }

    /// Le pasa trabajo al hilo de indexado. Si ese hilo ya no está, no se calla: lo dice
    /// en el estado, para que Ajustes no siga diciendo «Al día» (auditoría, A2).
    fn send(&self, job: Job) {
        if self.jobs.send(job).is_err() {
            tracing::error!("el hilo de indexado no está en marcha");
            let mut status = self.status.lock().unwrap_or_else(|p| p.into_inner());
            status.running = false;
            status.error = Some(STOPPED_NOTE.into());
        }
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
