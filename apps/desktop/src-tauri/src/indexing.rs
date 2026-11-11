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
//! El progreso se publica como evento para la UI, como mucho cada 120 ms.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
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

enum Job {
    /// Revisión completa de todas las carpetas.
    Sync,
    /// Rutas que acaban de cambiar (del vigilante).
    Paths(Vec<PathBuf>),
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
    /// `pdfium_dir` es la carpeta donde debería estar `pdfium.dll` (ver ADR 0008).
    pub fn start(app: &AppHandle, index: Arc<TextIndex>, pdfium_dir: Option<PathBuf>) -> Indexing {
        let (jobs, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(IndexStatus::default()));
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
        };
        let spawned = std::thread::Builder::new()
            .name("sift-indexing".into())
            .spawn(move || worker.run(rx, pdfium_dir));
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

    /// Pide una revisión (incremental) de todas las carpetas.
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
    watcher: Option<FolderWatcher>,
    throttle: Throttle<SystemLoad>,
    last_emit: Instant,
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
        loop {
            // Lo reciente primero: si hay cambios en cola, antes que otra revisión.
            let job = if sync_pending {
                Job::Sync
            } else {
                match rx.recv() {
                    Ok(job) => job,
                    Err(_) => return, // la app se está cerrando
                }
            };
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
            }
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
/// y dice si entre medias había una petición de revisión completa.
fn drain(rx: &Receiver<Job>, mut paths: Vec<PathBuf>) -> (Vec<PathBuf>, bool) {
    let mut sync = false;
    while let Ok(job) = rx.try_recv() {
        match job {
            Job::Sync => sync = true,
            Job::Paths(more) => paths.extend(more),
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
