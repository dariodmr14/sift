//! Indexado: recorrer carpetas (respetando los patrones a ignorar), extraer y trocear el
//! texto, y sincronizar SQLite y el índice de búsqueda de forma incremental.

pub mod chunk;
pub mod ignore;
pub mod sync;
pub mod throttle;

pub use chunk::{Chunk, chunk_text};
pub use ignore::{IgnoreRules, PatternError};
pub use sync::{
    Context, IndexSink, IndexedFile, PlanRun, RootPlan, SyncError, SyncReport, apply_plan,
    is_generated_dir, recover, scan_root, sync_paths, sync_root,
};
pub use throttle::{LoadSampler, NoLoad, Pace, Throttle, ThrottleConfig};
