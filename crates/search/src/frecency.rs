//! Frecencia: lo que abres a menudo y hace poco sube en los resultados.
//!
//! Cada apertura suma un punto que se va gastando con el tiempo (vida media de
//! [`HALF_LIFE_DAYS`] días): un apunte abierto cinco veces esta semana pesa más que uno
//! abierto cinco veces hace dos meses. La suma se convierte en un empujón acotado:
//!
//! ```text
//! puntuación final = puntuación de la fusión × (1 + PESO × s / (s + SATURACIÓN))
//! ```
//!
//! Dos reglas que la hacen segura (ADR 0010):
//! - **Solo reordena**: un archivo que no encaja con la búsqueda no aparece por mucho que
//!   lo abras.
//! - **Tiene techo**: como mucho multiplica por `1 + PESO`; no puede pasar por delante de
//!   un resultado mucho mejor.

use std::collections::HashMap;

/// Días para que una apertura valga la mitad. Dos semanas: lo que estudias este tema.
pub const HALF_LIFE_DAYS: f64 = 14.0;
/// Aperturas «recientes» para llegar a la mitad del empujón máximo.
const SATURATION: f64 = 3.0;
/// Empujón máximo: +25 %. Calibrado con una simulación sobre `eval/` (`calibrate_frecency`
/// en `tests/eval.rs`): con 0,25 abrir el archivo bueno sube el MRR de 0,66 a 0,69 y abrir
/// mucho otros cinco que salen en la búsqueda no quita recall; con 0,5 ya lo quita (0,75 →
/// 0,72).
const WEIGHT: f64 = 0.25;
/// Aperturas más viejas que esto no cuentan (valen menos del 0,3 %).
pub const MAX_AGE_DAYS: f64 = 120.0;

/// Puntos de frecencia por archivo. Vacía = no cambia nada.
#[derive(Debug, Clone, Default)]
pub struct Frecency {
    scores: HashMap<i64, f64>,
    weight: f64,
}

impl Frecency {
    /// A partir de las aperturas: (archivo, hace cuántos días).
    pub fn from_opens(opens: impl IntoIterator<Item = (i64, f64)>) -> Frecency {
        let mut scores: HashMap<i64, f64> = HashMap::new();
        for (file_id, age_days) in opens {
            // Un reloj atrasado no da aperturas «del futuro» que valgan más de 1.
            let age = age_days.max(0.0);
            if age > MAX_AGE_DAYS {
                continue;
            }
            *scores.entry(file_id).or_default() += 0.5f64.powf(age / HALF_LIFE_DAYS);
        }
        Frecency {
            scores,
            weight: WEIGHT,
        }
    }

    /// Otro empujón máximo (para calibrar con `eval/`).
    pub fn with_weight(mut self, weight: f64) -> Frecency {
        self.weight = weight;
        self
    }

    /// Por cuánto se multiplica la puntuación de un archivo (1 si nunca lo abriste).
    pub fn boost(&self, file_id: i64) -> f64 {
        match self.scores.get(&file_id) {
            Some(s) => 1.0 + self.weight * s / (s + SATURATION),
            None => 1.0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_opened_is_neutral() {
        let f = Frecency::from_opens([(1, 0.0)]);
        assert_eq!(f.boost(2), 1.0);
        assert!(Frecency::default().is_empty());
    }

    #[test]
    fn often_and_recent_beats_old() {
        let recent = Frecency::from_opens((0..5).map(|_| (1, 2.0)));
        let old = Frecency::from_opens((0..5).map(|_| (1, 60.0)));
        assert!(recent.boost(1) > old.boost(1));
        assert!(old.boost(1) > 1.0);
    }

    #[test]
    fn more_opens_help_but_with_a_ceiling() {
        let few = Frecency::from_opens([(1, 0.0)]);
        let many = Frecency::from_opens((0..1000).map(|_| (1, 0.0)));
        assert!(many.boost(1) > few.boost(1));
        assert!(many.boost(1) <= 1.0 + WEIGHT + 1e-9);
    }

    #[test]
    fn too_old_does_not_count() {
        let f = Frecency::from_opens([(1, MAX_AGE_DAYS + 1.0)]);
        assert_eq!(f.boost(1), 1.0);
    }
}
