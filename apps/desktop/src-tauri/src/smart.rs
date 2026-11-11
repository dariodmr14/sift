//! «Búsqueda inteligente»: lo que comparten el hilo de indexado (que calcula los vectores)
//! y la búsqueda (que los usa). Ver ADR 0009.
//!
//! Cuando el modelo está descargado y cargado, aquí hay:
//! - Un embedder **solo para las consultas**. El del indexado va aparte: si compartieran,
//!   una búsqueda esperaría a que acabara un lote de fragmentos (medio segundo).
//! - El índice de vectores, que el indexado va llenando mientras la búsqueda lo lee.
//!
//! Si no hay modelo (descargándose, desactivado, sin conexión), la búsqueda es solo por
//! palabras, como siempre.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use sift_embeddings::{ModelCard, OnnxEmbedder};
use sift_search::VectorIndex;

/// Hilos para calcular el vector de una consulta: es un instante y conviene que sea rápido.
pub const QUERY_THREADS: usize = 2;
/// Hilos para vectorizar en segundo plano (de 8 en un i7 de 4 núcleos): es el límite de
/// CPU de este trabajo, además del descanso y la pausa de la cola.
pub const BACKGROUND_THREADS: usize = 2;

/// Lo que necesita la búsqueda por significado, ya cargado.
#[derive(Clone)]
pub struct Ready {
    pub card: &'static ModelCard,
    pub query: Arc<OnnxEmbedder>,
    pub vectors: Arc<VectorIndex>,
}

#[derive(Default)]
pub struct SmartSearch {
    ready: RwLock<Option<Ready>>,
}

impl SmartSearch {
    /// Lo cargado, si lo hay (para buscar).
    pub fn ready(&self) -> Option<Ready> {
        self.ready.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn set(&self, ready: Option<Ready>) {
        *self.ready.write().unwrap_or_else(|p| p.into_inner()) = ready;
    }
}

/// Dónde se guarda el índice de vectores de un modelo (junto al de Tantivy).
pub fn vectors_path(app_data: &Path, card: &ModelCard) -> PathBuf {
    app_data
        .join("index")
        .join(format!("vectors-{}.usearch", card.id))
}
