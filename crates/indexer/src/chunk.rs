//! Troceado de texto en fragmentos con solapamiento (ver ADR 0006).
//!
//! ~1.200 caracteres por fragmento: cabe con margen en los 512 tokens de
//! `multilingual-e5-small` (Fase 4) y da fragmentos resaltados con contexto. Se corta
//! preferiblemente en un párrafo, si no en una línea, una frase o un espacio, para no
//! partir palabras. Los fragmentos se solapan para que una idea en la frontera entre dos
//! no se pierda.

/// Un fragmento: dónde empieza (en caracteres) y su texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub start: usize,
    pub text: String,
}

pub const TARGET: usize = 1_200;
pub const OVERLAP: usize = 150;

pub fn chunk_text(text: &str) -> Vec<Chunk> {
    chunk_with(text, TARGET, OVERLAP)
}

pub fn chunk_with(text: &str, target: usize, overlap: usize) -> Vec<Chunk> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < n {
        let hard_end = (start + target).min(n);
        let end = if hard_end == n {
            n
        } else {
            best_cut(&chars, start, hard_end, target)
        };
        let piece: String = chars[start..end].iter().collect();
        if !piece.trim().is_empty() {
            chunks.push(Chunk {
                start,
                text: piece.trim().to_owned(),
            });
        }
        if end == n {
            break;
        }
        // El siguiente empieza `overlap` antes, en el principio de una palabra, y siempre
        // avanza (si no, un texto sin espacios entraría en bucle).
        let mut next = end.saturating_sub(overlap).max(start + 1);
        while next < end && !chars[next - 1].is_whitespace() {
            next += 1;
        }
        start = next;
    }
    chunks
}

/// Dónde cortar entre `start` y `hard_end`: el último salto de párrafo, de línea, fin de
/// frase o espacio que no deje el fragmento demasiado corto (< 60 % del objetivo).
fn best_cut(chars: &[char], start: usize, hard_end: usize, target: usize) -> usize {
    let min = start + target * 6 / 10;
    let window = min..hard_end;
    let last = |pred: &dyn Fn(usize) -> bool| window.clone().rev().find(|&i| pred(i));
    last(&|i| chars[i] == '\n' && i > 0 && chars[i - 1] == '\n')
        .or_else(|| last(&|i| chars[i] == '\n'))
        .or_else(|| {
            last(&|i| matches!(chars[i - 1], '.' | '?' | '!' | ';') && chars[i].is_whitespace())
        })
        .or_else(|| last(&|i| chars[i].is_whitespace()))
        .map_or(hard_end, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_one_chunk() {
        let c = chunk_text("# Tema 4\nNormalización: 1FN, 2FN y 3FN.");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].start, 0);
    }

    #[test]
    fn empty_text_has_no_chunks() {
        assert!(chunk_text("   \n\n ").is_empty());
    }

    #[test]
    fn long_text_is_split_within_target_with_overlap() {
        let para = "La normalización evita la redundancia en las tablas. ".repeat(20);
        let text = format!("{para}\n\n{para}\n\n{para}");
        let chunks = chunk_with(&text, 300, 50);
        assert!(chunks.len() > 3);
        for c in &chunks {
            assert!(c.text.chars().count() <= 300, "fragmento demasiado largo");
        }
        // Solapamiento: cada fragmento empieza antes de donde acabó el anterior.
        for w in chunks.windows(2) {
            let prev_end = w[0].start + w[0].text.chars().count();
            assert!(w[1].start < prev_end, "sin solapamiento");
            assert!(w[1].start > w[0].start, "no avanza");
        }
        // El final del texto está en el último fragmento.
        assert!(chunks.last().unwrap().text.ends_with("tablas."));
    }

    #[test]
    fn does_not_split_words() {
        let text = "palabra ".repeat(100);
        for c in chunk_with(&text, 50, 10) {
            assert!(
                c.text.split_whitespace().all(|w| w == "palabra"),
                "{:?}",
                c.text
            );
        }
    }

    #[test]
    fn handles_text_without_spaces_and_unicode() {
        let text = "ñ".repeat(250);
        let chunks = chunk_with(&text, 100, 20);
        assert!(chunks.len() >= 3);
        assert!(chunks.iter().all(|c| c.text.chars().all(|ch| ch == 'ñ')));
    }

    #[test]
    fn prefers_paragraph_breaks() {
        let a = "a ".repeat(40);
        let text = format!("{a}\n\n{a}");
        let chunks = chunk_with(&text, 100, 10);
        assert!(
            chunks[0].text.ends_with('a'),
            "cortó a mitad: {:?}",
            chunks[0].text
        );
        assert_eq!(chunks[0].text.chars().filter(|c| *c == 'a').count(), 40);
    }
}
