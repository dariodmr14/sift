//! SQLite: la fuente de verdad de Sift (ver ADR 0001 y 0004). Esquema, migraciones y
//! repositorios. Los índices de búsqueda (Fase 2) se podrán reconstruir desde aquí.

mod db;
mod index;
mod spaces;
mod vectors;

pub use db::Database;
pub use index::{ChunkText, FileRow, FileState, NewChunk, NewFile, Root, StoredChunk, UnreadGroup};
pub use vectors::{EmbedPolicy, text_hash};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("error de la base de datos: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("datos guardados ilegibles: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(#[from] sift_spaces::ValidationError),
    #[error("no existe el Espacio {0}")]
    NotFound(i64),
    #[error("el nuevo orden no coincide con los Espacios guardados")]
    ReorderMismatch,
    #[error("la base de datos es de una versión más nueva de Sift (esquema v{0})")]
    NewerSchema(u32),
    #[error("{0}")]
    Conflict(String),
    #[error("no se pudo crear la carpeta de datos: {0}")]
    Io(String),
}
