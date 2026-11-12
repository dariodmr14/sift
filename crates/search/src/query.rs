//! Una consulta ya analizada. Cada palabra tiene dos formas:
//! - `stem`: su raíz («normalizacion» → «normaliz»), para encontrar otras formas de la
//!   misma palabra (normalizar, normalizado…).
//! - `word`: en minúsculas y sin tildes, pero entera. Las erratas y los prefijos se
//!   comparan con esta: sobre raíces no funcionan, porque una letra mal puede cambiar qué
//!   sufijo se quita («redundancia» → «redund», pero «redundansia» → «redundansi»).
//!
//! Es la entrada de todos los recuperadores (léxico hoy, semántico en la Fase 4).

use crate::analyzer::{fold_analyzer, spanish, tokens};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTerm {
    pub stem: String,
    pub word: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    /// Texto tal cual (el recuperador semántico de la Fase 4 lo usará entero).
    pub raw: String,
    /// Palabras completas (sin palabras vacías como «de» o «la»).
    pub terms: Vec<QueryTerm>,
    /// Última palabra a medio escribir (sin tildes), para buscar por prefijo. `None` si la
    /// consulta acaba en espacio: entonces la última palabra ya está completa.
    pub prefix: Option<String>,
    /// Solo archivos con estas extensiones (un filtro rápido: «pdf …»). Vacío = todos.
    pub exts: Vec<String>,
}

impl ParsedQuery {
    pub fn parse(raw: &str) -> ParsedQuery {
        let mut stemmer = spanish();
        let words: Vec<String> = tokens(&mut fold_analyzer(), raw)
            .into_iter()
            .map(|t| t.text)
            .collect();
        let writing_last_word = !raw.ends_with(char::is_whitespace);
        let (complete, prefix) = match (writing_last_word, words.split_last()) {
            (true, Some((last, rest))) => (rest.to_vec(), Some(last.clone())),
            _ => (words, None),
        };
        let terms = complete
            .into_iter()
            .filter_map(|word| {
                // Sin raíz = palabra vacía («de», «la»…): no cuenta.
                let stem = tokens(&mut stemmer, &word).into_iter().next()?.text;
                Some(QueryTerm { stem, word })
            })
            .collect();
        ParsedQuery {
            raw: raw.to_owned(),
            terms,
            prefix,
            exts: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty() && self.prefix.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_word_being_typed_is_a_prefix() {
        let q = ParsedQuery::parse("Tema Normali");
        assert_eq!(q.terms.len(), 1);
        assert_eq!(q.terms[0].word, "tema");
        assert_eq!(q.prefix.as_deref(), Some("normali"));
    }

    #[test]
    fn trailing_space_means_all_words_are_complete() {
        let q = ParsedQuery::parse("tema normalización ");
        assert_eq!(q.terms.len(), 2);
        assert_eq!(q.terms[1].word, "normalizacion");
        assert_ne!(q.terms[1].stem, q.terms[1].word); // la raíz es más corta
        assert_eq!(q.prefix, None);
    }

    #[test]
    fn stopwords_do_not_count() {
        assert_eq!(ParsedQuery::parse("bases de datos ").terms.len(), 2);
    }

    #[test]
    fn a_stopword_being_typed_is_still_a_prefix() {
        // «de» es palabra vacía, pero mientras se escribe puede ser «del…», «derecho»…
        let q = ParsedQuery::parse("de");
        assert!(q.terms.is_empty());
        assert_eq!(q.prefix.as_deref(), Some("de"));
    }

    #[test]
    fn empty_queries() {
        assert!(ParsedQuery::parse("   ").is_empty());
        assert!(ParsedQuery::parse("").is_empty());
    }
}
