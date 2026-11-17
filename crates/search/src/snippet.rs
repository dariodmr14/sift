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

use std::cell::RefCell;

use tantivy::tokenizer::TextAnalyzer;

use crate::analyzer::{fold_analyzer, spanish, tokens};
use crate::query::ParsedQuery;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
    let len = folded_word.chars().count();
    q.terms.iter().any(|t| {
        stem == t.stem || {
            let word_len = t.word.chars().count();
            let d = max_typos(word_len) as usize;
            // Si las longitudes ya se llevan más que las erratas permitidas, no hace falta
            // calcular la distancia (es lo caro, y casi ninguna palabra pasa de aquí).
            d > 0 && len.abs_diff(word_len) <= d && levenshtein(folded_word, &t.word) <= d
        }
    }) || q
        .prefix
        .as_deref()
        .is_some_and(|p| !p.is_empty() && folded_word.starts_with(p))
}

/// Resalta `text` y, si es más largo que `max_chars`, recorta una ventana alrededor de
/// la primera coincidencia (con «…» donde se corta).
pub fn highlight(text: &str, q: &ParsedQuery, max_chars: usize) -> Vec<Segment> {
    let hits = find_hits(text, q);
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

/// Fragmento ampliado para la vista previa: como [`highlight`], pero más largo y, si
/// `keep_lines` (código), conservando los saltos de línea y cortando por líneas: hasta
/// `max_lines` líneas empezando dos antes de la primera coincidencia.
pub fn preview(
    text: &str,
    q: &ParsedQuery,
    max_chars: usize,
    max_lines: usize,
    keep_lines: bool,
) -> Vec<Segment> {
    if !keep_lines {
        return highlight(text, q, max_chars);
    }
    let text = text.replace('\r', "");
    let hits = find_hits(&text, q);
    // Inicio de cada línea, en bytes.
    let mut line_starts = vec![0];
    line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let hit_line = hits
        .first()
        .map_or(0, |h| line_starts.partition_point(|&s| s <= h.0) - 1);
    let first_line = hit_line.saturating_sub(2);
    let start = line_starts[first_line];
    let last_line = first_line + max_lines.max(1);
    let mut cut_end = last_line < line_starts.len();
    let mut end = if cut_end {
        line_starts[last_line] - 1
    } else {
        text.len()
    };
    if let Some((i, _)) = text[start..end].char_indices().nth(max_chars) {
        end = start + i;
        cut_end = true;
    }

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
        push("…\n", false);
    }
    let mut cursor = start;
    for &(from, to) in hits.iter().filter(|(f, t)| *f >= start && *t <= end) {
        push(&text[cursor..from], false);
        push(&text[from..to], true);
        cursor = to;
    }
    push(&text[cursor..end], false);
    if cut_end {
        push("\n…", false);
    }
    segments
}

/// Posiciones (inicio, fin en bytes) de las palabras de `text` que encajan con la consulta.
fn find_hits(text: &str, q: &ParsedQuery) -> Vec<(usize, usize)> {
    ANALYZERS.with_borrow_mut(|(stemmer, folder)| {
        tokens(stemmer, text)
            .into_iter()
            .filter(|t| {
                let word = tokens(folder, &text[t.from..t.to]);
                let folded = word.first().map_or("", |w| w.text.as_str());
                matches(&t.text, folded, q)
            })
            .map(|t| (t.from, t.to))
            .collect()
    })
}

thread_local! {
    /// Los analizadores, uno por hilo y no uno por palabra: crearlos (con la lista de
    /// palabras vacías) costaba más que analizar el fragmento (medido con el benchmark).
    static ANALYZERS: RefCell<(TextAnalyzer, TextAnalyzer)> =
        RefCell::new((spanish(), fold_analyzer()));
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

    fn joined(s: &[Segment]) -> String {
        s.iter().map(|x| x.text.as_str()).collect()
    }

    fn numbered(n: usize) -> String {
        (0..n)
            .map(|i| format!("línea {i}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn preview_without_keep_lines_is_highlight() {
        let text = "uno\n\n  dos tres";
        let q = ParsedQuery::parse("dos ");
        assert_eq!(preview(text, &q, 500, 12, false), highlight(text, &q, 500));
    }

    #[test]
    fn preview_keeps_line_breaks_and_indentation() {
        let text = "fn main() {\n    let primaria = 1;\n}";
        let s = preview(text, &ParsedQuery::parse("primaria "), 500, 12, true);
        assert_eq!(joined(&s), text);
        assert_eq!(hits(&s), vec!["primaria"]);
    }

    #[test]
    fn preview_starts_two_lines_before_the_hit() {
        let text = format!("{}\nla primaria\nfin", numbered(10));
        let s = preview(&text, &ParsedQuery::parse("primaria "), 900, 12, true);
        assert_eq!(joined(&s), "…\nlínea 8\nlínea 9\nla primaria\nfin");
        assert_eq!(hits(&s), vec!["primaria"]);
    }

    #[test]
    fn preview_respects_max_lines() {
        let text = numbered(30);
        let s = preview(&text, &ParsedQuery::parse("zzz "), 900, 3, true);
        assert_eq!(joined(&s), "línea 0\nlínea 1\nlínea 2\n…");
    }

    #[test]
    fn preview_without_hits_starts_at_line_zero() {
        let s = preview("a\nb\nc", &ParsedQuery::parse("zzz "), 900, 12, true);
        assert_eq!(joined(&s), "a\nb\nc");
    }

    #[test]
    fn preview_strips_carriage_returns_and_cuts_by_chars() {
        let s = preview("uno\r\ndos\r\n", &ParsedQuery::parse("zzz "), 900, 12, true);
        assert_eq!(joined(&s), "uno\ndos\n");
        let long = preview("áéíóúáéíóú", &ParsedQuery::parse("zzz "), 4, 12, true);
        assert_eq!(joined(&long), "áéíó\n…");
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
