//! La revisión de carpetas: preparar la pasada, recuperar el índice y poner al día los
//! archivos, tanto los que cambian al momento como la revisión completa.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use sift_extract::Extractor;
use sift_indexer::{
    Context, IgnoreRules, IndexedFile, Pace, PlanRun, RootPlan, SyncError, SyncReport, scan_root,
    sync_paths,
};
use sift_search::TextIndex;
use tauri::Manager;

use super::status::{IndexPhase, IndexStatus, LastRun};
use super::worker::{Pass, Worker, drain};
use super::{Job, PAUSE_CHECK};
use crate::spaces_commands::Db;

/// A partir de cuántos archivos quitados en una pasada se compacta la base de datos.
const COMPACT_AFTER_REMOVING: u32 = 5_000;
/// Con más cambios de golpe que estos (descomprimir un zip enorme), sale más a cuenta
/// una revisión completa que ir ruta a ruta.
const MAX_PATHS_PER_BATCH: usize = 2_000;

impl Worker {
    /// Carpetas y reglas para una pasada. De paso, deja al vigilante mirando esas carpetas.
    pub(super) fn prepare(&mut self) -> Result<Pass, String> {
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
    pub(super) fn rebuild_from_db(&self) {
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
    pub(super) fn recover(&self) {
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
    pub(super) fn changed(&mut self, extractor: &Extractor, paths: Vec<PathBuf>) -> Result<(), ()> {
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
    pub(super) fn sync_all(&mut self, extractor: &Extractor, rx: &Receiver<Job>) -> bool {
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
    pub(super) fn between_files(
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
}
