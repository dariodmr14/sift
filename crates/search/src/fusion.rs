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
    let mut scores: HashMap<T, (f64, usize)> = HashMap::new();
    let mut order = 0;
    for list in lists {
        for (rank, item) in list.iter().enumerate() {
            let entry = scores.entry(item.clone()).or_insert_with(|| {
                order += 1;
                (0.0, order)
            });
            entry.0 += 1.0 / (K + rank as f64 + 1.0);
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
}
