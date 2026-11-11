//! El motor de búsqueda: recuperadores → fusión RRF → agrupar por archivo → fragmento.
//!
//! Recuperadores: el léxico (Tantivy) y el semántico (vectores, Fase 4). Cada uno pesa
//! en la fusión según el tipo de búsqueda ([`Intent`]): en una búsqueda exacta («shell-v4.3»)
//! la semántica no participa. Ver ADR 0006 y 0009.

use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;

use crate::fusion::rrf_weighted;
use crate::index::{ChunkHit, IndexError, TextIndex};
use crate::intent::Intent;
use crate::query::ParsedQuery;
use crate::snippet::{Segment, highlight};

/// Cuántos fragmentos pide cada recuperador antes de fusionar y agrupar.
const CANDIDATES: usize = 100;
/// Longitud del fragmento resaltado que se muestra, en caracteres.
const SNIPPET_CHARS: usize = 160;

/// Un recuperador devuelve fragmentos ordenados, el mejor primero.
pub trait Retriever: Send + Sync {
    fn name(&self) -> &'static str;
    /// Cuánto pesa en la fusión para este tipo de búsqueda. 0 = no se le pregunta.
    fn weight(&self, _intent: Intent) -> f64 {
        1.0
    }
    /// ¿Encuentra por significado? (sus resultados se marcan si solo llegan por aquí)
    fn by_meaning(&self) -> bool {
        false
    }
    fn retrieve(&self, q: &ParsedQuery, k: usize) -> Result<Vec<ChunkHit>, IndexError>;
}

/// El recuperador léxico: palabras, prefijos y erratas (Tantivy).
pub struct Lexical<'a>(pub &'a TextIndex);

impl Retriever for Lexical<'_> {
    fn name(&self) -> &'static str {
        "léxico"
    }
    fn retrieve(&self, q: &ParsedQuery, k: usize) -> Result<Vec<ChunkHit>, IndexError> {
        self.0.search_chunks(q, k)
    }
}

/// Un archivo encontrado, con su mejor fragmento.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FileHit {
    /// Id de SQLite. En TypeScript es un `number` normal: nunca pasará de 2³¹.
    #[cfg_attr(feature = "specta", specta(type = i32))]
    pub file_id: i64,
    pub path: String,
    pub name: String,
    pub ext: String,
    /// Nombre con lo que encaja resaltado.
    pub name_segments: Vec<Segment>,
    /// Fragmento del contenido con lo que encaja resaltado (vacío si solo encajó el nombre).
    pub snippet: Vec<Segment>,
    pub score: f64,
    /// Encontrado solo por significado (no comparte palabras con la búsqueda): la UI lo
    /// marca para que se entienda por qué está ahí.
    pub by_meaning: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub hits: Vec<FileHit>,
    /// Tiempo de la búsqueda en Rust (sin la IPC), en milisegundos.
    pub took_ms: f64,
}

/// Busca con todos los recuperadores, fusiona, agrupa por archivo y prepara los fragmentos.
pub fn search(
    retrievers: &[&dyn Retriever],
    raw: &str,
    limit: usize,
) -> Result<SearchResults, IndexError> {
    let started = Instant::now();
    let q = ParsedQuery::parse(raw);
    if q.is_empty() {
        return Ok(SearchResults {
            hits: Vec::new(),
            took_ms: 0.0,
        });
    }

    let intent = Intent::of(raw);
    let mut lists = Vec::with_capacity(retrievers.len());
    let mut by_chunk: HashMap<(i64, i64), ChunkHit> = HashMap::new();
    // Archivos que encuentra algún recuperador por palabras.
    let mut by_words = std::collections::HashSet::new();
    for r in retrievers {
        let weight = r.weight(intent);
        if weight <= 0.0 {
            continue;
        }
        let hits = r.retrieve(&q, CANDIDATES)?;
        if !r.by_meaning() {
            by_words.extend(hits.iter().map(|h| h.file_id));
        }
        lists.push((
            weight,
            hits.iter()
                .map(|h| (h.file_id, h.chunk_id))
                .collect::<Vec<_>>(),
        ));
        for h in hits {
            by_chunk.entry((h.file_id, h.chunk_id)).or_insert(h);
        }
    }

    // Un resultado por archivo: su mejor fragmento (el primero tras fusionar).
    let mut seen = HashMap::new();
    let mut hits = Vec::new();
    for ((file_id, chunk_id), score) in rrf_weighted(&lists) {
        if seen.insert(file_id, ()).is_some() {
            continue;
        }
        let Some(h) = by_chunk.remove(&(file_id, chunk_id)) else {
            continue;
        };
        hits.push(FileHit {
            name_segments: highlight(&h.name, &q, 200),
            snippet: if h.body.is_empty() {
                Vec::new()
            } else {
                highlight(&h.body, &q, SNIPPET_CHARS)
            },
            by_meaning: !by_words.contains(&h.file_id),
            file_id: h.file_id,
            path: h.path,
            name: h.name,
            ext: h.ext,
            score,
        });
    }
    // Lo que nombras va primero: si todas las palabras están en el nombre de un archivo,
    // ningún resultado que solo llega por significado se le pone delante.
    let words: Vec<String> = q
        .terms
        .iter()
        .map(|t| t.word.clone())
        .chain(q.prefix.clone())
        .collect();
    if !words.is_empty() {
        let named = |h: &FileHit| {
            let name = crate::analyzer::fold(&h.name);
            words.iter().all(|w| name.contains(w.as_str()))
        };
        hits.sort_by_key(|h| !named(h)); // estable: dentro de cada grupo, el orden de la fusión
    }
    hits.truncate(limit);
    Ok(SearchResults {
        hits,
        took_ms: started.elapsed().as_secs_f64() * 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sift_indexer::{IndexSink, IndexedFile};

    fn index_with(files: &[(i64, &str, &[&str])]) -> TextIndex {
        let mut idx = TextIndex::in_memory().unwrap();
        let mut next_chunk = 1;
        for (id, name, chunks) in files {
            let chunks = chunks
                .iter()
                .map(|t| {
                    next_chunk += 1;
                    (next_chunk, t.to_string())
                })
                .collect();
            idx.upsert(&IndexedFile {
                file_id: *id,
                path: format!(r"C:\Uni\{name}"),
                name: name.to_string(),
                ext: name.rsplit('.').next().unwrap_or("").to_string(),
                chunks,
            })
            .unwrap();
        }
        idx.commit().unwrap();
        idx
    }

    fn names(idx: &TextIndex, q: &str) -> Vec<String> {
        search(&[&Lexical(idx)], q, 10)
            .unwrap()
            .hits
            .into_iter()
            .map(|h| h.name)
            .collect()
    }

    fn sample() -> TextIndex {
        index_with(&[
            (
                1,
                "tema4.md",
                &[
                    "La normalización elimina la redundancia.",
                    "La tercera forma normal (3FN) evita dependencias transitivas.",
                ],
            ),
            (2, "Tema 5 - Transacciones.pdf", &[]),
            (
                3,
                "practica2.sql",
                &["CREATE TABLE alumnos (id INT PRIMARY KEY);"],
            ),
            (
                4,
                "redes.md",
                &["El protocolo TCP garantiza la entrega ordenada."],
            ),
        ])
    }

    #[test]
    fn finds_words_inside_files() {
        assert_eq!(names(&sample(), "redundancia "), vec!["tema4.md"]);
        assert_eq!(names(&sample(), "protocolo tcp "), vec!["redes.md"]);
    }

    #[test]
    fn finds_by_file_name_even_without_content() {
        assert_eq!(
            names(&sample(), "transacciones "),
            vec!["Tema 5 - Transacciones.pdf"]
        );
    }

    #[test]
    fn spanish_roots_and_accents() {
        // «normalizar» y «normalizacion» comparten raíz con «normalización».
        assert_eq!(names(&sample(), "normalizar "), vec!["tema4.md"]);
        assert_eq!(names(&sample(), "NORMALIZACION "), vec!["tema4.md"]);
    }

    #[test]
    fn prefixes_while_typing() {
        assert_eq!(names(&sample(), "redun"), vec!["tema4.md"]);
        assert_eq!(names(&sample(), "alum"), vec!["practica2.sql"]);
    }

    #[test]
    fn tolerates_typos() {
        assert_eq!(names(&sample(), "redundansia "), vec!["tema4.md"]);
        assert_eq!(names(&sample(), "protocolo "), vec!["redes.md"]);
    }

    #[test]
    fn all_words_must_match() {
        assert!(names(&sample(), "redundancia tcp ").is_empty());
    }

    #[test]
    fn one_result_per_file_with_its_best_snippet() {
        let r = search(&[&Lexical(&sample())], "transitivas ", 10).unwrap();
        assert_eq!(r.hits.len(), 1);
        let hit: Vec<_> = r.hits[0]
            .snippet
            .iter()
            .filter(|s| s.hit)
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(hit, vec!["transitivas"]);
    }

    #[test]
    fn name_matches_rank_above_content_matches() {
        let idx = index_with(&[
            (1, "apuntes.md", &["Hablamos de redes en general."]),
            (2, "redes.md", &["Temario completo."]),
        ]);
        assert_eq!(names(&idx, "redes ")[0], "redes.md");
    }

    #[test]
    fn reindexing_a_file_replaces_it_and_removing_deletes_it() {
        let mut idx = sample();
        idx.upsert(&IndexedFile {
            file_id: 4,
            path: r"C:\Uni\redes.md".into(),
            name: "redes.md".into(),
            ext: "md".into(),
            chunks: vec![(99, "Ahora habla de UDP.".into())],
        })
        .unwrap();
        idx.commit().unwrap();
        assert!(names(&idx, "tcp ").is_empty());
        assert_eq!(names(&idx, "udp "), vec!["redes.md"]);
        idx.remove(&[4]).unwrap();
        idx.commit().unwrap();
        assert!(names(&idx, "udp ").is_empty());
    }

    #[test]
    fn empty_query_returns_nothing() {
        assert!(
            search(&[&Lexical(&sample())], "  ", 10)
                .unwrap()
                .hits
                .is_empty()
        );
    }

    #[test]
    fn index_on_disk_is_rebuilt_when_the_format_changes() {
        let dir = tempfile::tempdir().unwrap();
        let first = TextIndex::open(dir.path()).unwrap();
        assert!(first.needs_rebuild); // nuevo
        drop(first);
        assert!(!TextIndex::open(dir.path()).unwrap().needs_rebuild);
        std::fs::write(dir.path().join("sift-schema-version"), "0").unwrap();
        assert!(TextIndex::open(dir.path()).unwrap().needs_rebuild);
    }
}
