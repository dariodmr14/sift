//! Indexado: recorrer carpetas (respetando los patrones a ignorar), extraer y trocear el
//! texto, y sincronizar SQLite y el índice de búsqueda de forma incremental.

pub mod chunk;
pub mod ignore;
pub mod sync;

pub use chunk::{Chunk, chunk_text};
pub use ignore::{IgnoreRules, PatternError};
pub use sync::{IndexSink, IndexedFile, SyncError, SyncReport, sync_root};
