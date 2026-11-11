//! Repositorio del indexado: carpetas (`roots`), patrones a ignorar, archivos y sus
//! fragmentos (`chunks`), y aperturas (`opens`). Ver ADR 0006.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::{Database, StorageError};

/// Una carpeta indexada, con su estado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root {
    pub id: i64,
    pub path: PathBuf,
    pub last_indexed_at: Option<String>,
    pub files: i64,
    /// Archivos que no se pudieron leer.
    pub errors: i64,
}

/// Lo que se sabe de un archivo ya indexado, para decidir si hay que reprocesarlo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileState {
    pub id: i64,
    pub size: i64,
    pub mtime: i64,
    pub content_hash: Option<String>,
}

/// Un archivo listo para guardar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFile {
    pub root_id: i64,
    pub path: PathBuf,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub mtime: i64,
    pub content_hash: Option<String>,
    pub error: Option<String>,
    /// Categoría del error (código de `sift_extract::UnreadReason`), si lo hubo.
    pub error_kind: Option<String>,
}

/// Archivos que no se pudieron leer por un mismo motivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadGroup {
    /// Código del motivo (`unsupported`, `tooLarge`…).
    pub kind: String,
    pub count: i64,
    /// Unas pocas rutas de ejemplo.
    pub examples: Vec<String>,
}

/// Un fragmento de texto de un archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChunk {
    pub ordinal: i64,
    /// Desplazamiento en caracteres desde el principio del texto.
    pub start: i64,
    pub text: String,
}

/// (id del fragmento, texto).
pub type ChunkText = (i64, String);

/// Un archivo indexado (lo justo para reconstruir el índice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRow {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub ext: String,
}

/// Un fragmento guardado, con lo necesario para (re)construir el índice de búsqueda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredChunk {
    pub id: i64,
    pub file_id: i64,
    pub path: String,
    pub name: String,
    pub ext: String,
    pub ordinal: i64,
    pub text: String,
}

impl Database {
    pub fn list_roots(&self) -> Result<Vec<Root>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT r.id, r.path, r.last_indexed_at,
                    COUNT(f.id), COUNT(f.error)
             FROM roots r LEFT JOIN files f ON f.root_id = r.id
             GROUP BY r.id ORDER BY r.path COLLATE NOCASE",
        )?;
        let roots = stmt
            .query_map([], |r| {
                Ok(Root {
                    id: r.get(0)?,
                    path: PathBuf::from(r.get::<_, String>(1)?),
                    last_indexed_at: r.get(2)?,
                    files: r.get(3)?,
                    errors: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(roots)
    }

    /// Añade una carpeta. Rechaza las que ya están, las que están dentro de otra ya
    /// indexada y las que contienen a otra: el mismo archivo se indexaría dos veces.
    pub fn add_root(&mut self, path: &Path) -> Result<Root, StorageError> {
        if !path.is_absolute() {
            return Err(StorageError::Conflict(
                "la carpeta tiene que ser una ruta completa".into(),
            ));
        }
        for existing in self.list_roots()? {
            if same_or_inside(path, &existing.path) || same_or_inside(&existing.path, path) {
                return Err(StorageError::Conflict(format!(
                    "se solapa con una carpeta que ya se indexa: {}",
                    existing.path.display()
                )));
            }
        }
        self.conn.execute(
            "INSERT INTO roots (path) VALUES (?1)",
            [path.display().to_string()],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Root {
            id,
            path: path.to_path_buf(),
            last_indexed_at: None,
            files: 0,
            errors: 0,
        })
    }

    /// Quita una carpeta y todo lo indexado en ella. Devuelve los ids de los archivos
    /// borrados, para quitarlos también del índice de búsqueda.
    pub fn remove_root(&mut self, id: i64) -> Result<Vec<i64>, StorageError> {
        let tx = self.conn.transaction()?;
        let files: Vec<i64> = tx
            .prepare("SELECT id FROM files WHERE root_id = ?1")?
            .query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        if tx.execute("DELETE FROM roots WHERE id = ?1", [id])? == 0 {
            return Err(StorageError::NotFound(id));
        }
        tx.commit()?;
        Ok(files)
    }

    pub fn mark_root_indexed(&mut self, id: i64) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE roots SET last_indexed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn ignore_patterns(&self) -> Result<Vec<String>, StorageError> {
        Ok(self
            .conn
            .prepare("SELECT pattern FROM ignore_patterns ORDER BY position")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?)
    }

    /// Sustituye los patrones (recortados, sin vacíos ni repetidos).
    pub fn set_ignore_patterns(
        &mut self,
        patterns: &[String],
    ) -> Result<Vec<String>, StorageError> {
        let mut clean: Vec<String> = Vec::new();
        for p in patterns.iter().map(|p| p.trim()).filter(|p| !p.is_empty()) {
            if !clean.iter().any(|c| c == p) {
                clean.push(p.to_owned());
            }
        }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM ignore_patterns", [])?;
        for (i, p) in clean.iter().enumerate() {
            tx.execute(
                "INSERT INTO ignore_patterns (position, pattern) VALUES (?1, ?2)",
                params![i as i64, p],
            )?;
        }
        tx.commit()?;
        Ok(clean)
    }

    /// Estado de cada archivo ya indexado de una carpeta, por ruta.
    pub fn file_states(&self, root_id: i64) -> Result<HashMap<PathBuf, FileState>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, id, size, mtime, content_hash FROM files WHERE root_id = ?1")?;
        let rows = stmt.query_map([root_id], |r| {
            Ok((
                PathBuf::from(r.get::<_, String>(0)?),
                FileState {
                    id: r.get(1)?,
                    size: r.get(2)?,
                    mtime: r.get(3)?,
                    content_hash: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Guarda (o actualiza) un archivo con sus fragmentos, en una transacción.
    /// Devuelve el id del archivo y los ids de sus fragmentos, en orden.
    pub fn upsert_file(
        &mut self,
        file: &NewFile,
        chunks: &[NewChunk],
    ) -> Result<(i64, Vec<i64>), StorageError> {
        let tx = self.conn.transaction()?;
        let path = file.path.display().to_string();
        let existing: Option<i64> = tx
            .query_row("SELECT id FROM files WHERE path = ?1", [&path], |r| {
                r.get(0)
            })
            .optional()?;
        let id = match existing {
            Some(id) => {
                tx.execute(
                    "UPDATE files SET root_id = ?1, name = ?2, ext = ?3, size = ?4, mtime = ?5,
                            content_hash = ?6, error = ?7, error_kind = ?8, search_synced = 0,
                            indexed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?9",
                    params![
                        file.root_id,
                        file.name,
                        file.ext,
                        file.size,
                        file.mtime,
                        file.content_hash,
                        file.error,
                        file.error_kind,
                        id
                    ],
                )?;
                tx.execute("DELETE FROM chunks WHERE file_id = ?1", [id])?;
                id
            }
            None => {
                tx.execute(
                    "INSERT INTO files
                        (root_id, path, name, ext, size, mtime, content_hash, error, error_kind,
                         search_synced)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)",
                    params![
                        file.root_id,
                        path,
                        file.name,
                        file.ext,
                        file.size,
                        file.mtime,
                        file.content_hash,
                        file.error,
                        file.error_kind
                    ],
                )?;
                tx.last_insert_rowid()
            }
        };
        let mut chunk_ids = Vec::with_capacity(chunks.len());
        for c in chunks {
            tx.execute(
                "INSERT INTO chunks (file_id, ordinal, start, text) VALUES (?1, ?2, ?3, ?4)",
                params![id, c.ordinal, c.start, c.text],
            )?;
            chunk_ids.push(tx.last_insert_rowid());
        }
        tx.commit()?;
        Ok((id, chunk_ids))
    }

    /// El archivo cambió de fecha pero no de contenido: solo se apunta la fecha nueva.
    pub fn touch_file(&mut self, id: i64, size: i64, mtime: i64) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE files SET size = ?1, mtime = ?2 WHERE id = ?3",
            params![size, mtime, id],
        )?;
        Ok(())
    }

    /// Borra archivos (y sus fragmentos) y, en la misma transacción, apunta sus ids como
    /// pendientes de quitar del índice de búsqueda (ver [`Database::pending_removals`]).
    pub fn delete_files(&mut self, ids: &[i64]) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        for id in ids {
            tx.execute("DELETE FROM files WHERE id = ?1", [id])?;
            tx.execute(
                "INSERT OR IGNORE INTO index_removals (file_id) VALUES (?1)",
                [id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Lo que se sabe de un archivo por su ruta (o `None` si no está indexado).
    pub fn file_state(&self, path: &Path) -> Result<Option<FileState>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, size, mtime, content_hash FROM files WHERE path = ?1",
                [path.display().to_string()],
                |r| {
                    Ok(FileState {
                        id: r.get(0)?,
                        size: r.get(1)?,
                        mtime: r.get(2)?,
                        content_hash: r.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// Los archivos indexados que son `path` o están dentro de esa carpeta. Sirve cuando
    /// se borra, mueve o renombra una carpeta entera.
    pub fn files_under(&self, path: &Path) -> Result<HashMap<PathBuf, FileState>, StorageError> {
        let exact = path.display().to_string();
        // `C:\Uni\BD` no debe arrastrar a `C:\Uni\BD2`: se compara con la barra final.
        let inside = format!(
            "{}{}",
            exact.trim_end_matches(['\\', '/']),
            std::path::MAIN_SEPARATOR
        );
        let mut stmt = self.conn.prepare(
            "SELECT path, id, size, mtime, content_hash FROM files
             WHERE path = ?1 OR substr(path, 1, ?3) = ?2",
        )?;
        let rows = stmt.query_map(params![exact, inside, inside.chars().count() as i64], |r| {
            Ok((
                PathBuf::from(r.get::<_, String>(0)?),
                FileState {
                    id: r.get(1)?,
                    size: r.get(2)?,
                    mtime: r.get(3)?,
                    content_hash: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// El archivo `id` ahora está en otra ruta (se renombró o se movió) con el mismo
    /// contenido: cambia la ruta y conserva el id, los fragmentos y el historial de
    /// aperturas. Devuelve sus fragmentos, para actualizar el índice de búsqueda.
    pub fn move_file(&mut self, id: i64, to: &NewFile) -> Result<Vec<(i64, String)>, StorageError> {
        let tx = self.conn.transaction()?;
        // Si en el destino ya había otro archivo indexado, el movido lo sustituye.
        let replaced: Option<i64> = tx
            .query_row(
                "SELECT id FROM files WHERE path = ?1 AND id <> ?2",
                params![to.path.display().to_string(), id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = replaced {
            tx.execute("DELETE FROM files WHERE id = ?1", [old])?;
            tx.execute(
                "INSERT OR IGNORE INTO index_removals (file_id) VALUES (?1)",
                [old],
            )?;
        }
        tx.execute(
            "UPDATE files SET root_id = ?1, path = ?2, name = ?3, ext = ?4, size = ?5, mtime = ?6,
                    search_synced = 0
             WHERE id = ?7",
            params![
                to.root_id,
                to.path.display().to_string(),
                to.name,
                to.ext,
                to.size,
                to.mtime,
                id
            ],
        )?;
        let chunks = tx
            .prepare("SELECT id, text FROM chunks WHERE file_id = ?1 ORDER BY ordinal")?
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<(i64, String)>, _>>()?;
        tx.commit()?;
        Ok(chunks)
    }

    /// El índice de búsqueda acaba de confirmar: todo lo guardado hasta ahora está en él.
    pub fn mark_search_synced(&mut self) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE files SET search_synced = 1 WHERE search_synced = 0",
            [],
        )?;
        tx.execute("DELETE FROM index_removals", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Ids de archivos borrados que el índice de búsqueda aún no ha quitado (la app se
    /// cerró antes de confirmar).
    pub fn pending_removals(&self) -> Result<Vec<i64>, StorageError> {
        Ok(self
            .conn
            .prepare("SELECT file_id FROM index_removals")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?)
    }

    /// Archivos guardados que el índice de búsqueda aún no ha confirmado, con sus
    /// fragmentos: se pueden reenviar sin volver a leer el disco.
    pub fn unsynced_files(&self) -> Result<Vec<(FileRow, Vec<ChunkText>)>, StorageError> {
        let files: Vec<FileRow> = self
            .conn
            .prepare("SELECT id, path, name, ext FROM files WHERE search_synced = 0 ORDER BY id")?
            .query_map([], |r| {
                Ok(FileRow {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    name: r.get(2)?,
                    ext: r.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        let mut stmt = self
            .conn
            .prepare("SELECT id, text FROM chunks WHERE file_id = ?1 ORDER BY ordinal")?;
        let mut out = Vec::with_capacity(files.len());
        for file in files {
            let chunks = stmt
                .query_map([file.id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<(i64, String)>, _>>()?;
            out.push((file, chunks));
        }
        Ok(out)
    }

    /// Los archivos de una carpeta que no se pudieron leer, agrupados por motivo (de más a
    /// menos), con hasta `examples` rutas de ejemplo por grupo. No los lista todos: pueden
    /// ser miles.
    pub fn unread_summary(
        &self,
        root_id: i64,
        examples: usize,
    ) -> Result<Vec<UnreadGroup>, StorageError> {
        let mut groups: Vec<UnreadGroup> = self
            .conn
            .prepare(
                "SELECT error_kind, COUNT(*) FROM files
                 WHERE root_id = ?1 AND error_kind IS NOT NULL
                 GROUP BY error_kind ORDER BY COUNT(*) DESC, error_kind",
            )?
            .query_map([root_id], |r| {
                Ok(UnreadGroup {
                    kind: r.get(0)?,
                    count: r.get(1)?,
                    examples: Vec::new(),
                })
            })?
            .collect::<Result<_, _>>()?;
        let mut stmt = self.conn.prepare(
            "SELECT path FROM files WHERE root_id = ?1 AND error_kind = ?2
                 ORDER BY mtime DESC LIMIT ?3",
        )?;
        for group in &mut groups {
            group.examples = stmt
                .query_map(params![root_id, group.kind, examples as i64], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
        }
        Ok(groups)
    }

    /// Recorre todos los fragmentos (para reconstruir el índice de búsqueda desde cero).
    pub fn for_each_chunk(&self, mut f: impl FnMut(StoredChunk)) -> Result<usize, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.file_id, f.path, f.name, f.ext, c.ordinal, c.text
             FROM chunks c JOIN files f ON f.id = c.file_id ORDER BY c.id",
        )?;
        let mut rows = stmt.query([])?;
        let mut n = 0;
        while let Some(r) = rows.next()? {
            f(StoredChunk {
                id: r.get(0)?,
                file_id: r.get(1)?,
                path: r.get(2)?,
                name: r.get(3)?,
                ext: r.get(4)?,
                ordinal: r.get(5)?,
                text: r.get(6)?,
            });
            n += 1;
        }
        Ok(n)
    }

    /// Recorre cada archivo con sus fragmentos en orden (también los que no tienen
    /// fragmentos: se indexan por nombre). Para reconstruir el índice de búsqueda.
    pub fn for_each_file_with_chunks(
        &self,
        mut f: impl FnMut(&FileRow, Vec<(i64, String)>),
    ) -> Result<usize, StorageError> {
        let files: Vec<FileRow> = self
            .conn
            .prepare("SELECT id, path, name, ext FROM files ORDER BY id")?
            .query_map([], |r| {
                Ok(FileRow {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    name: r.get(2)?,
                    ext: r.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        let mut stmt = self
            .conn
            .prepare("SELECT id, text FROM chunks WHERE file_id = ?1 ORDER BY ordinal")?;
        for file in &files {
            let chunks = stmt
                .query_map([file.id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<(i64, String)>, _>>()?;
            f(file, chunks);
        }
        Ok(files.len())
    }

    pub fn count_chunks(&self) -> Result<i64, StorageError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get(0))?)
    }

    /// ¿Está esta ruta en el índice?
    pub fn is_indexed(&self, path: &Path) -> Result<bool, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM files WHERE path = ?1",
                [path.display().to_string()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Apunta que has abierto un archivo (para la frecuencia de uso de la Fase 5).
    pub fn record_file_open(&mut self, path: &Path) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO opens (file_id) SELECT id FROM files WHERE path = ?1",
            [path.display().to_string()],
        )?;
        Ok(())
    }
}

/// ¿Es `a` la misma carpeta que `b` o está dentro de ella? Sin distinguir mayúsculas,
/// como Windows.
fn same_or_inside(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.display().to_string().replace('/', "\\").to_lowercase();
        format!("{}\\", s.trim_end_matches('\\'))
    };
    norm(a).starts_with(&norm(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(root: i64, path: &str) -> NewFile {
        NewFile {
            root_id: root,
            path: PathBuf::from(path),
            name: Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into(),
            ext: "md".into(),
            size: 10,
            mtime: 1,
            content_hash: Some("h1".into()),
            error: None,
            error_kind: None,
        }
    }

    fn chunk(i: i64, text: &str) -> NewChunk {
        NewChunk {
            ordinal: i,
            start: i * 100,
            text: text.into(),
        }
    }

    #[test]
    fn default_ignore_patterns_come_with_the_migration() {
        let db = Database::open_in_memory().unwrap();
        let p = db.ignore_patterns().unwrap();
        assert!(p.contains(&"node_modules".to_string()) && p.contains(&".git".to_string()));
    }

    #[test]
    fn roots_cannot_overlap() {
        let mut db = Database::open_in_memory().unwrap();
        db.add_root(Path::new(r"C:\Uni")).unwrap();
        assert!(matches!(
            db.add_root(Path::new(r"C:\Uni")),
            Err(StorageError::Conflict(_))
        ));
        assert!(matches!(
            db.add_root(Path::new(r"c:\uni\BD")),
            Err(StorageError::Conflict(_))
        ));
        assert!(matches!(
            db.add_root(Path::new(r"C:\")),
            Err(StorageError::Conflict(_))
        ));
        // Un nombre que solo *empieza* igual no es la misma carpeta.
        db.add_root(Path::new(r"C:\Universo")).unwrap();
        assert!(matches!(
            db.add_root(Path::new("relativa")),
            Err(StorageError::Conflict(_))
        ));
    }

    #[test]
    fn upsert_replaces_chunks_and_keeps_the_file_id() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        let (id, c1) = db
            .upsert_file(
                &file(root, r"C:\Uni\a.md"),
                &[chunk(0, "uno"), chunk(1, "dos")],
            )
            .unwrap();
        assert_eq!(c1.len(), 2);
        let (id2, c2) = db
            .upsert_file(&file(root, r"C:\Uni\a.md"), &[chunk(0, "nuevo")])
            .unwrap();
        assert_eq!(id, id2);
        assert_eq!(db.count_chunks().unwrap(), 1);
        let mut texts = vec![];
        db.for_each_chunk(|c| texts.push((c.id, c.text))).unwrap();
        assert_eq!(texts, vec![(c2[0], "nuevo".to_string())]);
    }

    #[test]
    fn file_states_drive_incremental_sync() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        let (id, _) = db.upsert_file(&file(root, r"C:\Uni\a.md"), &[]).unwrap();
        db.touch_file(id, 20, 5).unwrap();
        let states = db.file_states(root).unwrap();
        let s = &states[Path::new(r"C:\Uni\a.md")];
        assert_eq!(
            (s.id, s.size, s.mtime, s.content_hash.as_deref()),
            (id, 20, 5, Some("h1"))
        );
    }

    #[test]
    fn removing_a_root_cascades_and_reports_files() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        let (a, _) = db
            .upsert_file(&file(root, r"C:\Uni\a.md"), &[chunk(0, "x")])
            .unwrap();
        db.record_file_open(Path::new(r"C:\Uni\a.md")).unwrap();
        assert_eq!(db.remove_root(root).unwrap(), vec![a]);
        assert_eq!(db.count_chunks().unwrap(), 0);
        assert!(db.list_roots().unwrap().is_empty());
        assert!(matches!(
            db.remove_root(root),
            Err(StorageError::NotFound(_))
        ));
    }

    #[test]
    fn roots_report_files_and_errors() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        db.upsert_file(&file(root, r"C:\Uni\a.md"), &[]).unwrap();
        let mut bad = file(root, r"C:\Uni\b.md");
        bad.error = Some("ilegible".into());
        db.upsert_file(&bad, &[]).unwrap();
        db.mark_root_indexed(root).unwrap();
        let r = &db.list_roots().unwrap()[0];
        assert_eq!((r.files, r.errors), (2, 1));
        assert!(r.last_indexed_at.is_some());
    }

    #[test]
    fn unread_files_are_grouped_by_reason_with_a_few_examples() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap();
        for i in 0..5 {
            let mut f = file(root.id, &format!(r"C:\Uni\raro{i}.txt"));
            f.error = Some("parece un archivo binario".into());
            f.error_kind = Some("unsupported".into());
            f.mtime = i;
            db.upsert_file(&f, &[]).unwrap();
        }
        let mut big = file(root.id, r"C:\Uni\enorme.txt");
        big.error = Some("demasiado grande".into());
        big.error_kind = Some("tooLarge".into());
        db.upsert_file(&big, &[]).unwrap();
        db.upsert_file(&file(root.id, r"C:\Uni\bien.md"), &[])
            .unwrap();

        // Otra carpeta con su propio archivo ilegible: no se mezcla.
        let other = db.add_root(Path::new(r"D:\Otra")).unwrap();
        let mut theirs = file(other.id, r"D:\Otra\x.txt");
        theirs.error_kind = Some("unsupported".into());
        db.upsert_file(&theirs, &[]).unwrap();
        assert_eq!(db.unread_summary(other.id, 3).unwrap()[0].count, 1);

        let groups = db.unread_summary(root.id, 3).unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(
            (groups[0].kind.as_str(), groups[0].count),
            ("unsupported", 5)
        );
        // Solo 3 ejemplos, los más recientes primero.
        assert_eq!(groups[0].examples.len(), 3);
        assert!(groups[0].examples[0].ends_with("raro4.txt"));
        assert_eq!((groups[1].kind.as_str(), groups[1].count), ("tooLarge", 1));
    }

    #[test]
    fn files_under_a_folder_do_not_include_similar_names() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap();
        for p in [
            r"C:\Uni\BD\t1.md",
            r"C:\Uni\BD\sub\t2.md",
            r"C:\Uni\BD2\otro.md",
        ] {
            db.upsert_file(&file(root.id, p), &[]).unwrap();
        }
        let mut under: Vec<_> = db
            .files_under(Path::new(r"C:\Uni\BD"))
            .unwrap()
            .into_keys()
            .map(|p| p.display().to_string())
            .collect();
        under.sort();
        assert_eq!(under, [r"C:\Uni\BD\sub\t2.md", r"C:\Uni\BD\t1.md"]);
        // Y un archivo suelto se encuentra por su ruta exacta.
        assert_eq!(
            db.files_under(Path::new(r"C:\Uni\BD2\otro.md"))
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn moving_a_file_keeps_its_id_and_chunks() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap();
        let (id, chunk_ids) = db
            .upsert_file(&file(root.id, r"C:\Uni\viejo.md"), &[chunk(0, "hola")])
            .unwrap();
        let chunks = db
            .move_file(id, &file(root.id, r"C:\Uni\sub\nuevo.md"))
            .unwrap();
        assert_eq!(chunks, [(chunk_ids[0], "hola".to_string())]);
        assert!(
            db.file_state(Path::new(r"C:\Uni\viejo.md"))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            db.file_state(Path::new(r"C:\Uni\sub\nuevo.md"))
                .unwrap()
                .unwrap()
                .id,
            id
        );
    }

    #[test]
    fn tracks_what_the_search_index_has_not_confirmed() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap();
        let (a, _) = db
            .upsert_file(&file(root.id, r"C:\Uni\a.md"), &[chunk(0, "uno")])
            .unwrap();
        db.mark_search_synced().unwrap();
        // Tras confirmar: un archivo nuevo y uno borrado quedan pendientes.
        let (b, _) = db
            .upsert_file(&file(root.id, r"C:\Uni\b.md"), &[chunk(0, "dos")])
            .unwrap();
        db.delete_files(&[a]).unwrap();
        let unsynced = db.unsynced_files().unwrap();
        assert_eq!(unsynced.len(), 1);
        assert_eq!((unsynced[0].0.id, unsynced[0].1[0].1.as_str()), (b, "dos"));
        assert_eq!(db.pending_removals().unwrap(), [a]);
        db.mark_search_synced().unwrap();
        assert!(db.unsynced_files().unwrap().is_empty());
        assert!(db.pending_removals().unwrap().is_empty());
    }

    #[test]
    fn ignore_patterns_are_cleaned() {
        let mut db = Database::open_in_memory().unwrap();
        let saved = db
            .set_ignore_patterns(&[
                " *.log ".into(),
                "".into(),
                "*.log".into(),
                "secreto".into(),
            ])
            .unwrap();
        assert_eq!(saved, vec!["*.log", "secreto"]);
        assert_eq!(db.ignore_patterns().unwrap(), saved);
    }

    #[test]
    fn walks_files_with_their_chunks_for_rebuilding() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        db.upsert_file(
            &file(root, r"C:\Uni\a.md"),
            &[chunk(0, "uno"), chunk(1, "dos")],
        )
        .unwrap();
        db.upsert_file(&file(root, r"C:\Uni\b.pdf"), &[]).unwrap();
        let mut seen = vec![];
        let n = db
            .for_each_file_with_chunks(|f, chunks| {
                seen.push((
                    f.name.clone(),
                    chunks.into_iter().map(|c| c.1).collect::<Vec<_>>(),
                ))
            })
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(
            seen[0],
            (
                "a.md".to_string(),
                vec!["uno".to_string(), "dos".to_string()]
            )
        );
        assert_eq!(seen[1], ("b.pdf".to_string(), vec![]));
    }

    #[test]
    fn delete_files_removes_their_chunks() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        let (a, _) = db
            .upsert_file(&file(root, r"C:\Uni\a.md"), &[chunk(0, "x")])
            .unwrap();
        db.delete_files(&[a]).unwrap();
        assert_eq!(db.count_chunks().unwrap(), 0);
    }
}
