//! Sincronización incremental de las carpetas con la base de datos y el índice.
//!
//! Hay dos formas de ponerse al día:
//!
//! - **Revisión completa** de una carpeta, en dos fases, para poder decir «Leyendo 32 de
//!   120 archivos»:
//!   1. *Buscar cambios* ([`scan_root`]): recorre la carpeta mirando solo tamaño y fecha
//!      (barato) y apunta qué hay que leer y qué ya no existe. Nada se lee todavía.
//!   2. *Leer* ([`PlanRun`]): extrae, trocea y guarda lo apuntado, **archivo a archivo**
//!      (así quien lo llama puede parar entre dos archivos), de lo cambiado más
//!      recientemente a lo más antiguo: lo que acabas de tocar es lo que vas a buscar.
//! - **Solo unas rutas** ([`sync_paths`]): lo que el vigilante de carpetas dice que ha
//!   cambiado. Editar, crear, borrar, renombrar o mover se refleja sin recorrer nada más.
//!   Un archivo movido o renombrado se reconoce por su contenido y **conserva su id**
//!   (sus fragmentos y su historial), en vez de borrarse y volver a leerse.
//!
//! Solo entran los **formatos útiles** (`sift_extract`), y se saltan las carpetas generadas
//! por herramientas aunque no estén en los patrones ([`is_generated_dir`]).
//!
//! Por cada archivo:
//! - Mismo tamaño y fecha que lo guardado → no se toca (ni se lee).
//! - Cambió la fecha pero el contenido es el mismo (mismo hash) → solo se apunta la fecha.
//! - Nuevo o cambiado → se extrae, se trocea y se guarda (SQLite primero, luego el índice).
//! - Ya no está → se borra de los dos.
//!
//! **Si la app se cierra a mitad** no se pierde nada: SQLite se guarda archivo a archivo,
//! y lo que el índice de búsqueda no llegó a confirmar queda marcado y [`recover`] lo
//! reenvía al arrancar, sin volver a leer el disco. La siguiente revisión sigue donde iba,
//! porque lo ya guardado tiene el mismo tamaño y fecha.
//!
//! El indexador no sabe nada de Tantivy: escribe en un [`IndexSink`], que implementa
//! `sift-search` (y en los tests, uno falso).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use sift_extract::Extractor;
use sift_storage::{Database, FileState, NewChunk, NewFile, Root, StorageError};
use walkdir::WalkDir;

use crate::chunk::chunk_text;
use crate::ignore::IgnoreRules;

/// Tamaño máximo de un archivo de texto para leer su contenido, y del texto que se
/// guarda de un documento (más grandes: solo por nombre, o el texto recortado).
pub const MAX_CONTENT_BYTES: u64 = 5 * 1024 * 1024;
/// Cada cuántos archivos se confirma el índice (para que se vean resultados mientras).
const COMMIT_EVERY: usize = 200;

/// Lo que se manda al índice de búsqueda por cada archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedFile {
    pub file_id: i64,
    pub path: String,
    pub name: String,
    pub ext: String,
    /// (id del fragmento en SQLite, texto). Vacío si se indexa solo por nombre.
    pub chunks: Vec<(i64, String)>,
}

/// Destino del indexado: el índice de búsqueda (Tantivy en la app).
pub trait IndexSink {
    fn upsert(&mut self, file: &IndexedFile) -> Result<(), String>;
    fn remove(&mut self, file_ids: &[i64]) -> Result<(), String>;
    fn commit(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    /// Renombrados o movidos: conservan su id y no se vuelven a leer.
    pub moved: usize,
    /// Archivos que no se pudieron leer (se indexan por nombre y se apunta el motivo).
    pub errors: usize,
}

impl SyncReport {
    /// ¿Cambió algo en el índice?
    pub fn changed(&self) -> bool {
        self.added + self.updated + self.removed + self.moved > 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("índice de búsqueda: {0}")]
    Index(String),
    #[error("cancelado")]
    Cancelled,
}

/// Lo que necesita cualquier sincronización: dónde se guarda, qué se ignora, con qué se
/// lee y cómo se cancela.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub db: &'a Mutex<Database>,
    pub rules: &'a IgnoreRules,
    pub extractor: &'a Extractor,
    pub cancel: &'a AtomicBool,
}

impl Context<'_> {
    fn db(&self) -> std::sync::MutexGuard<'_, Database> {
        self.db.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// ¿Es una carpeta que genera una herramienta (y se puede regenerar)? Se reconoce por lo
/// que la rodea, no por el nombre: «Library» o «Temp» a secas podrían ser tuyas.
///
/// - **Unity**: `Library`, `Temp`, `Obj`, `Logs` y `UserSettings` junto a `Assets` y
///   `ProjectSettings`. En un proyecto real, `Library` tenía 35.000 archivos de caché.
/// - **Entorno virtual de Python**: cualquier carpeta con `pyvenv.cfg` (se llame como se
///   llame: `venv`, `.venv`, `env`…).
/// - **Cachés**: cualquier carpeta con `CACHEDIR.TAG` (el estándar que usan Cargo y otros).
pub fn is_generated_dir(dir: &Path) -> bool {
    if dir.join("pyvenv.cfg").is_file() || dir.join("CACHEDIR.TAG").is_file() {
        return true;
    }
    const UNITY: [&str; 5] = ["library", "temp", "obj", "logs", "usersettings"];
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase());
    match (name, dir.parent()) {
        (Some(name), Some(parent)) if UNITY.contains(&name.as_str()) => {
            parent.join("Assets").is_dir() && parent.join("ProjectSettings").is_dir()
        }
        _ => false,
    }
}

/// ¿Queda `path` fuera del índice por estar en una carpeta ignorada o generada?
/// (`path` está dentro de `root`.)
fn is_skipped(rules: &IgnoreRules, root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return true;
    };
    if rel.as_os_str().is_empty() {
        return false;
    }
    if rules.is_ignored(rel) {
        return true;
    }
    // Las carpetas que contienen a `path`, de dentro afuera, hasta la indexada.
    path.ancestors()
        .skip(1)
        .take_while(|dir| *dir != root && dir.starts_with(root))
        .any(is_generated_dir)
}

/// ¿Está ya al día lo guardado de este archivo? Lo está si no ha cambiado de tamaño ni
/// de fecha… salvo que se guardara **solo por nombre** y ahora se sepa leer su contenido
/// (p. ej. los PDF de antes de tener PDFium, o los Word de la Fase 2): entonces se lee una
/// vez. Lo que se intentó leer y falló (protegido, dañado) no se reintenta si no cambia.
fn up_to_date(
    previous: Option<&FileState>,
    size: i64,
    mtime: i64,
    path: &Path,
    extractor: &Extractor,
) -> bool {
    previous.is_some_and(|p| {
        let same = p.size == size && p.mtime == mtime;
        let name_only = p.content_hash.is_none() && !p.unreadable;
        same && !(name_only && extractor.reads_content(path))
    })
}

/// Un archivo nuevo o cambiado, pendiente de leer.
#[derive(Debug, Clone)]
struct Pending {
    path: PathBuf,
    size: i64,
    mtime: i64,
    previous: Option<FileState>,
}

fn size_and_mtime(meta: &std::fs::Metadata) -> (i64, i64) {
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64);
    (meta.len() as i64, mtime)
}

/// Recorre `start` (la carpeta indexada `root` o una subcarpeta suya) y llama a `visit`
/// con cada archivo indexable: ruta, tamaño y fecha. No entra en carpetas ignoradas ni
/// generadas.
fn walk(
    ctx: Context<'_>,
    root: &Path,
    start: &Path,
    mut visit: impl FnMut(PathBuf, i64, i64),
) -> Result<(), SyncError> {
    let walker = WalkDir::new(start)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Las carpetas ignoradas ni se recorren (node_modules puede tener 100.000 archivos).
            if e.path() == start {
                return true;
            }
            let rel = e.path().strip_prefix(root).unwrap_or(e.path());
            !ctx.rules.is_ignored(rel) && !(e.file_type().is_dir() && is_generated_dir(e.path()))
        });
    for entry in walker {
        if ctx.cancelled() {
            return Err(SyncError::Cancelled);
        }
        // Una carpeta sin permiso no para el indexado del resto.
        let Ok(entry) = entry else { continue };
        // Solo formatos útiles para buscar: ni binarios ni compilados.
        if !entry.file_type().is_file() || !ctx.rules.indexes(entry.path()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let (size, mtime) = size_and_mtime(&meta);
        visit(entry.path().to_path_buf(), size, mtime);
    }
    Ok(())
}

/// Resultado de buscar cambios en una carpeta: qué hay que leer y qué borrar.
#[derive(Debug)]
pub struct RootPlan {
    root: Root,
    /// De lo más reciente a lo más antiguo.
    pending: Vec<Pending>,
    gone: Vec<i64>,
    unchanged: usize,
}

impl RootPlan {
    pub fn root(&self) -> &Root {
        &self.root
    }

    /// Archivos que hay que leer (nuevos o cambiados).
    pub fn to_read(&self) -> usize {
        self.pending.len()
    }

    /// Archivos de la carpeta, sin contar los que ya no existen.
    pub fn files(&self) -> usize {
        self.pending.len() + self.unchanged
    }
}

/// Fase 1: recorre la carpeta mirando solo tamaño y fecha. `progress` recibe los archivos
/// vistos hasta ahora.
pub fn scan_root(
    ctx: Context<'_>,
    root: &Root,
    mut progress: impl FnMut(usize),
) -> Result<RootPlan, SyncError> {
    let mut known = ctx.db().file_states(root.id)?;
    let mut pending = Vec::new();
    let (mut seen, mut unchanged) = (0, 0);

    walk(ctx, &root.path, &root.path, |path, size, mtime| {
        seen += 1;
        progress(seen);
        let previous = known.remove(&path);
        if up_to_date(previous.as_ref(), size, mtime, &path, ctx.extractor) {
            unchanged += 1;
        } else {
            pending.push(Pending {
                path,
                size,
                mtime,
                previous,
            });
        }
    })?;

    // Lo cambiado hace menos va primero: es lo que más probablemente vas a buscar.
    pending.sort_by_key(|p| std::cmp::Reverse(p.mtime));
    // Lo que queda en `known` estaba guardado y ya no existe (o ahora se ignora).
    let gone = known.into_values().map(|s| s.id).collect();
    Ok(RootPlan {
        root: root.clone(),
        pending,
        gone,
        unchanged,
    })
}

/// Confirma el índice de búsqueda y apunta en SQLite que está al día. Es lo que hace
/// posible la recuperación: lo que no pase por aquí se reenvía al arrancar.
fn commit(ctx: Context<'_>, sink: &mut dyn IndexSink) -> Result<(), SyncError> {
    sink.commit().map_err(SyncError::Index)?;
    ctx.db().mark_search_synced()?;
    Ok(())
}

/// Lee un archivo y lo guarda (SQLite primero, que es la fuente de verdad; luego el
/// índice). `moved` son los archivos que acaban de desaparecer, por hash de contenido: si
/// este es uno de ellos con otro nombre, se mueve en vez de leerse como nuevo.
fn index_file(
    ctx: Context<'_>,
    root_id: i64,
    file: Pending,
    moved: &mut HashMap<String, FileState>,
    sink: &mut dyn IndexSink,
    report: &mut SyncReport,
) -> Result<(), SyncError> {
    let Pending {
        path,
        size,
        mtime,
        previous,
    } = file;
    let ext = sift_extract::extension(&path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (text, hash, error) = match ctx.extractor.extract(&path, MAX_CONTENT_BYTES) {
        Ok(Some(e)) => (Some(e.text), Some(e.hash), None),
        Ok(None) => (None, None, None),
        Err(e) => (
            None,
            None,
            Some((e.to_string(), e.reason().code().to_owned())),
        ),
    };

    // Cambió la fecha pero no el contenido: no hace falta reindexar.
    if let (Some(p), Some(h)) = (&previous, &hash)
        && p.content_hash.as_ref() == Some(h)
    {
        ctx.db().touch_file(p.id, size, mtime)?;
        report.unchanged += 1;
        return Ok(());
    }

    let new_file = NewFile {
        root_id,
        path: path.clone(),
        name: name.clone(),
        ext: ext.clone(),
        size,
        mtime,
        content_hash: hash,
        error: error.as_ref().map(|(message, _)| message.clone()),
        error_kind: error.as_ref().map(|(_, kind)| kind.clone()),
    };

    // Un archivo "nuevo" con el contenido de uno que acaba de desaparecer es ese mismo,
    // renombrado o movido: conserva su id y sus fragmentos.
    if previous.is_none()
        && let Some(old) = new_file.content_hash.as_ref().and_then(|h| moved.remove(h))
    {
        let chunks = ctx.db().move_file(old.id, &new_file)?;
        sink.upsert(&IndexedFile {
            file_id: old.id,
            path: path.display().to_string(),
            name,
            ext,
            chunks,
        })
        .map_err(SyncError::Index)?;
        report.moved += 1;
        return Ok(());
    }

    let chunks: Vec<NewChunk> = text
        .as_deref()
        .map(chunk_text)
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, c)| NewChunk {
            ordinal: i as i64,
            start: c.start as i64,
            text: c.text,
        })
        .collect();
    let (file_id, chunk_ids) = ctx.db().upsert_file(&new_file, &chunks)?;
    sink.upsert(&IndexedFile {
        file_id,
        path: path.display().to_string(),
        name,
        ext,
        chunks: chunk_ids
            .into_iter()
            .zip(chunks.into_iter().map(|c| c.text))
            .collect(),
    })
    .map_err(SyncError::Index)?;

    if error.is_some() {
        report.errors += 1;
    }
    if previous.is_some() {
        report.updated += 1;
    } else {
        report.added += 1;
    }
    Ok(())
}

/// Fase 2, paso a paso: cada [`PlanRun::step`] lee un archivo. Entre dos pasos, quien lo
/// llama puede esperar (límite de CPU, pausa si el PC está ocupado) o atender algo más
/// urgente.
pub struct PlanRun {
    root: Root,
    pending: VecDeque<Pending>,
    gone: Vec<i64>,
    total: usize,
    report: SyncReport,
    since_commit: usize,
}

impl PlanRun {
    pub fn new(plan: RootPlan) -> PlanRun {
        PlanRun {
            total: plan.pending.len(),
            report: SyncReport {
                unchanged: plan.unchanged,
                ..SyncReport::default()
            },
            root: plan.root,
            pending: plan.pending.into(),
            gone: plan.gone,
            since_commit: 0,
        }
    }

    /// Archivos ya leídos de esta carpeta.
    pub fn done(&self) -> usize {
        self.total - self.pending.len()
    }

    /// El archivo que leerá el siguiente paso.
    pub fn next_path(&self) -> Option<&Path> {
        self.pending.front().map(|p| p.path.as_path())
    }

    /// Lee el siguiente archivo. Devuelve `false` cuando ya no queda ninguno.
    pub fn step(&mut self, ctx: Context<'_>, sink: &mut dyn IndexSink) -> Result<bool, SyncError> {
        if ctx.cancelled() {
            commit(ctx, sink)?;
            return Err(SyncError::Cancelled);
        }
        let Some(file) = self.pending.pop_front() else {
            return Ok(false);
        };
        let mut no_moves = HashMap::new();
        index_file(
            ctx,
            self.root.id,
            file,
            &mut no_moves,
            sink,
            &mut self.report,
        )?;
        self.since_commit += 1;
        if self.since_commit >= COMMIT_EVERY {
            commit(ctx, sink)?;
            self.since_commit = 0;
        }
        Ok(true)
    }

    /// Borra lo que ya no existe, confirma y apunta la carpeta como revisada.
    pub fn finish(
        self,
        ctx: Context<'_>,
        sink: &mut dyn IndexSink,
    ) -> Result<SyncReport, SyncError> {
        let mut report = self.report;
        if !self.gone.is_empty() {
            ctx.db().delete_files(&self.gone)?;
            sink.remove(&self.gone).map_err(SyncError::Index)?;
            report.removed = self.gone.len();
        }
        commit(ctx, sink)?;
        ctx.db().mark_root_indexed(self.root.id)?;
        Ok(report)
    }
}

/// Fase 2 de un tirón. `progress` recibe (leídos, ruta) antes de cada archivo.
pub fn apply_plan(
    ctx: Context<'_>,
    plan: RootPlan,
    sink: &mut dyn IndexSink,
    mut progress: impl FnMut(usize, &Path),
) -> Result<SyncReport, SyncError> {
    let mut run = PlanRun::new(plan);
    loop {
        if let Some(path) = run.next_path() {
            progress(run.done(), path);
        }
        if !run.step(ctx, sink)? {
            return run.finish(ctx, sink);
        }
    }
}

/// Las dos fases seguidas, sin informar del progreso.
pub fn sync_root(
    ctx: Context<'_>,
    root: &Root,
    sink: &mut dyn IndexSink,
) -> Result<SyncReport, SyncError> {
    let plan = scan_root(ctx, root, |_| {})?;
    apply_plan(ctx, plan, sink, |_, _| {})
}

/// Pone al día solo unas rutas (las que el vigilante de carpetas dice que han cambiado),
/// sin recorrer el resto. Cada ruta puede ser un archivo o una carpeta, existir o no:
///
/// - Archivo que existe → se lee si es nuevo o ha cambiado.
/// - Carpeta que existe → se revisa entera (alguien la ha movido o renombrado aquí).
/// - Ya no existe (o ahora se ignora) → se borra eso y todo lo que colgaba de ahí.
///
/// Un renombrado o un movimiento llegan como «desaparece la ruta vieja, aparece la
/// nueva»: se casan por contenido y el archivo conserva su id.
pub fn sync_paths(
    ctx: Context<'_>,
    roots: &[Root],
    paths: &[PathBuf],
    sink: &mut dyn IndexSink,
) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport::default();
    let mut pending: Vec<(i64, Pending)> = Vec::new();
    let mut gone: HashMap<PathBuf, FileState> = HashMap::new();
    let mut seen: HashSet<&PathBuf> = HashSet::new();

    for path in paths {
        if !seen.insert(path) {
            continue;
        }
        // La carpeta indexada que la contiene (no se solapan: como mucho hay una).
        let Some(root) = roots.iter().find(|r| path.starts_with(&r.path)) else {
            continue;
        };
        let skipped = is_skipped(ctx.rules, &root.path, path);
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.is_file() && !skipped && ctx.rules.indexes(path) => {
                let (size, mtime) = size_and_mtime(&meta);
                let previous = ctx.db().file_state(path)?;
                if up_to_date(previous.as_ref(), size, mtime, path, ctx.extractor) {
                    report.unchanged += 1;
                } else {
                    pending.push((
                        root.id,
                        Pending {
                            path: path.clone(),
                            size,
                            mtime,
                            previous,
                        },
                    ));
                }
            }
            Ok(meta) if meta.is_dir() && !skipped => {
                let mut known = ctx.db().files_under(path)?;
                walk(ctx, &root.path, path, |file, size, mtime| {
                    let previous = known.remove(&file);
                    if up_to_date(previous.as_ref(), size, mtime, &file, ctx.extractor) {
                        report.unchanged += 1;
                    } else {
                        pending.push((
                            root.id,
                            Pending {
                                path: file,
                                size,
                                mtime,
                                previous,
                            },
                        ));
                    }
                })?;
                gone.extend(known);
            }
            // No existe, se ignora o ya no es un formato útil: fuera, con lo que colgara.
            _ => gone.extend(ctx.db().files_under(path)?),
        }
    }

    // Una ruta puede venir dos veces por caminos distintos (ella y su carpeta).
    let mut queued = HashSet::new();
    pending.retain(|(_, p)| queued.insert(p.path.clone()));
    gone.retain(|path, _| !queued.contains(path));

    // Lo que desaparece, por contenido: si reaparece con otro nombre, es un movimiento.
    let mut moved: HashMap<String, FileState> = gone
        .values()
        .filter_map(|state| Some((state.content_hash.clone()?, state.clone())))
        .collect();
    let candidates: HashSet<i64> = moved.values().map(|s| s.id).collect();

    pending.sort_by_key(|(_, p)| std::cmp::Reverse(p.mtime));
    for (root_id, file) in pending {
        if ctx.cancelled() {
            commit(ctx, sink)?;
            return Err(SyncError::Cancelled);
        }
        index_file(ctx, root_id, file, &mut moved, sink, &mut report)?;
    }

    // Lo que no ha reaparecido en otro sitio se borra de verdad.
    let still_there: HashSet<i64> = moved.values().map(|s| s.id).collect();
    let removed: Vec<i64> = gone
        .values()
        .map(|s| s.id)
        .filter(|id| !candidates.contains(id) || still_there.contains(id))
        .collect();
    if !removed.is_empty() {
        ctx.db().delete_files(&removed)?;
        sink.remove(&removed).map_err(SyncError::Index)?;
        report.removed = removed.len();
    }
    commit(ctx, sink)?;
    Ok(report)
}

/// Al arrancar: lleva al índice de búsqueda lo que SQLite tiene guardado y el índice no
/// llegó a confirmar (la app se cerró a mitad). No lee el disco: los fragmentos ya están
/// en SQLite. Devuelve (archivos reenviados, archivos quitados).
pub fn recover(
    db: &Mutex<Database>,
    sink: &mut dyn IndexSink,
) -> Result<(usize, usize), SyncError> {
    let lock = || db.lock().unwrap_or_else(|p| p.into_inner());
    let removals = lock().pending_removals()?;
    let unsynced = lock().unsynced_files()?;
    if removals.is_empty() && unsynced.is_empty() {
        return Ok((0, 0));
    }
    if !removals.is_empty() {
        sink.remove(&removals).map_err(SyncError::Index)?;
    }
    for (file, chunks) in &unsynced {
        sink.upsert(&IndexedFile {
            file_id: file.id,
            path: file.path.clone(),
            name: file.name.clone(),
            ext: file.ext.clone(),
            chunks: chunks.clone(),
        })
        .map_err(SyncError::Index)?;
    }
    sink.commit().map_err(SyncError::Index)?;
    lock().mark_search_synced()?;
    Ok((unsynced.len(), removals.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Índice falso: apunta qué archivos tiene.
    #[derive(Default)]
    struct FakeIndex {
        files: HashMap<i64, IndexedFile>,
        upserts: usize,
        commits: usize,
    }

    impl IndexSink for FakeIndex {
        fn upsert(&mut self, file: &IndexedFile) -> Result<(), String> {
            self.upserts += 1;
            self.files.insert(file.file_id, file.clone());
            Ok(())
        }
        fn remove(&mut self, ids: &[i64]) -> Result<(), String> {
            for id in ids {
                self.files.remove(id);
            }
            Ok(())
        }
        fn commit(&mut self) -> Result<(), String> {
            self.commits += 1;
            Ok(())
        }
    }

    struct Fixture {
        dir: tempfile::TempDir,
        db: Mutex<Database>,
        root: Root,
        rules: IgnoreRules,
        extractor: Extractor,
        cancel: AtomicBool,
    }

    impl Fixture {
        fn ctx(&self) -> Context<'_> {
            Context {
                db: &self.db,
                rules: &self.rules,
                extractor: &self.extractor,
                cancel: &self.cancel,
            }
        }
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(dir.path()).unwrap();
        let rules = IgnoreRules::new(&db.ignore_patterns().unwrap()).unwrap();
        Fixture {
            dir,
            db: Mutex::new(db),
            root,
            rules,
            extractor: Extractor::new(),
            cancel: AtomicBool::new(false),
        }
    }

    fn sync(f: &Fixture, index: &mut FakeIndex) -> SyncReport {
        sync_root(f.ctx(), &f.root, index).unwrap()
    }

    fn write(f: &Fixture, rel: &str, content: &str) -> PathBuf {
        // Con las barras de Windows, como las rutas que da el sistema.
        let p = rel
            .split('/')
            .fold(f.dir.path().to_path_buf(), |p, part| p.join(part));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, content).unwrap();
        p
    }

    /// Cambia la fecha de modificación (sin depender de que pase tiempo real).
    fn set_mtime(p: &Path, secs: u64) {
        let f = std::fs::File::options().write(true).open(p).unwrap();
        f.set_modified(UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn first_sync_indexes_content_and_names() {
        let f = fixture();
        write(
            &f,
            "BD/tema4.md",
            "# Normalización\nLa 3FN evita dependencias transitivas.",
        );
        write(&f, "BD/tema4.pdf", "%PDF");
        let mut index = FakeIndex::default();
        let r = sync(&f, &mut index);
        assert_eq!((r.added, r.errors), (2, 0));
        let md = index.files.values().find(|x| x.name == "tema4.md").unwrap();
        assert_eq!(md.chunks.len(), 1);
        assert!(md.chunks[0].1.contains("3FN"));
        let pdf = index
            .files
            .values()
            .find(|x| x.name == "tema4.pdf")
            .unwrap();
        assert!(pdf.chunks.is_empty()); // solo por nombre
    }

    #[test]
    fn second_sync_does_nothing_if_nothing_changed() {
        let f = fixture();
        write(&f, "a.md", "uno");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let r = sync(&f, &mut index);
        assert_eq!((r.added, r.updated, r.unchanged), (0, 0, 1));
        assert_eq!(index.upserts, 1);
    }

    #[test]
    fn same_content_with_new_date_is_not_reindexed() {
        let f = fixture();
        let p = write(&f, "a.md", "igual");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        set_mtime(&p, 1_000_000);
        let r = sync(&f, &mut index);
        assert_eq!((r.updated, r.unchanged), (0, 1));
        assert_eq!(index.upserts, 1);
    }

    #[test]
    fn changed_files_are_updated_and_deleted_ones_removed() {
        let f = fixture();
        let a = write(&f, "a.md", "versión uno");
        let b = write(&f, "b.md", "se va a borrar");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        std::fs::write(&a, "versión dos, más larga").unwrap();
        set_mtime(&a, 2_000_000);
        std::fs::remove_file(&b).unwrap();
        let r = sync(&f, &mut index);
        assert_eq!((r.updated, r.removed), (1, 1));
        assert_eq!(index.files.len(), 1);
        assert!(
            index.files.values().next().unwrap().chunks[0]
                .1
                .contains("dos")
        );
    }

    #[test]
    fn ignored_folders_are_not_walked() {
        let f = fixture();
        write(&f, "web/node_modules/react/index.js", "no");
        write(&f, "web/app.js", "sí");
        write(&f, "BD/~$Tema.docx", "temporal de Office");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let names: Vec<_> = index.files.values().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["app.js"]);
    }

    #[test]
    fn only_useful_formats_are_indexed() {
        let f = fixture();
        write(&f, "apuntes.md", "sí");
        write(&f, "Tema 1.pdf", "%PDF");
        for noise in ["lib.dll", "Main.class", "datos.bin", "sin_extension"] {
            write(&f, noise, "ruido");
        }
        let mut index = FakeIndex::default();
        let r = sync(&f, &mut index);
        assert_eq!(r.added, 2);
        let mut names: Vec<_> = index.files.values().map(|x| x.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["Tema 1.pdf", "apuntes.md"]);
    }

    #[test]
    fn images_and_videos_are_found_by_name_unless_turned_off() {
        let mut f = fixture();
        write(&f, "viaje/playa.jpg", "no es texto");
        write(&f, "clase.mp4", "tampoco");
        write(
            &f,
            "web/node_modules/pkg/logo.png",
            "en una carpeta ignorada",
        );
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let mut names: Vec<_> = index.files.values().map(|x| x.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["clase.mp4", "playa.jpg"]);
        assert!(index.files.values().all(|x| x.chunks.is_empty())); // solo el nombre

        // Con la opción desactivada, desaparecen del índice en la siguiente pasada.
        f.rules = f.rules.clone().with_media(false);
        let r = sync(&f, &mut index);
        assert_eq!(r.removed, 2);
        assert!(index.files.is_empty());
    }
    #[test]
    fn files_indexed_before_that_are_no_longer_useful_are_removed() {
        // Lo que indexó una versión anterior (imágenes, binarios) desaparece solo.
        let f = fixture();
        let png = write(&f, "datos.bin", "x");
        f.db.lock()
            .unwrap()
            .upsert_file(
                &NewFile {
                    root_id: f.root.id,
                    path: png,
                    name: "datos.bin".into(),
                    ext: "bin".into(),
                    size: 1,
                    mtime: 1,
                    content_hash: None,
                    error: None,
                    error_kind: None,
                },
                &[],
            )
            .unwrap();
        let mut index = FakeIndex::default();
        let r = sync(&f, &mut index);
        assert_eq!(r.removed, 1);
        assert_eq!(f.db.lock().unwrap().list_roots().unwrap()[0].files, 0);
    }

    #[test]
    fn generated_folders_are_recognised_by_their_surroundings() {
        let f = fixture();
        // Proyecto de Unity: `Library` es caché; `Assets` es lo tuyo.
        write(&f, "Juego/Assets/Player.cs", "class Player {}");
        write(
            &f,
            "Juego/ProjectSettings/ProjectVersion.txt",
            "m_EditorVersion",
        );
        write(
            &f,
            "Juego/Library/PackageCache/com.unity.x/Runtime/X.cs",
            "caché",
        );
        // Entorno virtual de Python con un nombre cualquiera.
        write(&f, "practica/entorno/pyvenv.cfg", "home = C:\\Python312");
        write(&f, "practica/entorno/Lib/os.py", "import sys");
        write(&f, "practica/main.py", "print('hola')");
        // Una carpeta «Library» normal (sin Unity alrededor) sí se indexa.
        write(&f, "Library/libro.md", "apuntes");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let mut names: Vec<_> = index.files.values().map(|x| x.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["Player.cs", "ProjectVersion.txt", "libro.md", "main.py"]
        );
    }

    #[test]
    fn unreadable_content_is_indexed_by_name_with_the_reason() {
        let f = fixture();
        std::fs::write(f.dir.path().join("raro.txt"), b"MZ\x90\x00binario").unwrap();
        let mut index = FakeIndex::default();
        let r = sync(&f, &mut index);
        assert_eq!(r.errors, 1);
        assert!(index.files.values().next().unwrap().chunks.is_empty());
        assert_eq!(f.db.lock().unwrap().list_roots().unwrap()[0].errors, 1);
    }

    #[test]
    fn cancelling_stops_and_commits_what_was_done() {
        let f = fixture();
        write(&f, "a.md", "uno");
        let mut index = FakeIndex::default();
        f.cancel.store(true, Ordering::Relaxed);
        let r = sync_root(f.ctx(), &f.root, &mut index);
        assert!(matches!(r, Err(SyncError::Cancelled)));
        assert!(index.files.is_empty());
    }

    #[test]
    fn scanning_reads_nothing_and_counts_what_to_read() {
        let f = fixture();
        write(&f, "a.md", "uno");
        write(&f, "b.md", "dos");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let c = write(&f, "c.md", "tres");
        set_mtime(&c, 3_000_000);
        let plan = scan_root(f.ctx(), &f.root, |_| {}).unwrap();
        assert_eq!((plan.to_read(), plan.files()), (1, 3));
        assert_eq!(index.upserts, 2); // buscar cambios no ha escrito nada
    }

    #[test]
    fn most_recently_changed_files_are_read_first() {
        let f = fixture();
        for (name, secs) in [
            ("viejo.md", 1_000),
            ("nuevo.md", 9_000),
            ("medio.md", 5_000),
        ] {
            let p = write(&f, name, name);
            set_mtime(&p, secs);
        }
        let plan = scan_root(f.ctx(), &f.root, |_| {}).unwrap();
        let mut order = Vec::new();
        let mut index = FakeIndex::default();
        apply_plan(f.ctx(), plan, &mut index, |done, p| {
            order.push((done, p.file_name().unwrap().to_string_lossy().into_owned()))
        })
        .unwrap();
        let names: Vec<_> = order.iter().map(|(_, n)| n.as_str()).collect();
        assert_eq!(names, ["nuevo.md", "medio.md", "viejo.md"]);
        assert_eq!(order.last().unwrap().0, 2);
    }

    // --- Cambios sueltos (lo que avisa el vigilante de carpetas) -----------------------

    fn paths(f: &Fixture, index: &mut FakeIndex, changed: &[&Path]) -> SyncReport {
        let changed: Vec<PathBuf> = changed.iter().map(|p| p.to_path_buf()).collect();
        sync_paths(f.ctx(), std::slice::from_ref(&f.root), &changed, index).unwrap()
    }

    fn names(index: &FakeIndex) -> Vec<String> {
        let mut names: Vec<_> = index.files.values().map(|x| x.name.clone()).collect();
        names.sort_unstable();
        names
    }

    /// Lo que devolvería una búsqueda: ninguna ruta del índice puede apuntar a un archivo
    /// que ya no existe (un «resultado roto»).
    fn assert_no_broken_results(index: &FakeIndex) {
        for file in index.files.values() {
            assert!(
                Path::new(&file.path).is_file(),
                "resultado roto: {}",
                file.path
            );
        }
    }

    #[test]
    fn a_new_file_is_indexed_without_walking_the_folder() {
        let f = fixture();
        write(&f, "viejo.md", "ya estaba");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let nuevo = write(&f, "BD/tema5.md", "transacciones y bloqueos");
        let r = paths(&f, &mut index, &[&nuevo]);
        assert_eq!((r.added, r.updated, r.removed), (1, 0, 0));
        assert_eq!(names(&index), ["tema5.md", "viejo.md"]);
        assert_eq!(index.upserts, 2); // el que ya estaba no se ha tocado
    }

    #[test]
    fn an_edited_file_gets_its_new_content() {
        let f = fixture();
        let p = write(&f, "a.md", "versión uno");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        std::fs::write(&p, "versión dos, con normalización").unwrap();
        set_mtime(&p, 2_000_000);
        let r = paths(&f, &mut index, &[&p]);
        assert_eq!(r.updated, 1);
        let file = index.files.values().next().unwrap();
        assert!(file.chunks[0].1.contains("normalización"));
        assert_eq!(index.files.len(), 1); // el mismo archivo, no uno más
    }

    #[test]
    fn a_deleted_file_disappears() {
        let f = fixture();
        let a = write(&f, "a.md", "se queda");
        let b = write(&f, "b.md", "se borra");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        std::fs::remove_file(&b).unwrap();
        let r = paths(&f, &mut index, &[&b]);
        assert_eq!(r.removed, 1);
        assert_eq!(names(&index), ["a.md"]);
        assert_no_broken_results(&index);
        assert!(a.is_file());
    }

    #[test]
    fn a_renamed_file_keeps_its_id_and_leaves_no_broken_result() {
        let f = fixture();
        let old = write(&f, "apuntes.md", "las formas normales evitan redundancia");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let (id, chunks) = {
            let file = index.files.values().next().unwrap();
            (file.file_id, file.chunks.clone())
        };

        let new = old.with_file_name("Tema 4 - Normalización.md");
        std::fs::rename(&old, &new).unwrap();
        // Windows avisa de las dos rutas: la que desaparece y la que aparece.
        let r = paths(&f, &mut index, &[&old, &new]);

        assert_eq!((r.moved, r.added, r.removed), (1, 0, 0));
        assert_eq!(index.files.len(), 1);
        let file = &index.files[&id]; // mismo id: conserva fragmentos e historial
        assert_eq!(file.name, "Tema 4 - Normalización.md");
        assert_eq!(file.chunks, chunks);
        assert_no_broken_results(&index);
        assert!(f.db.lock().unwrap().file_state(&old).unwrap().is_none());
    }

    #[test]
    fn a_file_moved_to_another_folder_is_found_in_its_new_place() {
        let f = fixture();
        let old = write(&f, "Descargas/practica.md", "enunciado de la práctica 3");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let new = f.dir.path().join("BD").join("practica.md");
        std::fs::create_dir_all(new.parent().unwrap()).unwrap();
        std::fs::rename(&old, &new).unwrap();
        // El orden de los avisos no importa.
        let r = paths(&f, &mut index, &[&new, &old]);
        assert_eq!((r.moved, r.removed), (1, 0));
        assert_eq!(
            index.files.values().next().unwrap().path,
            new.display().to_string()
        );
        assert_no_broken_results(&index);
    }

    #[test]
    fn renaming_a_folder_moves_everything_inside() {
        let f = fixture();
        write(&f, "BD/tema1.md", "modelo relacional");
        write(&f, "BD/practicas/p1.sql", "select * from alumnos");
        write(&f, "otra/suelto.md", "no se toca");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let ids: HashSet<i64> = index.files.keys().copied().collect();

        let (old, new) = (f.dir.path().join("BD"), f.dir.path().join("Bases de datos"));
        std::fs::rename(&old, &new).unwrap();
        let r = paths(&f, &mut index, &[&old, &new]);

        assert_eq!((r.moved, r.added, r.removed), (2, 0, 0));
        assert_eq!(index.files.keys().copied().collect::<HashSet<_>>(), ids);
        assert!(
            index
                .files
                .values()
                .any(|x| x.path.ends_with(r"Bases de datos\practicas\p1.sql"))
        );
        assert_no_broken_results(&index);
    }

    #[test]
    fn deleting_a_folder_removes_everything_inside() {
        let f = fixture();
        write(&f, "BD/tema1.md", "uno");
        write(&f, "BD/sub/tema2.md", "dos");
        write(
            &f,
            "BD2/otro.md",
            "se parece el nombre, pero es otra carpeta",
        );
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let folder = f.dir.path().join("BD");
        std::fs::remove_dir_all(&folder).unwrap();
        let r = paths(&f, &mut index, &[&folder]);
        assert_eq!(r.removed, 2);
        assert_eq!(names(&index), ["otro.md"]);
        assert_no_broken_results(&index);
    }

    #[test]
    fn a_file_moved_into_an_ignored_folder_leaves_the_index() {
        let f = fixture();
        let old = write(&f, "app.js", "console.log('hola')");
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let new = f.dir.path().join("node_modules").join("app.js");
        std::fs::create_dir_all(new.parent().unwrap()).unwrap();
        std::fs::rename(&old, &new).unwrap();
        let r = paths(&f, &mut index, &[&old, &new]);
        assert_eq!((r.removed, r.moved, r.added), (1, 0, 0));
        assert!(index.files.is_empty());
    }

    #[test]
    fn changes_outside_indexed_folders_or_in_useless_formats_are_ignored() {
        let f = fixture();
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().join("fuera.md");
        std::fs::write(&outside, "no es de aquí").unwrap();
        let binary = write(&f, "programa.exe", "MZ");
        let r = paths(&f, &mut index, &[&outside, &binary]);
        assert_eq!(r, SyncReport::default());
        assert!(index.files.is_empty());
    }

    #[test]
    fn the_same_change_reported_twice_is_processed_once() {
        let f = fixture();
        let mut index = FakeIndex::default();
        sync(&f, &mut index);
        let p = write(&f, "BD/a.md", "hola");
        // El archivo y su carpeta: los dos avisos hablan del mismo archivo.
        let r = paths(&f, &mut index, &[&p, &p, p.parent().unwrap()]);
        assert_eq!(r.added, 1);
        assert_eq!(index.upserts, 1);
    }

    // --- Cierre a mitad de indexado -----------------------------------------------------

    /// Índice que pierde lo no confirmado al "cerrarse", como Tantivy.
    #[derive(Default)]
    struct CrashyIndex {
        committed: HashMap<i64, IndexedFile>,
        staged_upserts: Vec<IndexedFile>,
        staged_removals: Vec<i64>,
    }

    impl CrashyIndex {
        /// La app se cierra de golpe: lo que no se confirmó, se pierde.
        fn crash(&mut self) {
            self.staged_upserts.clear();
            self.staged_removals.clear();
        }
    }

    impl IndexSink for CrashyIndex {
        fn upsert(&mut self, file: &IndexedFile) -> Result<(), String> {
            self.staged_upserts.push(file.clone());
            Ok(())
        }
        fn remove(&mut self, ids: &[i64]) -> Result<(), String> {
            self.staged_removals.extend(ids);
            Ok(())
        }
        fn commit(&mut self) -> Result<(), String> {
            for id in self.staged_removals.drain(..) {
                self.committed.remove(&id);
            }
            for file in self.staged_upserts.drain(..) {
                self.committed.insert(file.file_id, file);
            }
            Ok(())
        }
    }

    #[test]
    fn a_crash_mid_indexing_loses_nothing_after_recovery() {
        let f = fixture();
        for i in 0..5 {
            let p = write(&f, &format!("t{i}.md"), &format!("tema {i}"));
            set_mtime(&p, 1_000 + i);
        }
        let mut index = CrashyIndex::default();
        // Se leen 3 de 5 archivos y la app se cierra sin llegar a confirmar el índice.
        let plan = scan_root(f.ctx(), &f.root, |_| {}).unwrap();
        let mut run = PlanRun::new(plan);
        for _ in 0..3 {
            assert!(run.step(f.ctx(), &mut index).unwrap());
        }
        index.crash();
        assert!(index.committed.is_empty());

        // Al volver a arrancar: se reenvía lo guardado, sin leer el disco…
        assert_eq!(recover(&f.db, &mut index).unwrap(), (3, 0));
        assert_eq!(index.committed.len(), 3);
        assert_eq!(recover(&f.db, &mut index).unwrap(), (0, 0)); // y solo una vez
        // …y la revisión sigue donde iba: solo lee los 2 que faltaban.
        let plan = scan_root(f.ctx(), &f.root, |_| {}).unwrap();
        assert_eq!(plan.to_read(), 2);
        apply_plan(f.ctx(), plan, &mut index, |_, _| {}).unwrap();
        assert_eq!(index.committed.len(), 5);
    }

    #[test]
    fn a_crash_after_deleting_does_not_leave_ghost_results() {
        let f = fixture();
        write(&f, "a.md", "se queda");
        let b = write(&f, "b.md", "se borra");
        let mut index = CrashyIndex::default();
        sync_root(f.ctx(), &f.root, &mut index).unwrap();
        assert_eq!(index.committed.len(), 2);

        // Se borra de SQLite y la app se cierra antes de confirmar el índice.
        std::fs::remove_file(&b).unwrap();
        let gone = f.db.lock().unwrap().file_state(&b).unwrap().unwrap().id;
        f.db.lock().unwrap().delete_files(&[gone]).unwrap();
        index.remove(&[gone]).unwrap();
        index.crash();
        assert_eq!(index.committed.len(), 2); // el índice aún enseñaría b.md

        assert_eq!(recover(&f.db, &mut index).unwrap(), (0, 1));
        let left: Vec<_> = index.committed.values().map(|x| x.name.as_str()).collect();
        assert_eq!(left, ["a.md"]);
    }

    #[test]
    fn documents_saved_by_name_are_read_once_a_reader_exists() {
        // Un .docx guardado solo por nombre (como en la Fase 2), sin cambiar desde entonces.
        let f = fixture();
        let path = f.dir.path().join("Tema 2.docx");
        let xml = r#"<w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Álgebra relacional</w:t></w:r></w:p></w:body></w:document>"#;
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(&mut zip, xml.as_bytes()).unwrap();
        zip.finish().unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        let (size, mtime) = size_and_mtime(&meta);
        f.db.lock()
            .unwrap()
            .upsert_file(
                &NewFile {
                    root_id: f.root.id,
                    path: path.clone(),
                    name: "Tema 2.docx".into(),
                    ext: "docx".into(),
                    size,
                    mtime,
                    content_hash: None,
                    error: None,
                    error_kind: None,
                },
                &[],
            )
            .unwrap();

        let mut index = FakeIndex::default();
        let r = sync(&f, &mut index);
        assert_eq!(r.updated, 1); // se ha leído, aunque no había cambiado
        assert!(
            index.files.values().next().unwrap().chunks[0]
                .1
                .contains("Álgebra")
        );
        // Y una vez leído, ya no se vuelve a leer.
        assert_eq!(sync(&f, &mut index).unchanged, 1);
    }

    #[test]
    fn a_document_that_failed_is_not_retried_until_it_changes() {
        let f = fixture();
        write(&f, "roto.docx", "esto no es un zip");
        let mut index = FakeIndex::default();
        assert_eq!(sync(&f, &mut index).errors, 1);
        let again = sync(&f, &mut index);
        assert_eq!((again.unchanged, again.errors), (1, 0));
    }

    #[test]
    fn word_documents_are_searchable_by_content() {
        let f = fixture();
        let path = f.dir.path().join("Tema 4.docx");
        let xml = r#"<w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Dependencias funcionales y claves candidatas</w:t></w:r></w:p></w:body></w:document>"#;
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(&mut zip, xml.as_bytes()).unwrap();
        zip.finish().unwrap();

        let mut index = FakeIndex::default();
        let r = sync(&f, &mut index);
        assert_eq!((r.added, r.errors), (1, 0));
        let file = index.files.values().next().unwrap();
        assert!(file.chunks[0].1.contains("claves candidatas"));
    }
}
