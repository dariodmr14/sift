//! Sincronización incremental de una carpeta con la base de datos y el índice.
//!
//! Por cada archivo:
//! - Mismo tamaño y fecha que lo guardado → no se toca (ni se lee).
//! - Cambió la fecha pero el contenido es el mismo (mismo hash) → solo se apunta la fecha.
//! - Nuevo o cambiado → se extrae, se trocea y se guarda (SQLite primero, luego el índice).
//! - Ya no está → se borra de los dos.
//!
//! El indexador no sabe nada de Tantivy: escribe en un [`IndexSink`], que implementa
//! `sift-search` (y en los tests, uno falso).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use sift_storage::{Database, NewChunk, NewFile, Root, StorageError};
use walkdir::WalkDir;

use crate::chunk::chunk_text;
use crate::ignore::IgnoreRules;

/// Tamaño máximo de un archivo para leer su contenido (más grandes: solo por nombre).
pub const MAX_CONTENT_BYTES: u64 = 5 * 1024 * 1024;
/// Cada cuántos archivos se confirma el índice (para que se vean resultados mientras).
const COMMIT_EVERY: usize = 500;

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
    /// Archivos que no se pudieron leer (se indexan por nombre y se apunta el motivo).
    pub errors: usize,
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

/// Sincroniza una carpeta. `progress` recibe (archivos vistos, ruta actual).
pub fn sync_root(
    db: &Mutex<Database>,
    root: &Root,
    rules: &IgnoreRules,
    sink: &mut dyn IndexSink,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, &Path),
) -> Result<SyncReport, SyncError> {
    let lock = || db.lock().unwrap_or_else(|p| p.into_inner());
    let known = lock().file_states(root.id)?;
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut report = SyncReport::default();
    let mut since_commit = 0;

    let walker = WalkDir::new(&root.path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Las carpetas ignoradas ni se recorren (node_modules puede tener 100.000 archivos).
            let rel = e.path().strip_prefix(&root.path).unwrap_or(e.path());
            rel.as_os_str().is_empty() || !rules.is_ignored(rel)
        });

    for entry in walker {
        if cancel.load(Ordering::Relaxed) {
            sink.commit().map_err(SyncError::Index)?;
            return Err(SyncError::Cancelled);
        }
        // Una carpeta sin permiso no para el indexado del resto.
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let Ok(meta) = entry.metadata() else { continue };
        let size = meta.len() as i64;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as i64);
        seen.insert(path.clone());
        progress(seen.len(), &path);

        let previous = known.get(&path);
        if previous.is_some_and(|p| p.size == size && p.mtime == mtime) {
            report.unchanged += 1;
            continue;
        }

        let ext = sift_extract::extension(&path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (text, hash, error) = match sift_extract::extract(&path, MAX_CONTENT_BYTES) {
            Ok(Some(e)) => (Some(e.text), Some(e.hash), None),
            Ok(None) => (None, None, None),
            Err(e) => (None, None, Some(e.to_string())),
        };

        // Cambió la fecha pero no el contenido: no hace falta reindexar.
        if let (Some(p), Some(h)) = (previous, &hash)
            && p.content_hash.as_ref() == Some(h)
        {
            lock().touch_file(p.id, size, mtime)?;
            report.unchanged += 1;
            continue;
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
        let file = NewFile {
            root_id: root.id,
            path: path.clone(),
            name: name.clone(),
            ext: ext.clone(),
            size,
            mtime,
            content_hash: hash,
            error: error.clone(),
        };
        // SQLite primero (fuente de verdad); si el índice fallara, se puede reconstruir.
        let (file_id, chunk_ids) = lock().upsert_file(&file, &chunks)?;
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
        since_commit += 1;
        if since_commit >= COMMIT_EVERY {
            sink.commit().map_err(SyncError::Index)?;
            since_commit = 0;
        }
    }

    // Lo que estaba guardado y ya no existe (o ahora se ignora).
    let gone: Vec<i64> = known
        .iter()
        .filter(|(p, _)| !seen.contains(*p))
        .map(|(_, s)| s.id)
        .collect();
    if !gone.is_empty() {
        lock().delete_files(&gone)?;
        sink.remove(&gone).map_err(SyncError::Index)?;
        report.removed = gone.len();
    }
    sink.commit().map_err(SyncError::Index)?;
    lock().mark_root_indexed(root.id)?;
    Ok(report)
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
        }
    }

    fn sync(f: &Fixture, index: &mut FakeIndex) -> SyncReport {
        sync_root(
            &f.db,
            &f.root,
            &f.rules,
            index,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap()
    }

    fn write(f: &Fixture, rel: &str, content: &str) -> PathBuf {
        let p = f.dir.path().join(rel);
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
        let cancel = AtomicBool::new(true);
        let r = sync_root(&f.db, &f.root, &f.rules, &mut index, &cancel, |_, _| {});
        assert!(matches!(r, Err(SyncError::Cancelled)));
        assert_eq!(index.commits, 1);
    }
}
