//! Fragmento con las palabras encontradas resaltadas.
//!
//! No se usa el generador de Tantivy porque solo resalta términos exactos: lo encontrado
//! por prefijo («normali…») o con una errata saldría sin resaltar. Aquí el texto pasa por
//! el mismo analizador que la consulta y se marcan las palabras que encajan igual que en
//! la búsqueda (exacta, prefijo o dentro de la distancia de erratas).
//!
//! Se devuelven trozos `{texto, resaltado}` y no posiciones: Rust cuenta en bytes y
//! JavaScript en unidades UTF-16, y con tildes las posiciones no coincidirían.

use serde::Serialize;

use crate::analyzer::{fold, spanish, tokens};
use crate::query::ParsedQuery;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Segment {
    pub text: String,
    pub hit: bool,
}

/// Erratas permitidas según la longitud de la palabra (la misma regla al buscar).
pub fn max_typos(len: usize) -> u8 {
    match len {
        0..=4 => 0,
        5..=8 => 1,
        _ => 2,
    }
}

/// ¿Encaja una palabra del texto con la consulta? La misma regla que la búsqueda: raíz
/// exacta, errata sobre la palabra entera, o prefijo de la última palabra.
fn matches(stem: &str, folded_word: &str, q: &ParsedQuery) -> bool {
    q.terms.iter().any(|t| {
        stem == t.stem || {
            let d = max_typos(t.word.chars().count());
            d > 0 && levenshtein(folded_word, &t.word) <= d as usize
        }
    }) || q
        .prefix
        .as_deref()
        .is_some_and(|p| !p.is_empty() && folded_word.starts_with(p))
}

/// Resalta `text` y, si es más largo que `max_chars`, recorta una ventana alrededor de
/// la primera coincidencia (con «…» donde se corta).
pub fn highlight(text: &str, q: &ParsedQuery, max_chars: usize) -> Vec<Segment> {
    let hits: Vec<(usize, usize)> = tokens(&mut spanish(), text)
        .into_iter()
        .filter(|t| matches(&t.text, &fold(&text[t.from..t.to]), q))
        .map(|t| (t.from, t.to))
        .collect();

    let (start, end) = window(text, hits.first().map(|h| h.0), max_chars);
    let mut segments = Vec::new();
    let mut push = |s: &str, hit: bool| {
        if !s.is_empty() {
            segments.push(Segment {
                text: s.to_owned(),
                hit,
            });
        }
    };
    if start > 0 {
        push("…", false);
    }
    let mut cursor = start;
    for &(from, to) in hits.iter().filter(|(f, t)| *f >= start && *t <= end) {
        push(&collapse(&text[cursor..from]), false);
        push(&text[from..to], true);
        cursor = to;
    }
    push(&collapse(&text[cursor..end]), false);
    if end < text.len() {
        push("…", false);
    }
    segments
}

/// Ventana de `max_chars` caracteres que empieza un poco antes de `first_hit` (en bytes),
/// ajustada a espacios para no cortar palabras. Devuelve (inicio, fin) en bytes.
fn window(text: &str, first_hit: Option<usize>, max_chars: usize) -> (usize, usize) {
    const CONTEXT: usize = 40;
    let char_starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    if char_starts.len() <= max_chars {
        return (0, text.len());
    }
    let hit_char = first_hit.map_or(0, |b| char_starts.partition_point(|&i| i < b));
    let mut s = hit_char.saturating_sub(CONTEXT);
    let mut e = (s + max_chars).min(char_starts.len());
    if e == char_starts.len() {
        s = e.saturating_sub(max_chars);
    }
    let byte = |c: usize| char_starts.get(c).copied().unwrap_or(text.len());
    // Empezar tras un espacio y acabar antes de uno, si hay alguno cerca.
    while s > 0 && s < hit_char && !text[byte(s - 1)..byte(s)].chars().all(char::is_whitespace) {
        s += 1;
    }
    while e < char_starts.len()
        && e > s + max_chars / 2
        && !text[byte(e)..byte(e + 1)].chars().all(char::is_whitespace)
    {
        e -= 1;
    }
    (byte(s), byte(e))
}

/// Saltos de línea y espacios repetidos → un espacio: el fragmento se lee en una línea.
fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(segments: &[Segment]) -> Vec<&str> {
        segments
            .iter()
            .filter(|s| s.hit)
            .map(|s| s.text.as_str())
            .collect()
    }

    #[test]
    fn highlights_inflections_accents_and_prefixes() {
        let text = "La Normalización evita la redundancia. Normalizar tablas.";
        assert_eq!(
            hits(&highlight(text, &ParsedQuery::parse("normalizacion "), 500)),
            vec!["Normalización", "Normalizar"]
        );
        assert_eq!(
            hits(&highlight(text, &ParsedQuery::parse("redun"), 500)),
            vec!["redundancia"]
        );
    }

    #[test]
    fn highlights_typos_like_the_search_does() {
        let text = "Transacciones y concurrencia";
        assert_eq!(
            hits(&highlight(text, &ParsedQuery::parse("transaciones "), 500)),
            vec!["Transacciones"]
        );
    }

    #[test]
    fn segments_rebuild_the_text() {
        let text = "Tema 4: normalización";
        let joined: String = highlight(text, &ParsedQuery::parse("tema "), 500)
            .into_iter()
            .map(|s| s.text)
            .collect();
        assert_eq!(joined, text);
    }

    #[test]
    fn long_texts_are_cut_around_the_first_hit() {
        let text = format!(
            "{} la clave primaria identifica cada fila {}",
            "relleno ".repeat(100),
            "más ".repeat(100)
        );
        let s = highlight(&text, &ParsedQuery::parse("primaria "), 120);
        let joined: String = s.iter().map(|x| x.text.as_str()).collect();
        assert!(joined.starts_with('…') && joined.ends_with('…'));
        assert!(joined.chars().count() <= 124);
        assert_eq!(hits(&s), vec!["primaria"]);
    }

    #[test]
    fn collapses_line_breaks() {
        let s = highlight("línea uno\n\n  línea dos", &ParsedQuery::parse("dos "), 500);
        let joined: String = s.into_iter().map(|x| x.text).collect();
        assert_eq!(joined, "línea uno línea dos");
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("transaccion", "transacion"), 1);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(max_typos(4), 0);
        assert_eq!(max_typos(5), 1);
        assert_eq!(max_typos(9), 2);
    }
}
