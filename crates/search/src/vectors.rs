//! Índice de vectores (HNSW, con usearch) para la búsqueda por significado.
//!
//! Es **derivado**, como el de Tantivy: los vectores están en SQLite y este índice se
//! puede borrar y reconstruir en segundos sin recalcular nada (ADR 0009, punto 3). La
//! clave de cada vector es el hash de su texto (`sift_storage::text_hash`), el mismo que
//! lleva cada fragmento en Tantivy.

use std::path::Path;
use std::sync::RwLock;

use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

#[derive(Debug, thiserror::Error)]
#[error("índice de vectores: {0}")]
pub struct VectorError(String);

fn err(e: impl std::fmt::Display) -> VectorError {
    VectorError(e.to_string())
}

/// Parámetros del grafo HNSW (ADR 0009). Más conexiones y más exploración dan más
/// precisión a cambio de memoria y tiempo; estos son los habituales para decenas de miles
/// de vectores.
const CONNECTIVITY: usize = 16;
const EXPANSION_ADD: usize = 128;
const EXPANSION_SEARCH: usize = 64;

pub struct VectorIndex {
    index: RwLock<Index>,
    dims: usize,
}

fn options(dims: usize) -> IndexOptions {
    IndexOptions {
        dimensions: dims,
        metric: MetricKind::Cos,
        // La mitad de memoria que f32, sin pérdida apreciable con vectores normalizados.
        quantization: ScalarKind::F16,
        connectivity: CONNECTIVITY,
        expansion_add: EXPANSION_ADD,
        expansion_search: EXPANSION_SEARCH,
        multi: false,
    }
}

fn key(hash: i64) -> u64 {
    hash as u64
}

impl VectorIndex {
    pub fn new(dims: usize) -> Result<VectorIndex, VectorError> {
        let index = Index::new(&options(dims)).map_err(err)?;
        index.reserve(1024).map_err(err)?;
        Ok(VectorIndex {
            index: RwLock::new(index),
            dims,
        })
    }

    /// Carga un índice guardado. Si no está, no es de estas dimensiones o está dañado,
    /// devuelve `None`: hay que reconstruirlo desde SQLite.
    pub fn load(path: &Path, dims: usize) -> Option<VectorIndex> {
        if !path.is_file() {
            return None;
        }
        let index = Index::new(&options(dims)).ok()?;
        index.load(path.to_str()?).ok()?;
        (index.dimensions() == dims).then(|| VectorIndex {
            index: RwLock::new(index),
            dims,
        })
    }

    /// Guarda en un temporal y lo cambia de nombre: un cierre a mitad no deja un archivo
    /// a medias (y si lo dejara, `load` fallaría y se reconstruiría).
    pub fn save(&self, path: &Path) -> Result<(), VectorError> {
        let tmp = path.with_extension("tmp");
        let tmp_str = tmp.to_str().ok_or_else(|| err("ruta no válida"))?;
        self.read().save(tmp_str).map_err(err)?;
        std::fs::rename(&tmp, path).map_err(err)
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Index> {
        self.index.read().unwrap_or_else(|p| p.into_inner())
    }

    pub fn len(&self) -> usize {
        self.read().size()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn dims(&self) -> usize {
        self.dims
    }

    pub fn contains(&self, hash: i64) -> bool {
        self.read().contains(key(hash))
    }

    /// Añade (o sustituye) vectores. Agranda el índice si hace falta.
    pub fn add(&self, vectors: &[(i64, Vec<f32>)]) -> Result<(), VectorError> {
        let index = self.index.write().unwrap_or_else(|p| p.into_inner());
        let needed = index.size() + vectors.len();
        if needed > index.capacity() {
            index
                .reserve(needed.max(index.capacity() * 2))
                .map_err(err)?;
        }
        for (hash, vector) in vectors {
            if vector.len() != self.dims {
                return Err(err(format!(
                    "vector de {} dimensiones en un índice de {}",
                    vector.len(),
                    self.dims
                )));
            }
            if index.contains(key(*hash)) {
                index.remove(key(*hash)).map_err(err)?;
            }
            index.add(key(*hash), vector).map_err(err)?;
        }
        Ok(())
    }

    pub fn remove(&self, hashes: &[i64]) -> Result<(), VectorError> {
        let index = self.index.write().unwrap_or_else(|p| p.into_inner());
        for hash in hashes {
            index.remove(key(*hash)).map_err(err)?;
        }
        Ok(())
    }

    /// Los `k` vectores más parecidos a `query` (normalizado): (hash del texto,
    /// similitud coseno), el más parecido primero.
    pub fn nearest(&self, query: &[f32], k: usize) -> Result<Vec<(i64, f32)>, VectorError> {
        let index = self.read();
        if index.size() == 0 {
            return Ok(Vec::new());
        }
        let found = index.search(query, k).map_err(err)?;
        Ok(found
            .keys
            .into_iter()
            .zip(found.distances)
            // usearch da la distancia coseno (1 − similitud).
            .map(|(k, d)| (k as i64, 1.0 - d))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(v: &[f32]) -> Vec<f32> {
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter().map(|x| x / n).collect()
    }

    #[test]
    fn finds_the_nearest_vectors_and_survives_save_and_load() {
        let idx = VectorIndex::new(3).unwrap();
        idx.add(&[
            (-7, unit(&[1.0, 0.0, 0.0])),
            (8, unit(&[0.9, 0.1, 0.0])),
            (9, unit(&[0.0, 0.0, 1.0])),
        ])
        .unwrap();
        let near = idx.nearest(&unit(&[1.0, 0.05, 0.0]), 2).unwrap();
        let keys: Vec<i64> = near.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, [-7, 8]); // los hashes negativos también valen
        assert!(near[0].1 > 0.99 && near[0].1 <= 1.001);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.usearch");
        idx.save(&path).unwrap();
        let back = VectorIndex::load(&path, 3).unwrap();
        assert_eq!(back.len(), 3);
        assert!(VectorIndex::load(&path, 4).is_none()); // otras dimensiones: reconstruir
        assert!(VectorIndex::load(&dir.path().join("no.usearch"), 3).is_none());
    }

    #[test]
    fn grows_replaces_and_removes() {
        let idx = VectorIndex::new(2).unwrap();
        let many: Vec<(i64, Vec<f32>)> = (0..3000)
            .map(|i| (i, unit(&[1.0, i as f32 / 3000.0])))
            .collect();
        idx.add(&many).unwrap();
        assert_eq!(idx.len(), 3000);
        idx.add(&[(5, unit(&[0.0, 1.0]))]).unwrap(); // sustituye, no duplica
        assert_eq!(idx.len(), 3000);
        idx.remove(&[5]).unwrap();
        assert!(!idx.contains(5));
        assert!(idx.add(&[(1, vec![1.0, 0.0, 0.0])]).is_err());
    }
}
