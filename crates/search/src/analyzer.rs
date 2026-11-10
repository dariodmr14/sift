//! Analizador de texto en español: el mismo para indexar y para consultar.
//!
//! tokenizar → quitar palabras larguísimas → minúsculas → quitar palabras vacías
//! («de», «la», «y»…) → quitar tildes → raíz en español.
//! Así «Normalización», «normalizacion» y «normalizar» acaban en la misma raíz.

use tantivy::tokenizer::{
    AsciiFoldingFilter, Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer,
    StopWordFilter, TextAnalyzer, TokenStream,
};

pub const NAME: &str = "sift_es";
/// Analizador sin raíz (minúsculas y sin tildes), para erratas y prefijos.
pub const FOLD_NAME: &str = "sift_fold";

pub fn spanish() -> TextAnalyzer {
    let builder = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter_dynamic(
            // Las palabras vacías se comparan antes de quitar tildes («está», «más»).
            StopWordFilter::new(Language::Spanish).expect("tantivy trae la lista en español"),
        )
        .filter_dynamic(AsciiFoldingFilter)
        .filter_dynamic(Stemmer::new(Language::Spanish));
    builder.build()
}

/// Una palabra analizada y dónde estaba en el texto original (en bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub from: usize,
    pub to: usize,
}

pub fn tokens(analyzer: &mut TextAnalyzer, text: &str) -> Vec<Token> {
    let mut stream = analyzer.token_stream(text);
    let mut out = Vec::new();
    while let Some(t) = stream.next() {
        out.push(Token {
            text: t.text.clone(),
            from: t.offset_from,
            to: t.offset_to,
        });
    }
    out
}

/// Minúsculas y sin tildes, pero **sin** sacar la raíz: las erratas y los prefijos se
/// comparan con la palabra entera («normali» todavía no es una palabra).
pub fn fold_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .build()
}

pub fn fold(text: &str) -> String {
    tokens(&mut fold_analyzer(), text)
        .into_iter()
        .map(|t| t.text)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stems(text: &str) -> Vec<String> {
        tokens(&mut spanish(), text)
            .into_iter()
            .map(|t| t.text)
            .collect()
    }

    #[test]
    fn same_root_with_or_without_accents_and_inflections() {
        let a = stems("Normalización");
        assert_eq!(a, stems("normalizacion"));
        assert_eq!(stems("normalizar"), stems("normalizado"));
    }

    #[test]
    fn removes_stopwords() {
        assert_eq!(stems("bases de datos"), stems("bases datos"));
    }

    #[test]
    fn keeps_offsets_into_the_original_text() {
        let text = "La Normalización";
        let t = &tokens(&mut spanish(), text)[0];
        assert_eq!(&text[t.from..t.to], "Normalización");
    }

    #[test]
    fn fold_lowercases_and_strips_accents_without_stemming() {
        assert_eq!(fold("Normaliz"), "normaliz");
        assert_eq!(fold("Árbol B+"), "arbol b");
    }
}
