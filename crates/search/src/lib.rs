//! Búsqueda: índice de texto en español (Tantivy), pipeline de recuperadores con fusión
//! RRF y fragmentos resaltados. Preparado para añadir la búsqueda por significado en la
//! Fase 4 (ver ADR 0006).

pub mod analyzer;
pub mod engine;
pub mod frecency;
pub mod fusion;
pub mod index;
pub mod intent;
pub mod paths;
pub mod query;
pub mod semantic;
pub mod snippet;
pub mod vectors;

pub use engine::{FileHit, Lexical, Retriever, SearchResults, search, search_with};
pub use frecency::Frecency;
pub use index::{Candidate, IndexError, TextIndex};
pub use intent::Intent;
pub use query::ParsedQuery;
pub use semantic::Semantic;
pub use snippet::Segment;
pub use vectors::{VectorError, VectorIndex};
