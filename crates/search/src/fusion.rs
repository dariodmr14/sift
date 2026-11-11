//! Reciprocal Rank Fusion (RRF): combina varias listas ordenadas en una.
//!
//! Cada elemento suma `1 / (k + posición)` por cada lista en la que aparece. Usa la
//! *posición*, no la puntuación: así se pueden mezclar BM25 (Tantivy) y similitud coseno
//! (Fase 4), que están en escalas distintas, sin calibrar nada. `k = 60` es el valor del
//! artículo original (Cormack et al., 2009) y suaviza el peso de los primeros puestos.

use std::collections::HashMap;
use std::hash::Hash;

pub const K: f64 = 60.0;

/// Fusiona listas ordenadas (mejor primero). Devuelve (elemento, puntuación RRF), mejor
/// primero; a igual puntuación, gana el que apareció antes en la primera lista.
pub fn rrf<T: Clone + Eq + Hash>(lists: &[Vec<T>]) -> Vec<(T, f64)> {
    let weighted: Vec<(f64, Vec<T>)> = lists.iter().map(|l| (1.0, l.clone())).collect();
    rrf_weighted(&weighted)
}

/// Como [`rrf`], pero cada lista pesa lo que se le diga: `peso / (k + posición)`. Así la
/// búsqueda por significado puede contar menos que la de palabras (ADR 0009, punto 4).
pub fn rrf_weighted<T: Clone + Eq + Hash>(lists: &[(f64, Vec<T>)]) -> Vec<(T, f64)> {
    let mut scores: HashMap<T, (f64, usize)> = HashMap::new();
    let mut order = 0;
    for (weight, list) in lists {
        for (rank, item) in list.iter().enumerate() {
            let entry = scores.entry(item.clone()).or_insert_with(|| {
                order += 1;
                (0.0, order)
            });
            entry.0 += weight / (K + rank as f64 + 1.0);
        }
    }
    let mut out: Vec<(T, f64, usize)> = scores.into_iter().map(|(t, (s, o))| (t, s, o)).collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.2.cmp(&b.2)));
    out.into_iter().map(|(t, s, _)| (t, s)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_list_keeps_its_order() {
        let fused: Vec<_> = rrf(&[vec!["a", "b", "c"]])
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(fused, vec!["a", "b", "c"]);
    }

    #[test]
    fn appearing_in_both_lists_beats_being_first_in_one() {
        // Léxico: a, b, c. Semántico: d, b, e. «b» está en las dos.
        let fused: Vec<_> = rrf(&[vec!["a", "b", "c"], vec!["d", "b", "e"]])
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(fused[0], "b");
        assert_eq!(fused.len(), 5);
    }

    #[test]
    fn scores_follow_the_formula() {
        let fused = rrf(&[vec!["a"], vec!["a"]]);
        assert!((fused[0].1 - 2.0 / 61.0).abs() < 1e-12);
    }

    #[test]
    fn empty_input() {
        assert!(rrf::<&str>(&[]).is_empty());
        assert!(rrf::<&str>(&[vec![], vec![]]).is_empty());
    }

    #[test]
    fn a_lighter_list_counts_less() {
        // «b» es primero en la lista ligera y segundo en la pesada; «a», al revés.
        let fused = rrf_weighted(&[(1.0, vec!["a", "b"]), (0.3, vec!["b", "a"])]);
        assert_eq!(fused[0].0, "a");
        // Con peso 0 la lista no cuenta nada.
        let only_first = rrf_weighted(&[(1.0, vec!["a"]), (0.0, vec!["z"])]);
        assert_eq!(only_first[1], ("z", 0.0));
    }
}
