//! El motor de búsqueda: recuperadores → fusión RRF → agrupar por archivo → fragmento.
//!
//! Recuperadores: el léxico (Tantivy) y el semántico (vectores, Fase 4). Cada uno pesa
//! en la fusión según el tipo de búsqueda ([`Intent`]): en una búsqueda exacta («shell-v4.3»)
//! la semántica no participa. Ver ADR 0006 y 0009.

use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;

use crate::fusion::rrf_weighted;
use crate::index::{Candidate, IndexError, TextIndex};
use crate::intent::Intent;
use crate::query::ParsedQuery;
use crate::snippet::{Segment, highlight};

/// Cuántos fragmentos pide cada recuperador antes de fusionar y agrupar.
const CANDIDATES: usize = 100;
/// Fragmentos de un mismo archivo que suman a su puntuación (calibrado con `eval/`:
/// 1, 2, 3 y 5; ver ADR 0010).
const CHUNKS_PER_FILE: usize = 3;

/// De cuántos resultados por encima del límite se carga el texto: los justos para
/// reordenar por nombre sin cargar cien documentos (lo caro de una búsqueda).
const FETCH_MARGIN: usize = 2;
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
    /// Los fragmentos que encuentra, el mejor primero (solo ids: el texto se carga al
    /// final, y solo de lo que se muestra).
    fn retrieve(&self, q: &ParsedQuery, k: usize) -> Result<Vec<Candidate>, IndexError>;
}

/// El recuperador léxico: palabras, prefijos y erratas (Tantivy).
pub struct Lexical<'a>(pub &'a TextIndex);

impl Retriever for Lexical<'_> {
    fn name(&self) -> &'static str {
        "léxico"
    }
    fn retrieve(&self, q: &ParsedQuery, k: usize) -> Result<Vec<Candidate>, IndexError> {
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
/// `text` es el índice del que se cargan nombre, ruta y texto de lo que se muestra.
pub fn search(
    text: &TextIndex,
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
    // Archivos que encuentra algún recuperador por palabras.
    let mut by_words = std::collections::HashSet::new();
    // Los recuperadores, a la vez: la búsqueda tarda lo que el más lento, no la suma
    // (las palabras y el vector de la consulta tardan parecido).
    let active: Vec<(&dyn Retriever, f64)> = retrievers
        .iter()
        .map(|r| (*r, r.weight(intent)))
        .filter(|(_, w)| *w > 0.0)
        .collect();
    let found: Vec<Result<Vec<Candidate>, IndexError>> = std::thread::scope(|s| {
        let q = &q;
        let handles: Vec<_> = active
            .iter()
            .skip(1)
            .map(|(r, _)| s.spawn(move || r.retrieve(q, CANDIDATES)))
            .collect();
        let first = active.first().map(|(r, _)| r.retrieve(q, CANDIDATES));
        first
            .into_iter()
            .chain(handles.into_iter().map(|h| {
                // Si un recuperador entra en pánico, se propaga como si fuera en este hilo.
                h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))
            }))
            .collect()
    });
    for ((r, weight), hits) in active.into_iter().zip(found) {
        let hits = hits?;
        if !r.by_meaning() {
            by_words.extend(hits.iter().map(|h| h.file_id));
        }
        lists.push((
            weight,
            hits.iter()
                .map(|h| (h.file_id, h.chunk_id))
                .collect::<Vec<_>>(),
        ));
    }

    // Un resultado por archivo. Su puntuación suma la de sus mejores fragmentos (hasta
    // `CHUNKS_PER_FILE`): un apunte con varios fragmentos sobre el tema pesa más que otro
    // con uno solo. Se enseña su mejor fragmento. Solo se carga el texto de los primeros.
    let mut files: HashMap<i64, (usize, (i64, i64), f64, usize)> = HashMap::new();
    for (order, ((file_id, chunk_id), score)) in rrf_weighted(&lists).into_iter().enumerate() {
        let entry = files
            .entry(file_id)
            .or_insert((order, (file_id, chunk_id), 0.0, 0));
        if entry.3 < CHUNKS_PER_FILE {
            entry.2 += score;
            entry.3 += 1;
        }
    }
    let mut ranked: Vec<(usize, (i64, i64), f64)> = files
        .into_values()
        .map(|(order, key, score, _)| (order, key, score))
        .collect();
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    let best: Vec<((i64, i64), f64)> = ranked
        .into_iter()
        .take(limit * FETCH_MARGIN)
        .map(|(_, key, score)| (key, score))
        .collect();
    let keys: Vec<(i64, i64)> = best.iter().map(|(k, _)| *k).collect();
    let loaded: HashMap<(i64, i64), _> = text
        .fetch(&keys)?
        .into_iter()
        .map(|h| ((h.file_id, h.chunk_id), h))
        .collect();
    let mut loaded = loaded;
    let mut hits = Vec::with_capacity(best.len());
    for (key, score) in best {
        let Some(h) = loaded.remove(&key) else {
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
        search(idx, &[&Lexical(idx)], q, 10)
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
    fn a_phrase_finds_chunks_missing_one_word_but_after_full_matches() {
        let idx = index_with(&[
            (1, "a.md", &["Cómo evitar la redundancia: normalizar."]),
            (
                2,
                "b.md",
                &["La redundancia en las tablas se evita normalizando."],
            ),
            (3, "c.md", &["Las tablas hash evitan colisiones."]),
            (4, "d.md", &["Nada que ver."]),
        ]);
        // Ningún fragmento tiene las tres palabras; antes, nada. Ahora, los que tienen dos.
        let found = names(&idx, "evitar redundancia tablas ");
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(!found.contains(&"d.md".to_string()));
        // Dentro de las palabras, el fragmento que lo tiene todo va primero (luego, en la
        // fusión, un archivo con varios fragmentos parciales puede sumar más).
        let idx = index_with(&[
            (
                1,
                "parcial.md",
                &["evitar redundancia", "evitar redundancia otra vez"],
            ),
            (2, "completo.md", &["evitar redundancia en tablas"]),
        ]);
        let chunks = idx
            .search_chunks(&ParsedQuery::parse("evitar redundancia tablas "), 10)
            .unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].file_id, 2);
    }

    #[test]
    fn two_words_are_still_both_required() {
        // Con dos palabras, «falta una» sería «cualquiera de las dos»: no se relaja.
        assert!(names(&sample(), "redundancia tcp ").is_empty());
    }

    #[test]
    fn several_good_chunks_make_a_file_rank_higher() {
        // Mismo mejor fragmento, pero b.md tiene más fragmentos que encajan.
        let idx = index_with(&[
            (1, "a.md", &["redes", "otra cosa", "nada"]),
            (2, "b.md", &["redes", "más redes", "redes otra vez"]),
        ]);
        let r = search(&idx, &[&Lexical(&idx)], "redes ", 10).unwrap();
        assert_eq!(r.hits[0].name, "b.md");
        assert!(r.hits[0].score > r.hits[1].score);
    }

    /// Un recuperador que devuelve una lista fija (para probar la fusión en paralelo).
    struct Fixed(Vec<Candidate>, bool);

    impl Retriever for Fixed {
        fn name(&self) -> &'static str {
            "fijo"
        }
        fn by_meaning(&self) -> bool {
            self.1
        }
        fn retrieve(&self, _: &ParsedQuery, _: usize) -> Result<Vec<Candidate>, IndexError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn retrievers_run_together_and_all_count() {
        let idx = sample();
        // tema4.md (id 1): fragmentos 2 y 3; redes.md (id 4): fragmento 5.
        let meaning = Fixed(
            vec![Candidate {
                file_id: 4,
                chunk_id: 5,
                score: 0.9,
            }],
            true,
        );
        let r = search(&idx, &[&Lexical(&idx), &meaning], "redundancia ", 10).unwrap();
        let found: Vec<(&str, bool)> = r
            .hits
            .iter()
            .map(|h| (h.name.as_str(), h.by_meaning))
            .collect();
        assert_eq!(found, vec![("tema4.md", false), ("redes.md", true)]);
    }

    #[test]
    fn one_result_per_file_with_its_best_snippet() {
        let idx = sample();
        let r = search(&idx, &[&Lexical(&idx)], "transitivas ", 10).unwrap();
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
            search(&sample(), &[&Lexical(&sample())], "  ", 10)
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
