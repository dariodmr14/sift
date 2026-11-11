//! ¿Qué tipo de búsqueda es? Decide cuánto pesa la búsqueda por significado.
//!
//! El riesgo de la semántica es que **siempre devuelve algo**: para «shell-v4.3» traería
//! fragmentos «parecidos» que ensucian un resultado que las palabras clavan. Con reglas
//! simples, sin modelo (ADR 0009, punto 4):
//!
//! - Parece un nombre o un código → solo palabras.
//! - Muy corta (alguien tecleando) → solo palabras.
//! - Una palabra normal → híbrida, con la semántica a medio peso.
//! - Lenguaje natural (dos o más palabras) → híbrida.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Un nombre de archivo, un código, una ruta o algo entre comillas.
    Exact,
    /// Menos de 3 letras.
    Short,
    /// Una sola palabra normal.
    Word,
    /// Dos o más palabras.
    Natural,
}

impl Intent {
    pub fn of(raw: &str) -> Intent {
        let text = raw.trim();
        let letters = text.chars().filter(|c| c.is_alphanumeric()).count();
        if letters < 3 {
            return Intent::Short;
        }
        if looks_exact(text) {
            return Intent::Exact;
        }
        if text.split_whitespace().count() >= 2 {
            Intent::Natural
        } else {
            Intent::Word
        }
    }
}

/// Señales de que se busca algo concreto por su nombre y no una idea.
fn looks_exact(text: &str) -> bool {
    if text.starts_with('"') || text.contains(['/', '\\', '_']) {
        return true;
    }
    text.split_whitespace().any(|word| {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        let chars: Vec<char> = word.chars().collect();
        // Extensión de archivo: «tema4.pdf», «main.rs».
        let extension = word.rsplit_once('.').is_some_and(|(stem, ext)| {
            !stem.is_empty()
                && (1..=5).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
                && !ext.chars().all(|c| c.is_ascii_digit())
        });
        // Guion entre letras y números o con versión: «shell-v4», «P3-JS», «03-Schemas».
        let code_hyphen = word.contains('-') && chars.iter().any(char::is_ascii_digit);
        // Letras y números pegados: «UO306650», «p3», «3FN».
        let mixed = chars.windows(2).any(|w| {
            (w[0].is_alphabetic() && w[1].is_ascii_digit())
                || (w[0].is_ascii_digit() && w[1].is_alphabetic())
        });
        // Mayúsculas por dentro: «ActionListener», «PreparedStatement».
        let camel = chars
            .windows(2)
            .any(|w| w[0].is_lowercase() && w[1].is_uppercase());
        extension || code_hyphen || mixed || camel
    })
}

/// Cuánto pesa cada recuperador en la fusión, según el tipo de búsqueda. Las palabras
/// siempre pesan 1; la semántica, nada en búsquedas exactas.
pub fn semantic_weight(intent: Intent) -> f64 {
    match intent {
        Intent::Exact | Intent::Short => 0.0,
        Intent::Word => 0.5,
        Intent::Natural => 0.7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_codes_are_exact() {
        for q in [
            "Tablas Hash.pdf",
            "shell-v4.3",
            "P3-JS",
            "03-Schemas",
            "UO306650",
            "practica_2",
            "src/main.rs",
            "\"forma normal\"",
            "ActionListener",
        ] {
            assert_eq!(Intent::of(q), Intent::Exact, "{q}");
        }
    }

    #[test]
    fn ideas_are_natural_language() {
        for q in [
            "cómo evitar redundancia en tablas",
            "bases de datos",
            "tema 6 ficheros", // un número suelto no lo hace exacto
            "dos procesos esperando",
            "año 2024",
        ] {
            assert_eq!(Intent::of(q), Intent::Natural, "{q}");
        }
    }

    #[test]
    fn single_words_and_short_input() {
        assert_eq!(Intent::of("normalización"), Intent::Word);
        assert_eq!(Intent::of("tcp"), Intent::Word);
        assert_eq!(Intent::of("bd"), Intent::Short);
        assert_eq!(Intent::of(" a "), Intent::Short);
        assert_eq!(Intent::of("hola."), Intent::Word); // un punto final no es extensión
        assert_eq!(Intent::of("versión 2.0"), Intent::Natural);
    }

    #[test]
    fn exact_searches_do_not_use_meaning() {
        assert_eq!(semantic_weight(Intent::Exact), 0.0);
        assert!(semantic_weight(Intent::Natural) > semantic_weight(Intent::Word));
    }
}
