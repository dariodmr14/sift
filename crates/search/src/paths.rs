//! Sugerencias de archivos y carpetas del índice mientras escribes una ruta (editor de
//! acciones de los Espacios, Fase 6). Se busca por **nombre**, no por contenido: al
//! elegir «qué abrir», lo que recuerdas es cómo se llama.
//!
//! Por orden:
//! 1. El nombre empieza por lo escrito («tema4» → `tema4.pdf`).
//! 2. Cada palabra empieza una palabra del nombre («apu bd» → `Apuntes BD.pdf`).
//! 3. Cada palabra aparece en algún sitio de la ruta («bd tema4» → `…\BD\tema4.pdf`).
//!
//! Dentro de cada nivel, lo que más abres (frecencia) y, después, la ruta más corta.

use std::collections::{BTreeSet, HashMap};

use crate::frecency::Frecency;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    File,
    Folder,
}

/// Minúsculas y sin tildes, carácter a carácter (sin crear un analizador por ruta:
/// esto se hace con miles de rutas en cada tecla).
pub fn simple_fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
}

/// Lo bien que encaja (3, 2, 1; 0 = no encaja). `query` y `path` ya plegados.
fn quality(query: &str, name: &str, path: &str) -> u8 {
    let q: Vec<&str> = words(query).collect();
    if q.is_empty() {
        return 0;
    }
    if name.starts_with(query.trim()) {
        return 3;
    }
    let name_words: Vec<&str> = words(name).collect();
    if q.iter()
        .all(|w| name_words.iter().any(|n| n.starts_with(w)))
    {
        return 2;
    }
    if q.iter().all(|w| path.contains(w)) {
        return 1;
    }
    0
}

/// Último trozo de una ruta de Windows (o con `/`).
fn last_part(path: &str) -> &str {
    path.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
}

/// Hasta `limit` rutas que encajan con `query`, la mejor primero. `files` son los
/// archivos del índice (id, ruta). Las carpetas salen de las rutas de los archivos.
pub fn suggest_paths(
    files: &[(i64, String)],
    query: &str,
    kind: PathKind,
    frecency: &Frecency,
    limit: usize,
) -> Vec<String> {
    let query = simple_fold(query);
    if words(&query).next().is_none() {
        return Vec::new();
    }
    // (ruta, frecencia): un archivo, su propia frecencia; una carpeta, la del archivo
    // que más abres dentro de ella.
    let candidates: Vec<(&str, f64)> = match kind {
        PathKind::File => files
            .iter()
            .map(|(id, p)| (p.as_str(), frecency.boost(*id)))
            .collect(),
        PathKind::Folder => {
            let mut folders: HashMap<&str, f64> = HashMap::new();
            for (id, path) in files {
                let boost = frecency.boost(*id);
                let mut current = path.as_str();
                // Todas las carpetas por encima del archivo, no solo la suya.
                while let Some(end) = current.rfind(['\\', '/']) {
                    current = &current[..end];
                    if current.is_empty() || current.ends_with(':') {
                        break;
                    }
                    let entry = folders.entry(current).or_insert(1.0);
                    *entry = entry.max(boost);
                }
            }
            folders.into_iter().collect()
        }
    };
    let mut scored: Vec<(u8, f64, &str)> = candidates
        .into_iter()
        .filter_map(|(path, boost)| {
            let folded = simple_fold(path);
            let q = quality(&query, last_part(&folded), &folded);
            (q > 0).then_some((q, boost, path))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.total_cmp(&a.1))
            .then(a.2.len().cmp(&b.2.len()))
            .then(a.2.cmp(b.2))
    });
    // Sin repetidos (la misma ruta no debería estar dos veces, pero por si acaso).
    let mut seen = BTreeSet::new();
    scored
        .into_iter()
        .filter(|(_, _, p)| seen.insert(*p))
        .take(limit)
        .map(|(_, _, p)| p.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<(i64, String)> {
        [
            r"C:\Uni\BD\Teoría\tema4.pdf",
            r"C:\Uni\BD\Apuntes BD.pdf",
            r"C:\Uni\Redes\tema4-redes.pdf",
            r"C:\Uni\BD\Prácticas\practica2.sql",
            r"C:\Uni\Redes\Prácticas\p1.pdf",
        ]
        .iter()
        .enumerate()
        .map(|(i, p)| (i as i64 + 1, p.to_string()))
        .collect()
    }

    fn suggest(q: &str, kind: PathKind) -> Vec<String> {
        suggest_paths(&files(), q, kind, &Frecency::default(), 10)
    }

    #[test]
    fn name_start_beats_word_start_beats_anywhere() {
        let found = suggest("tema4", PathKind::File);
        // Las dos empiezan por «tema4»: la más corta primero.
        assert_eq!(found[0], r"C:\Uni\BD\Teoría\tema4.pdf");
        assert_eq!(found[1], r"C:\Uni\Redes\tema4-redes.pdf");
        assert_eq!(
            suggest("apu bd", PathKind::File),
            vec![r"C:\Uni\BD\Apuntes BD.pdf"]
        );
        // «bd» y «tema4» solo juntas en la ruta entera.
        assert_eq!(
            suggest("bd tema4", PathKind::File),
            vec![r"C:\Uni\BD\Teoría\tema4.pdf"]
        );
    }

    #[test]
    fn accents_and_case_do_not_matter() {
        assert_eq!(
            suggest("PRACTICAS", PathKind::Folder),
            vec![r"C:\Uni\BD\Prácticas", r"C:\Uni\Redes\Prácticas"]
        );
        assert_eq!(
            suggest("teoria", PathKind::Folder),
            vec![r"C:\Uni\BD\Teoría"]
        );
    }

    #[test]
    fn folders_include_every_level_above_the_files() {
        let found = suggest("uni", PathKind::Folder);
        assert_eq!(found[0], r"C:\Uni");
        assert!(suggest("redes", PathKind::Folder).contains(&r"C:\Uni\Redes".to_string()));
    }

    #[test]
    fn what_you_open_goes_first_within_the_same_level() {
        // Las dos empiezan por «tema4»; la de Redes la abres mucho.
        let f = Frecency::from_opens((0..5).map(|_| (3, 1.0)));
        let found = suggest_paths(&files(), "tema4", PathKind::File, &f, 10);
        assert_eq!(found[0], r"C:\Uni\Redes\tema4-redes.pdf");
    }

    #[test]
    fn empty_query_or_no_match_gives_nothing() {
        assert!(suggest("  ", PathKind::File).is_empty());
        assert!(suggest("zzz", PathKind::Folder).is_empty());
    }
}
