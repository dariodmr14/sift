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

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sift_embeddings::OnnxEmbedder;
use sift_extract::Extractor;
use sift_indexer::{
    Context, IgnoreRules, IndexedFile, Pace, PlanRun, RootPlan, SyncError, SyncReport, Throttle,
    ThrottleConfig, scan_root, sync_paths,
};
use sift_search::TextIndex;
use sift_storage::Root;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::load::SystemLoad;
use crate::smart::SmartSearch;
use crate::spaces_commands::Db;
use crate::watcher::FolderWatcher;

/// En qué está el indexado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum IndexPhase {
    /// Sin trabajo: todo al día.
    #[default]
    Idle,
    /// Recorriendo las carpetas para ver qué ha cambiado («Buscando cambios…»).
    Scanning,
    /// Leyendo lo nuevo o cambiado («Leyendo `done` de `total` archivos»).
    Reading,
    /// Leyendo, pero en pausa porque el PC está ocupado.
    Paused,
    /// Poniendo al día unos pocos archivos que acaban de cambiar.
    Updating,
    /// Descargando el modelo de la búsqueda inteligente (la primera vez).
    Downloading,
    /// Calculando vectores («Preparando búsqueda inteligente · `done` de `total`»).
    Embedding,
}

/// Estado del indexado, para Ajustes → Carpetas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub running: bool,
    pub phase: IndexPhase,
    /// Carpeta que se está recorriendo o leyendo.
    pub current_root: Option<String>,
    /// Buscando cambios: archivos vistos hasta ahora.
    pub scanned: u32,
    /// Leyendo: archivos ya leídos de `total`.
    pub done: u32,
    /// Leyendo: archivos nuevos o cambiados que hay que leer en esta pasada.
    pub total: u32,
    /// Resumen de la última revisión completa.
    pub last: Option<LastRun>,
    pub error: Option<String>,
    /// ¿Se vigilan las carpetas en tiempo real? (si no, solo se revisan al arrancar)
    pub watching: bool,
    /// ¿Se lee el contenido de los PDF? (hace falta `pdfium.dll`)
    pub reads_pdf: bool,
    /// Búsqueda inteligente: activada, lista (modelo cargado) y, si algo va mal, por qué.
    pub smart: SmartStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartStatus {
    pub enabled: bool,
    /// Modelo cargado: la búsqueda ya usa el significado (aunque falten vectores).
    pub ready: bool,
    /// Vectores calculados y los que tendría que haber.
    pub done: u32,
    pub total: u32,
    /// Descarga del modelo, en MB.
    pub downloaded_mb: u32,
    pub download_mb: u32,
    /// Por qué no está lista (sin conexión, etc.), en palabras.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    /// Archivos en todas las carpetas al acabar.
    pub files: u32,
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub removed: u32,
    pub errors: u32,
    pub seconds: f64,
}

/// Lo que dice Ajustes si un trabajo del indexado falla de forma inesperada.
const PANIC_NOTE: &str = "Algo falló al revisar tus carpetas. Se volverá a intentar con el próximo cambio o al abrir Sift.";
/// Lo que dice si el hilo de indexado se ha parado del todo.
const STOPPED_NOTE: &str = "El indexado se ha parado por un fallo. Cierra Sift y vuelve a abrirlo.";

/// Evento: el estado del indexado ha cambiado.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct IndexProgress(pub IndexStatus);

/// A partir de cuántos archivos quitados en una pasada se compacta la base de datos.
const COMPACT_AFTER_REMOVING: u32 = 5_000;
/// Con más cambios de golpe que estos (descomprimir un zip enorme), sale más a cuenta
/// una revisión completa que ir ruta a ruta.
const MAX_PATHS_PER_BATCH: usize = 2_000;
/// Cada cuánto se vuelve a mirar si el PC sigue ocupado, estando en pausa.
const PAUSE_CHECK: Duration = Duration::from_secs(2);
/// Cada cuánto se avisa a la UI del progreso, como mucho.
const EMIT_EVERY: Duration = Duration::from_millis(120);

/// Fragmentos que se vectorizan de una vez: pocos, para ceder el paso enseguida.
const EMBED_BATCH: usize = 8;
/// Cada cuántos vectores nuevos se guarda el índice HNSW en disco.
const SAVE_VECTORS_EVERY: usize = 2_000;

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

struct Worker {
    app: AppHandle,
    index: Arc<TextIndex>,
    status: Arc<Mutex<IndexStatus>>,
    cancel: Arc<AtomicBool>,
    watcher: Option<FolderWatcher>,
    throttle: Throttle<SystemLoad>,
    last_emit: Instant,
    smart: Arc<SmartSearch>,
    /// El embedder del indexado y el índice de vectores que va llenando, del modelo
    /// actual (`MODELS[0]`). Las consultas van aparte, en `smart`: mientras se cambia de
    /// modelo, siguen usando el anterior.
    embedder: Option<OnnxEmbedder>,
    target_vectors: Option<Arc<sift_search::VectorIndex>>,
    /// Vectores añadidos al índice HNSW desde la última vez que se guardó.
    unsaved_vectors: usize,
}

/// Lo que una pasada necesita y no cambia mientras dura.
struct Pass {
    roots: Vec<Root>,
    rules: IgnoreRules,
}

impl Worker {
    fn run(mut self, rx: Receiver<Job>, pdfium_dir: Option<PathBuf>) {
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

    /// Como `update`, pero como mucho cada `EMIT_EVERY` (el progreso archivo a archivo).
    fn update_throttled(&mut self, f: impl FnOnce(&mut IndexStatus)) {
        if self.last_emit.elapsed() >= EMIT_EVERY {
            self.last_emit = Instant::now();
            self.update(f);
        }
    }

    /// Carpetas y reglas para una pasada. De paso, deja al vigilante mirando esas carpetas.
    fn prepare(&mut self) -> Result<Pass, String> {
        let (roots, patterns) = {
            let db = self.db();
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            (db.list_roots(), db.ignore_patterns())
        };
        let roots = roots.map_err(|e| e.to_string())?;
        let patterns = patterns.map_err(|e| e.to_string())?;
        let media = !crate::load_settings(&self.app).indexing.skip_media;
        let rules = IgnoreRules::new(&patterns)
            .map_err(|e| e.to_string())?
            .with_media(media);
        if let Some(watcher) = &mut self.watcher {
            let existing: Vec<PathBuf> = roots
                .iter()
                .filter(|r| r.path.is_dir())
                .map(|r| r.path.clone())
                .collect();
            watcher.watch(&existing);
        }
        Ok(Pass { roots, rules })
    }

    /// El índice es nuevo o de otro formato: se rellena desde SQLite (la fuente de verdad).
    fn rebuild_from_db(&self) {
        let started = Instant::now();
        let mut sink: &TextIndex = &self.index;
        let db = self.db();
        let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
        let result = db.for_each_file_with_chunks(|f, chunks| {
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
            (Ok(n), Ok(())) => {
                // Todo SQLite está ya en el índice: nada pendiente de recuperar.
                if let Err(e) = db.mark_search_synced() {
                    tracing::warn!(error = %e, "no se pudo apuntar el índice como al día");
                }
                tracing::info!(
                    files = n,
                    secs = started.elapsed().as_secs_f64(),
                    "índice reconstruido desde SQLite"
                );
            }
            (Err(e), _) => tracing::error!(error = %e, "no se pudo reconstruir el índice"),
            (_, Err(e)) => {
                tracing::error!(error = %e, "no se pudo confirmar el índice reconstruido")
            }
        }
    }

    /// La app se cerró a mitad de un indexado: se reenvía al índice lo que SQLite tiene
    /// guardado y el índice no llegó a confirmar. No lee el disco.
    fn recover(&self) {
        let mut sink: &TextIndex = &self.index;
        match sift_indexer::recover(&self.db().0, &mut sink) {
            Ok((0, 0)) => {}
            Ok((resent, removed)) => tracing::info!(
                resent,
                removed,
                "indexado anterior interrumpido: índice de búsqueda recuperado"
            ),
            Err(e) => tracing::error!(error = %e, "no se pudo recuperar el indexado interrumpido"),
        }
    }

    /// Pone al día las rutas que acaban de cambiar. `Err` = mejor una revisión completa.
    fn changed(&mut self, extractor: &Extractor, paths: Vec<PathBuf>) -> Result<(), ()> {
        if paths.len() > MAX_PATHS_PER_BATCH {
            tracing::info!(
                paths = paths.len(),
                "demasiados cambios de golpe: revisión completa"
            );
            return Err(());
        }
        let pass = self.prepare().map_err(|e| {
            self.update(|s| s.error = Some(e));
        })?;
        let never = AtomicBool::new(false);
        let app = self.app.clone();
        let db = app.state::<Db>();
        let ctx = Context {
            db: &db.0,
            rules: &pass.rules,
            extractor,
            cancel: &never,
        };
        let started = Instant::now();
        let index = self.index.clone();
        let mut sink: &TextIndex = &index;
        let was_idle = !self
            .status
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .running;
        if was_idle {
            self.update(|s| {
                s.running = true;
                s.phase = IndexPhase::Updating;
                s.total = paths.len() as u32;
                s.done = 0;
            });
        }
        let result = sync_paths(ctx, &pass.roots, &paths, &mut sink);
        if was_idle {
            self.update(|s| {
                s.running = false;
                s.phase = IndexPhase::Idle;
            });
        }
        match result {
            Ok(report) => {
                if report.changed() {
                    tracing::info!(
                        added = report.added,
                        updated = report.updated,
                        moved = report.moved,
                        removed = report.removed,
                        ms = started.elapsed().as_millis() as u64,
                        "cambios al día"
                    );
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "fallo al poner al día los cambios");
                Err(())
            }
        }
    }

    /// Revisión completa. Devuelve si hay que repetirla (llegó otra petición mientras).
    fn sync_all(&mut self, extractor: &Extractor, rx: &Receiver<Job>) -> bool {
        let started = Instant::now();
        let pass = match self.prepare() {
            Ok(pass) => pass,
            Err(e) => {
                self.update(|s| s.error = Some(e));
                return false;
            }
        };
        let cancel = self.cancel.clone();
        let app = self.app.clone();
        let db = app.state::<Db>();
        let ctx = Context {
            db: &db.0,
            rules: &pass.rules,
            extractor,
            cancel: &cancel,
        };
        self.update(|s| {
            *s = IndexStatus {
                running: true,
                phase: IndexPhase::Scanning,
                last: s.last.clone(),
                watching: s.watching,
                reads_pdf: s.reads_pdf,
                ..IndexStatus::default()
            };
        });

        // Fase 1: buscar cambios en todas las carpetas (barato: solo tamaño y fecha).
        let mut plans: Vec<RootPlan> = Vec::new();
        let mut scanned = 0;
        for root in &pass.roots {
            if !root.path.is_dir() {
                tracing::warn!(root = %root.path.display(), "la carpeta no existe (¿disco desconectado?); se salta");
                continue;
            }
            let shown = root.path.display().to_string();
            let result = scan_root(ctx, root, |n| {
                self.update_throttled(|s| {
                    s.scanned = (scanned + n) as u32;
                    s.current_root = Some(shown.clone());
                })
            });
            match result {
                Ok(plan) => {
                    scanned += plan.files();
                    plans.push(plan);
                }
                Err(SyncError::Cancelled) => return self.cancelled(),
                Err(e) => {
                    tracing::error!(root = %root.path.display(), error = %e, "fallo al buscar cambios");
                    self.update(|s| s.error = Some(e.to_string()));
                }
            }
        }

        // Fase 2: leer lo nuevo o cambiado, con el total ya conocido.
        let total: usize = plans.iter().map(RootPlan::to_read).sum();
        self.update(|s| {
            s.phase = IndexPhase::Reading;
            s.scanned = scanned as u32;
            s.total = total as u32;
            s.done = 0;
        });
        let mut summary = SyncReport::default();
        let mut offset = 0;
        let mut again = false;
        let index = self.index.clone();
        let mut sink: &TextIndex = &index;
        for plan in plans {
            let to_read = plan.to_read();
            let shown = plan.root().path.display().to_string();
            let mut run = PlanRun::new(plan);
            loop {
                let done = offset + run.done();
                self.update_throttled(|s| {
                    s.done = done as u32;
                    s.current_root = Some(shown.clone());
                });
                let worked = Instant::now();
                match run.step(ctx, &mut sink) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(SyncError::Cancelled) => return self.cancelled(),
                    Err(e) => {
                        tracing::error!(error = %e, "fallo al indexar");
                        self.update(|s| s.error = Some(e.to_string()));
                        break;
                    }
                }
                // Entre archivo y archivo: lo reciente se cuela, y se descansa o se pausa.
                again |= self.between_files(extractor, rx, worked.elapsed());
                if cancel.load(Ordering::Relaxed) {
                    return self.cancelled();
                }
            }
            offset += to_read;
            match run.finish(ctx, &mut sink) {
                Ok(r) => {
                    summary.added += r.added;
                    summary.updated += r.updated;
                    summary.unchanged += r.unchanged;
                    summary.removed += r.removed;
                    summary.errors += r.errors;
                }
                Err(SyncError::Cancelled) => return self.cancelled(),
                Err(e) => {
                    tracing::error!(error = %e, "fallo al terminar la carpeta");
                    self.update(|s| s.error = Some(e.to_string()));
                }
            }
        }

        // Tras una limpieza grande (p. ej. al dejar de indexar binarios), se devuelve el
        // espacio al disco: SQLite no encoge el fichero por sí solo.
        if summary.removed as u32 >= COMPACT_AFTER_REMOVING {
            let compacting = Instant::now();
            match db.0.lock().unwrap_or_else(|p| p.into_inner()).compact() {
                Ok(()) => tracing::info!(
                    removed = summary.removed,
                    secs = compacting.elapsed().as_secs_f64(),
                    "base de datos compactada"
                ),
                Err(e) => tracing::warn!(error = %e, "no se pudo compactar la base de datos"),
            }
        }
        let last = LastRun {
            files: scanned as u32,
            added: summary.added as u32,
            updated: summary.updated as u32,
            unchanged: summary.unchanged as u32,
            removed: summary.removed as u32,
            errors: summary.errors as u32,
            seconds: started.elapsed().as_secs_f64(),
        };
        tracing::info!(
            files = last.files,
            added = last.added,
            updated = last.updated,
            unchanged = last.unchanged,
            removed = last.removed,
            errors = last.errors,
            secs = last.seconds,
            "indexado terminado"
        );
        self.update(|s| {
            s.running = false;
            s.phase = IndexPhase::Idle;
            s.current_root = None;
            s.done = total as u32;
            s.last = Some(last);
        });
        again
    }

    /// Entre dos archivos de una revisión completa: atiende los cambios recientes que
    /// hayan llegado (prioridad), y descansa o se pausa según la carga del PC. Devuelve
    /// si alguien ha pedido otra revisión completa mientras tanto.
    fn between_files(
        &mut self,
        extractor: &Extractor,
        rx: &Receiver<Job>,
        worked: Duration,
    ) -> bool {
        let mut again = false;
        let mut worked = worked;
        loop {
            let (paths, sync_too) = drain(rx, Vec::new());
            again |= sync_too;
            if !paths.is_empty() && self.changed(extractor, paths).is_err() {
                again = true;
            }
            match self.throttle.after_work(worked) {
                Pace::Go { rest } => {
                    if self.status.lock().unwrap_or_else(|p| p.into_inner()).phase
                        == IndexPhase::Paused
                    {
                        self.update(|s| s.phase = IndexPhase::Reading);
                    }
                    std::thread::sleep(rest);
                    return again;
                }
                Pace::Pause => {
                    self.update(|s| s.phase = IndexPhase::Paused);
                    worked = Duration::ZERO;
                    // En pausa se sigue atento a cancelaciones y a cambios recientes.
                    let until = Instant::now() + PAUSE_CHECK;
                    while Instant::now() < until {
                        if self.cancel.load(Ordering::Relaxed) {
                            return again;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }

    /// Un rato de trabajo de búsqueda inteligente: prepararla si no lo está (descargar y
    /// cargar el modelo, cargar o reconstruir el índice de vectores) y vectorizar lo que
    /// falte. Cede el paso en cuanto llega otro trabajo.
    fn smart_work(&mut self, extractor: &Extractor, rx: &Receiver<Job>) -> SmartOutcome {
        let done = SmartOutcome {
            more: false,
            sync: false,
        };
        let card = sift_embeddings::MODELS[0];
        let enabled = !crate::load_settings(&self.app)
            .indexing
            .disable_smart_search;
        if !enabled {
            self.smart.set(None);
            self.embedder = None;
            self.target_vectors = None;
            self.update(|s| s.smart = SmartStatus::default());
            return done;
        }
        let missing = self.embedder.is_none() || self.target_vectors.is_none();
        if missing && let Err(note) = self.prepare_smart(card) {
            tracing::warn!(%note, "búsqueda inteligente no disponible por ahora");
            self.update(|s| {
                s.running = false;
                s.phase = IndexPhase::Idle;
                s.smart = SmartStatus {
                    enabled: true,
                    note: Some(note),
                    ..SmartStatus::default()
                };
            });
            // Se reintenta en el siguiente trabajo (p. ej. al volver la conexión y
            // revisar las carpetas al arrancar).
            return done;
        }
        self.embed_pending(card, extractor, rx)
    }

    /// Descarga el modelo si falta, lo carga (dos veces: indexado y consultas) y carga o
    /// reconstruye el índice de vectores. `Err` con el motivo en palabras.
    fn prepare_smart(&mut self, card: &'static sift_embeddings::ModelCard) -> Result<(), String> {
        let app_data = self.app.path().app_data_dir().map_err(|e| e.to_string())?;
        let dir = crate::models::model_dir(&app_data, card);
        if !crate::models::is_ready(&dir, card) {
            let total_mb = (card.download_bytes() / 1_000_000) as u32;
            self.update(|s| {
                s.running = true;
                s.phase = IndexPhase::Downloading;
                s.smart = SmartStatus {
                    enabled: true,
                    download_mb: total_mb,
                    ..SmartStatus::default()
                };
            });
            let started = Instant::now();
            let cancel = self.cancel.clone();
            let result = crate::models::download(&dir, card, &cancel, |done, _| {
                self.update_throttled(|s| s.smart.downloaded_mb = (done / 1_000_000) as u32);
            });
            if let Err(e) = result {
                // El detalle técnico, al log; a la persona, qué pasa y qué hará Sift.
                tracing::warn!(error = %e, "no se pudo descargar la búsqueda inteligente");
                return Err(match e {
                    crate::models::DownloadError::Network(_) => {
                        "Sin conexión: se descargará cuando la haya.".to_owned()
                    }
                    crate::models::DownloadError::Corrupt(_) => {
                        "La descarga llegó dañada: se volverá a intentar.".to_owned()
                    }
                    crate::models::DownloadError::Io(_) => {
                        "No se pudo guardar la descarga. ¿Queda espacio en el disco?".to_owned()
                    }
                    crate::models::DownloadError::Cancelled => "Descarga cancelada.".to_owned(),
                });
            }
            tracing::info!(
                model = card.id,
                secs = started.elapsed().as_secs_f64(),
                "modelo de búsqueda inteligente descargado"
            );
        }
        let load = |threads| {
            sift_embeddings::OnnxEmbedder::load(card, &dir, threads).map_err(|e| {
                tracing::warn!(error = %e, "no se pudo cargar el modelo");
                "No se pudo poner en marcha. Prueba a desactivarla y volver a activarla \
                 en Ajustes → General."
                    .to_owned()
            })
        };
        let background = load(crate::smart::BACKGROUND_THREADS)?;
        let vectors = Arc::new(self.open_vectors(&app_data, card)?);

        // ¿Con qué modelo se busca? Con el actual si ya lo cubre todo o si no hay otro;
        // si se está cambiando de modelo, con el anterior hasta que el nuevo acabe
        // (ADR 0009, punto 7): nunca se queda sin búsqueda por significado.
        let (done, total, others) = {
            let db = self.db();
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            let (done, total) = db
                .embedding_progress(card.id, &sift_storage::EmbedPolicy::standard())
                .map_err(|e| e.to_string())?;
            (
                done,
                total,
                db.embedding_models().map_err(|e| e.to_string())?,
            )
        };
        let previous = others
            .iter()
            .filter(|(id, _)| id != card.id)
            .filter_map(|(id, _)| sift_embeddings::model(id))
            .find(|old| crate::models::is_ready(&crate::models::model_dir(&app_data, old), old));
        let ready = match previous {
            Some(old) if done < total => {
                tracing::info!(
                    old = old.id,
                    new = card.id,
                    "cambio de modelo: se busca con el anterior mientras tanto"
                );
                let old_dir = crate::models::model_dir(&app_data, old);
                crate::smart::Ready {
                    card: old,
                    query: Arc::new(
                        sift_embeddings::OnnxEmbedder::load(
                            old,
                            &old_dir,
                            crate::smart::QUERY_THREADS,
                        )
                        .map_err(|e| e.to_string())?,
                    ),
                    vectors: Arc::new(self.open_vectors(&app_data, old)?),
                }
            }
            _ => crate::smart::Ready {
                card,
                query: Arc::new(load(crate::smart::QUERY_THREADS)?),
                vectors: vectors.clone(),
            },
        };
        self.embedder = Some(background);
        self.target_vectors = Some(vectors);
        self.smart.set(Some(ready));
        tracing::info!(model = card.id, "búsqueda inteligente lista");
        Ok(())
    }

    /// El índice de vectores de un modelo: el guardado si cuadra con SQLite; si no, se
    /// reconstruye desde SQLite (segundos: no se recalcula nada).
    fn open_vectors(
        &self,
        app_data: &std::path::Path,
        card: &sift_embeddings::ModelCard,
    ) -> Result<sift_search::VectorIndex, String> {
        let path = crate::smart::vectors_path(app_data, card);
        let db = self.db();
        let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
        let stored = db.count_embeddings(card.id).map_err(|e| e.to_string())?;
        if let Some(v) = sift_search::VectorIndex::load(&path, card.dims)
            && v.len() as u64 == stored
        {
            return Ok(v);
        }
        let started = Instant::now();
        let v = sift_search::VectorIndex::new(card.dims).map_err(|e| e.to_string())?;
        let mut batch = Vec::new();
        db.for_each_embedding(card.id, |h, vector| batch.push((h, vector)))
            .map_err(|e| e.to_string())?;
        v.add(&batch).map_err(|e| e.to_string())?;
        if let Err(e) = v.save(&path) {
            tracing::warn!(error = %e, "no se pudo guardar el índice de vectores");
        }
        tracing::info!(
            model = card.id,
            vectors = v.len(),
            secs = started.elapsed().as_secs_f64(),
            "índice de vectores reconstruido desde SQLite"
        );
        Ok(v)
    }

    /// Vectoriza lo que falte, en lotes pequeños, cediendo el paso entre lote y lote.
    fn embed_pending(
        &mut self,
        card: &'static sift_embeddings::ModelCard,
        extractor: &Extractor,
        rx: &Receiver<Job>,
    ) -> SmartOutcome {
        let policy = sift_storage::EmbedPolicy::standard();
        let Some(target) = self.target_vectors.clone() else {
            return SmartOutcome {
                more: false,
                sync: false,
            };
        };
        let progress = |worker: &Worker| {
            let db = worker.db();
            let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            db.embedding_progress(card.id, &policy).unwrap_or((0, 0))
        };
        let (mut done, total) = progress(self);
        let mut sync = false;
        let mut last_page = Vec::new();
        loop {
            if last_page.is_empty() {
                let page = {
                    let db = self.db();
                    let db = db.0.lock().unwrap_or_else(|p| p.into_inner());
                    db.pending_texts(card.id, &policy, 64)
                };
                match page {
                    Ok(mut page) if !page.is_empty() => {
                        // Longitudes parecidas en cada lote: menos relleno que calcular.
                        page.sort_by_key(|(_, t)| std::cmp::Reverse(t.len()));
                        last_page = page;
                    }
                    Ok(_) => break,
                    Err(e) => {
                        tracing::error!(error = %e, "no se pudo leer qué falta por vectorizar");
                        break;
                    }
                }
            }
            let batch: Vec<(i64, String)> =
                last_page.split_off(last_page.len().saturating_sub(EMBED_BATCH));
            let worked = Instant::now();
            let texts: Vec<&str> = batch.iter().map(|(_, t)| t.as_str()).collect();
            let Some(embedder) = &self.embedder else {
                break;
            };
            let vectors = match sift_embeddings::Embedder::embed(
                embedder,
                &texts,
                sift_embeddings::Purpose::Document,
            ) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "no se pudieron calcular vectores");
                    break;
                }
            };
            let rows: Vec<(i64, Vec<f32>)> = batch.iter().map(|(h, _)| *h).zip(vectors).collect();
            let saved = {
                let db = self.db();
                let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
                db.save_embeddings(card.id, &rows)
            };
            if let Err(e) = saved {
                tracing::error!(error = %e, "no se pudieron guardar vectores");
                break;
            }
            if let Err(e) = target.add(&rows) {
                tracing::error!(error = %e, "no se pudieron añadir vectores al índice");
            }
            done += rows.len() as u64;
            self.unsaved_vectors += rows.len();
            if self.unsaved_vectors >= SAVE_VECTORS_EVERY {
                self.save_vectors(card, &target);
            }
            self.update_throttled(|s| {
                s.running = true;
                s.phase = IndexPhase::Embedding;
                s.done = done.min(total) as u32;
                s.total = total as u32;
                s.smart.enabled = true;
                s.smart.ready = true;
                s.smart.done = done.min(total) as u32;
                s.smart.total = total as u32;
                s.smart.note = None;
            });
            // Cede el paso: cambios recientes al momento; una revisión, saliendo de aquí.
            let before = self.status.lock().unwrap_or_else(|p| p.into_inner()).phase;
            let (paths, sync_too) = drain(rx, Vec::new());
            if !paths.is_empty() && self.changed(extractor, paths).is_err() {
                sync = true;
            }
            if sync_too || sync {
                sync = true;
                break;
            }
            if self.status.lock().unwrap_or_else(|p| p.into_inner()).phase != before {
                self.update(|s| s.phase = IndexPhase::Embedding);
            }
            // Mismo descanso y misma pausa que el indexado.
            if self.pace(worked.elapsed()) {
                sync = true;
                break;
            }
        }
        // Al terminar (o al ceder): limpiar vectores huérfanos y guardar el índice.
        let finished = !sync;
        if finished {
            let pruned = {
                let db = self.db();
                let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
                db.prune_embeddings(card.id, &policy)
            };
            match pruned {
                Ok(gone) if !gone.is_empty() => {
                    let _ = target.remove(&gone);
                    self.unsaved_vectors += gone.len();
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "no se pudieron limpiar vectores"),
            }
        }
        if self.unsaved_vectors > 0 {
            self.save_vectors(card, &target);
        }
        let (done, total) = progress(self);
        // Cambio de modelo terminado: las consultas pasan al nuevo y se borra el anterior.
        if done >= total
            && let Some(current) = self.smart.ready()
            && current.card.id != card.id
        {
            self.switch_queries_to(card, target.clone());
        }
        self.update(|s| {
            if matches!(s.phase, IndexPhase::Embedding) {
                s.running = false;
                s.phase = IndexPhase::Idle;
            }
            s.smart = SmartStatus {
                enabled: true,
                ready: true,
                done: done as u32,
                total: total as u32,
                ..SmartStatus::default()
            };
        });
        SmartOutcome {
            more: sync && done < total,
            sync,
        }
    }

    /// El modelo nuevo ya lo cubre todo: las consultas pasan a usarlo y se borran los
    /// vectores (y el índice) del anterior.
    fn switch_queries_to(
        &mut self,
        card: &'static sift_embeddings::ModelCard,
        vectors: Arc<sift_search::VectorIndex>,
    ) {
        let Ok(app_data) = self.app.path().app_data_dir() else {
            return;
        };
        let dir = crate::models::model_dir(&app_data, card);
        let query =
            match sift_embeddings::OnnxEmbedder::load(card, &dir, crate::smart::QUERY_THREADS) {
                Ok(q) => q,
                Err(e) => {
                    tracing::error!(error = %e, "no se pudo cargar el modelo nuevo para buscar");
                    return;
                }
            };
        let old = self.smart.ready().map(|r| r.card);
        self.smart.set(Some(crate::smart::Ready {
            card,
            query: Arc::new(query),
            vectors,
        }));
        let dropped = {
            let db = self.db();
            let mut db = db.0.lock().unwrap_or_else(|p| p.into_inner());
            db.drop_other_models(card.id)
        };
        if let Some(old) = old {
            let _ = std::fs::remove_file(crate::smart::vectors_path(&app_data, old));
        }
        tracing::info!(new = card.id, dropped = ?dropped.ok(), "modelo cambiado");
    }

    fn save_vectors(
        &mut self,
        card: &sift_embeddings::ModelCard,
        vectors: &sift_search::VectorIndex,
    ) {
        let Ok(app_data) = self.app.path().app_data_dir() else {
            return;
        };
        match vectors.save(&crate::smart::vectors_path(&app_data, card)) {
            Ok(()) => self.unsaved_vectors = 0,
            Err(e) => tracing::warn!(error = %e, "no se pudo guardar el índice de vectores"),
        }
    }

    /// Descanso tras un trabajo y pausa si el PC está ocupado. Devuelve `true` si,
    /// durante la pausa, se ha pedido parar.
    fn pace(&mut self, worked: Duration) -> bool {
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

    fn cancelled(&self) -> bool {
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
fn drain(rx: &Receiver<Job>, mut paths: Vec<PathBuf>) -> (Vec<PathBuf>, bool) {
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

/// Qué ha pasado en un rato de trabajo de búsqueda inteligente.
struct SmartOutcome {
    /// Queda trabajo (se ha cedido el paso a otra cosa).
    more: bool,
    /// Alguien ha pedido una revisión completa mientras tanto.
    sync: bool,
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
