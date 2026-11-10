//! Indexado: recorrer carpetas (respetando los patrones a ignorar), extraer y trocear el
//! texto, y sincronizar SQLite y el índice de búsqueda de forma incremental.

pub mod chunk;
pub mod ignore;
pub mod sync;

pub use chunk::{Chunk, chunk_text};
pub use ignore::{IgnoreRules, PatternError};
pub use sync::{
    IndexSink, IndexedFile, RootPlan, SyncError, SyncReport, apply_plan, scan_root, sync_root,
};
