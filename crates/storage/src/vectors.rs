//! Vectores de significado (Fase 4, ADR 0009). SQLite es su fuente de verdad: el índice
//! HNSW (usearch) se reconstruye desde aquí sin recalcular nada.
//!
//! Un vector es de un **texto**, no de un fragmento: la clave es el hash del texto
//! ([`text_hash`]) y lo comparten todas las copias. El 40 % de los fragmentos de una
//! carpeta real estaban repetidos (las mismas prácticas en varias carpetas); además, el
//! hash no cambia al reindexar un archivo, mientras que el id del fragmento sí.

use rusqlite::{OptionalExtension, params};

use crate::{Database, StorageError};

/// Hash estable de un texto (64 bits de blake3), con signo porque así guarda SQLite los
/// enteros. Es la clave del vector en SQLite y en el índice HNSW.
pub fn text_hash(text: &str) -> i64 {
    let bytes = blake3::hash(text.as_bytes());
    let mut first = [0u8; 8];
    first.copy_from_slice(&bytes.as_bytes()[..8]);
    i64::from_le_bytes(first)
}

/// Qué fragmentos merecen vector (ADR 0009, punto 2): por tipo de archivo y con un tope
/// por archivo, para dejar fuera volcados de datos y archivos de miles de fragmentos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedPolicy {
    /// Grupos de extensiones, en orden de prioridad (el primero se vectoriza antes), con
    /// el máximo de fragmentos por archivo de cada uno.
    pub tiers: Vec<(Vec<String>, u32)>,
}

impl EmbedPolicy {
    /// Documentos (hasta 1.000 fragmentos: un libro entero), apuntes y texto (100) y
    /// código (100). Los formatos de datos (xml, json, html, csv…) no llevan vector.
    pub fn standard() -> EmbedPolicy {
        let list = |exts: &[&str]| exts.iter().map(|e| (*e).to_owned()).collect();
        EmbedPolicy {
            tiers: vec![
                (list(&["pdf", "docx", "pptx", "odt"]), 1000),
                (
                    list(&[
                        "md", "markdown", "mdx", "txt", "text", "tex", "rst", "adoc", "org",
                    ]),
                    100,
                ),
                (
                    list(&[
                        "java", "cs", "py", "c", "h", "cpp", "hpp", "cc", "js", "jsx", "ts", "tsx",
                        "mjs", "cjs", "sql", "sh", "bash", "ps1", "kt", "rs", "go", "php", "rb",
                        "swift", "scala", "r", "m", "lua", "dart", "vue", "svelte", "css", "scss",
                        "hs", "ml", "pl", "asm",
                    ]),
                    100,
                ),
            ],
        }
    }

    /// Condición SQL (sobre `f.ext` y `c.ordinal`) y prioridad de cada fila (`tier`).
    fn sql(&self) -> (String, String) {
        let mut filter = Vec::new();
        let mut tier = String::from("CASE");
        for (i, (exts, cap)) in self.tiers.iter().enumerate() {
            let list = exts
                .iter()
                .map(|e| format!("'{}'", e.replace('\'', "")))
                .collect::<Vec<_>>()
                .join(",");
            filter.push(format!("(f.ext IN ({list}) AND c.ordinal < {cap})"));
            tier.push_str(&format!(" WHEN f.ext IN ({list}) THEN {i}"));
        }
        tier.push_str(" ELSE 99 END");
        let filter = if filter.is_empty() {
            "0".to_owned()
        } else {
            filter.join(" OR ")
        };
        (filter, tier)
    }
}

/// Un vector en SQLite: 384 valores en `f16` (768 bytes). La mitad que `f32`, y los
/// modelos pequeños no pierden nada apreciable.
fn to_blob(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|x| half::f16::from_f32(*x).to_le_bytes())
        .collect()
}

fn from_blob(blob: &[u8]) -> Vec<f32> {
    blob.as_chunks::<2>()
        .0
        .iter()
        .map(|b| half::f16::from_le_bytes(*b).to_f32())
        .collect()
}

impl Database {
    /// Textos que aún no tienen vector de este modelo, según la política, con prioridad:
    /// primero los documentos, y dentro de cada grupo lo modificado más recientemente.
    /// Cada texto sale una vez aunque esté en varios archivos.
    pub fn pending_texts(
        &self,
        model: &str,
        policy: &EmbedPolicy,
        limit: usize,
    ) -> Result<Vec<(i64, String)>, StorageError> {
        let (filter, tier) = policy.sql();
        let sql = format!(
            "SELECT c.text_hash, MIN(c.text), MIN({tier}) AS tier, MAX(f.mtime) AS recent
             FROM chunks c JOIN files f ON f.id = c.file_id
             WHERE ({filter})
               AND NOT EXISTS (SELECT 1 FROM embeddings e
                               WHERE e.model = ?1 AND e.text_hash = c.text_hash)
             GROUP BY c.text_hash
             ORDER BY tier, recent DESC
             LIMIT ?2"
        );
        let rows = self
            .conn
            .prepare(&sql)?
            .query_map(params![model, limit as i64], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// (con vector, que deberían tenerlo) para este modelo: el progreso de «Preparando
    /// búsqueda inteligente · 3.200 de 13.000».
    pub fn embedding_progress(
        &self,
        model: &str,
        policy: &EmbedPolicy,
    ) -> Result<(u64, u64), StorageError> {
        let (filter, _) = policy.sql();
        let sql = format!(
            "SELECT COUNT(*), COUNT(e.text_hash) FROM
               (SELECT DISTINCT c.text_hash FROM chunks c JOIN files f ON f.id = c.file_id
                WHERE ({filter})) t
             LEFT JOIN embeddings e ON e.model = ?1 AND e.text_hash = t.text_hash"
        );
        let (total, done): (i64, i64) = self
            .conn
            .query_row(&sql, params![model], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok((done as u64, total as u64))
    }

    /// Guarda vectores (en una transacción).
    pub fn save_embeddings(
        &mut self,
        model: &str,
        vectors: &[(i64, Vec<f32>)],
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO embeddings (model, text_hash, vector) VALUES (?1, ?2, ?3)",
            )?;
            for (hash, vector) in vectors {
                stmt.execute(params![model, hash, to_blob(vector)])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Recorre todos los vectores de un modelo (para reconstruir el índice HNSW).
    pub fn for_each_embedding(
        &self,
        model: &str,
        mut f: impl FnMut(i64, Vec<f32>),
    ) -> Result<usize, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT text_hash, vector FROM embeddings WHERE model = ?1")?;
        let mut rows = stmt.query([model])?;
        let mut n = 0;
        while let Some(r) = rows.next()? {
            let blob: Vec<u8> = r.get(1)?;
            f(r.get(0)?, from_blob(&blob));
            n += 1;
        }
        Ok(n)
    }

    pub fn count_embeddings(&self, model: &str) -> Result<u64, StorageError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM embeddings WHERE model = ?1",
            [model],
            |r| r.get(0),
        )?;
        Ok(n as u64)
    }

    /// Borra los vectores cuyo texto ya no usa ningún fragmento, y los de modelos que no
    /// son `keep`. Devuelve los hashes borrados de `keep` (para quitarlos del índice).
    pub fn prune_embeddings(&mut self, keep: &str) -> Result<Vec<i64>, StorageError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM embeddings WHERE model <> ?1", [keep])?;
        let orphans: Vec<i64> = tx
            .prepare(
                "SELECT e.text_hash FROM embeddings e WHERE e.model = ?1
                 AND NOT EXISTS (SELECT 1 FROM chunks c WHERE c.text_hash = e.text_hash)",
            )?
            .query_map([keep], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for hash in &orphans {
            tx.execute(
                "DELETE FROM embeddings WHERE model = ?1 AND text_hash = ?2",
                params![keep, hash],
            )?;
        }
        tx.commit()?;
        Ok(orphans)
    }

    /// El vector de un texto, si lo hay (tests y diagnóstico).
    pub fn embedding(&self, model: &str, hash: i64) -> Result<Option<Vec<f32>>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT vector FROM embeddings WHERE model = ?1 AND text_hash = ?2",
                params![model, hash],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .optional()?
            .map(|b| from_blob(&b)))
    }

    /// Rellena `chunks.text_hash` de los fragmentos que no lo tienen (los guardados antes
    /// de la v5). SQLite no sabe calcular blake3, así que lo hace Rust, en lotes.
    pub(crate) fn backfill_text_hashes(&mut self) -> Result<usize, StorageError> {
        let mut total = 0;
        loop {
            let batch: Vec<(i64, String)> = self
                .conn
                .prepare("SELECT id, text FROM chunks WHERE text_hash IS NULL LIMIT 5000")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?;
            if batch.is_empty() {
                return Ok(total);
            }
            let tx = self.conn.transaction()?;
            {
                let mut stmt = tx.prepare("UPDATE chunks SET text_hash = ?1 WHERE id = ?2")?;
                for (id, text) in &batch {
                    stmt.execute(params![text_hash(text), id])?;
                }
            }
            tx.commit()?;
            total += batch.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewChunk, NewFile};
    use std::path::{Path, PathBuf};

    fn add(db: &mut Database, root: i64, path: &str, ext: &str, mtime: i64, texts: &[&str]) {
        let chunks: Vec<NewChunk> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| NewChunk {
                ordinal: i as i64,
                start: 0,
                text: (*t).into(),
            })
            .collect();
        db.upsert_file(
            &NewFile {
                root_id: root,
                path: PathBuf::from(path),
                name: path.rsplit('\\').next().unwrap().into(),
                ext: ext.into(),
                size: 1,
                mtime,
                content_hash: Some("h".into()),
                error: None,
                error_kind: None,
            },
            &chunks,
        )
        .unwrap();
    }

    #[test]
    fn vectors_round_trip_in_half_precision() {
        let v = vec![0.5f32, -0.25, 0.125, 0.0];
        let back = from_blob(&to_blob(&v));
        assert_eq!(back, v); // estos valores son exactos en f16
        assert_eq!(to_blob(&v).len(), 8);
    }

    #[test]
    fn pending_texts_follow_the_policy_and_share_repeated_text() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        add(
            &mut db,
            root,
            r"C:\Uni\viejo.pdf",
            "pdf",
            1,
            &["normalización"],
        );
        add(
            &mut db,
            root,
            r"C:\Uni\nuevo.pdf",
            "pdf",
            9,
            &["transacciones"],
        );
        add(&mut db, root, r"C:\Uni\p1.sql", "sql", 50, &["select 1"]);
        // La misma práctica en otra carpeta: el mismo texto, un solo vector.
        add(
            &mut db,
            root,
            r"C:\Uni\copia\p1.sql",
            "sql",
            50,
            &["select 1"],
        );
        add(&mut db, root, r"C:\Uni\datos.xml", "xml", 99, &["<a/>"]);

        let policy = EmbedPolicy::standard();
        let pending: Vec<String> = db
            .pending_texts("m", &policy, 10)
            .unwrap()
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        // Documentos primero (lo más reciente antes), luego código; el xml no.
        assert_eq!(pending, ["transacciones", "normalización", "select 1"]);
        assert_eq!(db.embedding_progress("m", &policy).unwrap(), (0, 3));

        db.save_embeddings("m", &[(text_hash("select 1"), vec![1.0, 0.0])])
            .unwrap();
        assert_eq!(db.embedding_progress("m", &policy).unwrap(), (1, 3));
        assert_eq!(db.pending_texts("m", &policy, 10).unwrap().len(), 2);
        // Otro modelo empieza de cero.
        assert_eq!(db.embedding_progress("otro", &policy).unwrap(), (0, 3));
    }

    #[test]
    fn caps_limit_how_much_of_a_file_gets_vectors() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        let texts: Vec<String> = (0..150).map(|i| format!("línea {i}")).collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        add(&mut db, root, r"C:\Uni\volcado.txt", "txt", 1, &refs);
        let (_, total) = db
            .embedding_progress("m", &EmbedPolicy::standard())
            .unwrap();
        assert_eq!(total, 100);
    }

    #[test]
    fn chunks_saved_before_v5_get_their_hash() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        add(&mut db, root, r"C:\Uni\a.md", "md", 1, &["antes de la v5"]);
        db.conn
            .execute("UPDATE chunks SET text_hash = NULL", [])
            .unwrap();
        assert_eq!(db.backfill_text_hashes().unwrap(), 1);
        let hash: i64 = db
            .conn
            .query_row("SELECT text_hash FROM chunks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(hash, text_hash("antes de la v5"));
        assert_eq!(db.backfill_text_hashes().unwrap(), 0);
    }

    #[test]
    fn orphan_vectors_and_other_models_are_pruned() {
        let mut db = Database::open_in_memory().unwrap();
        let root = db.add_root(Path::new(r"C:\Uni")).unwrap().id;
        add(&mut db, root, r"C:\Uni\a.md", "md", 1, &["sigue aquí"]);
        db.save_embeddings(
            "m",
            &[
                (text_hash("sigue aquí"), vec![1.0]),
                (text_hash("ya no está"), vec![1.0]),
            ],
        )
        .unwrap();
        db.save_embeddings("viejo", &[(text_hash("sigue aquí"), vec![1.0])])
            .unwrap();
        assert_eq!(db.prune_embeddings("m").unwrap(), [text_hash("ya no está")]);
        assert_eq!(db.count_embeddings("m").unwrap(), 1);
        assert_eq!(db.count_embeddings("viejo").unwrap(), 0);
        let mut seen = Vec::new();
        db.for_each_embedding("m", |h, v| seen.push((h, v)))
            .unwrap();
        assert_eq!(seen, [(text_hash("sigue aquí"), vec![1.0])]);
    }
}
