//! Búsqueda: índice de texto en español (Tantivy), pipeline de recuperadores con fusión
//! RRF y fragmentos resaltados. Preparado para añadir la búsqueda por significado en la
//! Fase 4 (ver ADR 0006).

pub mod analyzer;
pub mod engine;
pub mod fusion;
pub mod index;
pub mod query;
pub mod snippet;

pub use engine::{FileHit, Lexical, Retriever, SearchResults, search};
pub use index::{IndexError, TextIndex};
pub use query::ParsedQuery;
pub use snippet::Segment;
