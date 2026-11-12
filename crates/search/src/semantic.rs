//! El recuperador por significado: la consulta se convierte en un vector y se buscan los
//! fragmentos con el vector más parecido (ADR 0009).

use sift_embeddings::{Embedder, Purpose};

use crate::engine::Retriever;
use crate::index::{Candidate, IndexError, TextIndex};
use crate::intent::{Intent, semantic_weight};
use crate::query::ParsedQuery;
use crate::vectors::VectorIndex;

/// Vecinos que se piden al índice de vectores. Calibrado con `eval/` (40 o 100, junto con
/// la suma de fragmentos por archivo): con 100 entra más de un fragmento por apunte y la
/// suma lo nota (ADR 0010).
pub const SEMANTIC_CANDIDATES: usize = 100;

pub struct Semantic<'a> {
    pub vectors: &'a VectorIndex,
    /// De aquí salen el nombre, la ruta y el texto de cada fragmento encontrado.
    pub text: &'a TextIndex,
    pub embedder: &'a dyn Embedder,
    /// Por debajo de este parecido no entra, aunque sea lo más parecido que hay: la
    /// semántica siempre devuelve algo y eso es ruido (se calibra con `eval/`).
    pub min_similarity: f32,
}

impl Retriever for Semantic<'_> {
    fn name(&self) -> &'static str {
        "semántico"
    }

    fn weight(&self, intent: Intent) -> f64 {
        semantic_weight(intent)
    }

    fn by_meaning(&self) -> bool {
        true
    }

    fn retrieve(&self, q: &ParsedQuery, k: usize) -> Result<Vec<Candidate>, IndexError> {
        // Menos vecinos que fragmentos por palabras: más allá de los primeros, la semántica
        // aporta poco y cada uno cuesta buscar sus copias (ADR 0009, rendimiento).
        let k = k.min(SEMANTIC_CANDIDATES);
        if self.vectors.is_empty() || q.raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        let query = self
            .embedder
            .embed(&[q.raw.trim()], Purpose::Query)
            .map_err(|e| IndexError::Open(e.to_string()))?;
        let Some(query) = query.into_iter().next() else {
            return Ok(Vec::new());
        };
        let nearest: Vec<(i64, f32)> = self
            .vectors
            .nearest(&query, k)
            .map_err(|e| IndexError::Open(e.to_string()))?
            .into_iter()
            .filter(|(_, similarity)| *similarity >= self.min_similarity)
            .collect();
        self.text.chunks_by_hashes(&nearest, k, &q.exts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Lexical, search};
    use sift_embeddings::{EmbedError, ModelCard};
    use sift_indexer::{IndexSink, IndexedFile};

    /// Embedder de juguete con dos ideas: [bases de datos, redes].
    struct Toy;

    impl Embedder for Toy {
        fn card(&self) -> &'static ModelCard {
            sift_embeddings::MODELS[0]
        }
        fn embed(&self, texts: &[&str], _: Purpose) -> Result<Vec<Vec<f32>>, EmbedError> {
            Ok(texts
                .iter()
                .map(|t| {
                    let t = t.to_lowercase();
                    if ["normaliz", "repetid", "tablas"]
                        .iter()
                        .any(|w| t.contains(w))
                    {
                        vec![1.0, 0.0]
                    } else if ["tcp", "paquetes"].iter().any(|w| t.contains(w)) {
                        vec![0.0, 1.0]
                    } else {
                        vec![-0.6, -0.8] // nada que ver con ninguna de las dos
                    }
                })
                .collect())
        }
    }

    fn setup() -> (TextIndex, VectorIndex) {
        let mut text = TextIndex::in_memory().unwrap();
        let files: [(i64, &str, &[&str]); 3] = [
            (1, "tema4.md", &["La normalización elimina la redundancia."]),
            (
                2,
                "redes.md",
                &["El protocolo TCP garantiza la entrega ordenada."],
            ),
            (3, "shell-v4.3.pdf", &[]),
        ];
        let vectors = VectorIndex::new(2).unwrap();
        for (id, name, chunks) in files {
            let chunks: Vec<(i64, String)> = chunks
                .iter()
                .enumerate()
                .map(|(i, t)| (id * 10 + i as i64, t.to_string()))
                .collect();
            for (_, t) in &chunks {
                let v = Toy.embed(&[t], Purpose::Document).unwrap().remove(0);
                vectors.add(&[(sift_storage::text_hash(t), v)]).unwrap();
            }
            text.upsert(&IndexedFile {
                file_id: id,
                path: format!(r"C:\Uni\{name}"),
                name: name.into(),
                ext: name.rsplit('.').next().unwrap().into(),
                chunks,
            })
            .unwrap();
        }
        text.commit().unwrap();
        (text, vectors)
    }

    fn found(text: &TextIndex, vectors: &VectorIndex, q: &str) -> Vec<(String, bool)> {
        let semantic = Semantic {
            vectors,
            text,
            embedder: &Toy,
            min_similarity: 0.5,
        };
        search(text, &[&Lexical(text), &semantic], q, 10)
            .unwrap()
            .hits
            .into_iter()
            .map(|h| (h.name, h.by_meaning))
            .collect()
    }

    #[test]
    fn finds_by_meaning_without_shared_words_and_says_so() {
        let (text, vectors) = setup();
        assert_eq!(
            found(&text, &vectors, "cómo evitar datos repetidos"),
            [("tema4.md".to_string(), true)]
        );
    }

    #[test]
    fn a_word_match_is_not_marked_as_by_meaning() {
        let (text, vectors) = setup();
        assert_eq!(
            found(&text, &vectors, "normalización redundancia"),
            [("tema4.md".to_string(), false)]
        );
    }

    #[test]
    fn exact_searches_get_no_semantic_noise() {
        let (text, vectors) = setup();
        assert_eq!(
            found(&text, &vectors, "shell-v4.3"),
            [("shell-v4.3.pdf".to_string(), false)]
        );
    }

    #[test]
    fn nothing_similar_enough_means_nothing_by_meaning() {
        let (text, vectors) = setup();
        assert!(found(&text, &vectors, "receta de cocina casera").is_empty());
    }
}
