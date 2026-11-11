//! Vectores de significado («embeddings») con un modelo local en ONNX Runtime.
//!
//! - [`ModelCard`]: la ficha de un modelo (de dónde se baja, con qué hash, cómo se usa).
//!   Cambiar de modelo es añadir una ficha (ADR 0009, punto 7).
//! - [`Embedder`]: convierte textos en vectores normalizados. [`OnnxEmbedder`] lo hace con
//!   `ort` (ONNX Runtime) y `tokenizers`; los tests pueden usar uno falso.
//!
//! Nada de esto usa la red: los archivos del modelo se descargan aparte (en la app), y
//! aquí se cargan de una carpeta.

mod card;
mod onnx;
mod pooling;

pub use card::{MODELS, ModelCard, ModelFile, Pooling, model};
pub use onnx::OnnxEmbedder;

/// Para qué es el texto: algunos modelos (e5) esperan un prefijo distinto en cada caso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Lo que escribes en la barra.
    Query,
    /// Un fragmento de un archivo.
    Document,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("falta el archivo del modelo: {0}")]
    MissingFile(String),
    #[error("no se pudo cargar el tokenizador: {0}")]
    Tokenizer(String),
    #[error("ONNX Runtime: {0}")]
    Runtime(String),
    #[error("el modelo devolvió algo inesperado: {0}")]
    Shape(String),
}

/// Convierte textos en vectores normalizados (longitud 1): el coseno entre dos es su
/// producto escalar.
pub trait Embedder: Send + Sync {
    /// Ficha del modelo (id, dimensiones…).
    fn card(&self) -> &'static ModelCard;
    fn embed(&self, texts: &[&str], purpose: Purpose) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// Producto escalar: el coseno entre dos vectores normalizados.
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
