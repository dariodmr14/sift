//! Que el indexado en segundo plano **no se note**.
//!
//! Dos frenos, que se consultan entre archivo y archivo:
//!
//! 1. **Límite de CPU.** Tras leer un archivo, se descansa en proporción a lo que costó.
//!    Con un límite del 50 %, 30 ms de trabajo piden 30 ms de descanso: el indexado nunca
//!    ocupa más de medio núcleo.
//! 2. **Pausa si el PC está ocupado.** Si el uso de CPU de todo el sistema pasa de un
//!    umbral (estás compilando, jugando, en una videollamada…), el indexado se para y
//!    no vuelve hasta que baja de otro umbral más bajo. Los dos umbrales distintos
//!    (histéresis) evitan que se encienda y apague sin parar cuando la carga ronda el
//!    límite.
//!
//! Medir la carga depende del sistema (en Windows, `GetSystemTimes`), así que aquí solo
//! está la decisión, detrás del trait [`LoadSampler`]: se puede probar sin un PC ocupado.

use std::time::Duration;

/// Mide cuánta CPU está usando el sistema entero.
pub trait LoadSampler {
    /// Porcentaje (0–100) de CPU ocupada desde la última llamada, o `None` si no se sabe.
    fn busy_percent(&mut self) -> Option<u8>;
}

/// Para cuando no se quiere (o no se puede) medir: nunca hay carga.
pub struct NoLoad;

impl LoadSampler for NoLoad {
    fn busy_percent(&mut self) -> Option<u8> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrottleConfig {
    /// Fracción de un núcleo que puede ocupar el indexado (0–1).
    pub max_duty: f64,
    /// Con esta carga del sistema o más, el indexado se pausa.
    pub pause_above: u8,
    /// Pausado, no vuelve hasta que la carga baja de aquí.
    pub resume_below: u8,
    /// Descanso máximo tras un archivo (uno enorme no debe dormir medio minuto).
    pub max_rest: Duration,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        ThrottleConfig {
            max_duty: 0.5,
            pause_above: 75,
            resume_below: 55,
            max_rest: Duration::from_millis(500),
        }
    }
}

/// Qué hacer antes del siguiente archivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// Seguir, tras descansar esto.
    Go { rest: Duration },
    /// El PC está ocupado: esperar y volver a preguntar.
    Pause,
}

pub struct Throttle<S> {
    sampler: S,
    config: ThrottleConfig,
    paused: bool,
}

impl<S: LoadSampler> Throttle<S> {
    pub fn new(sampler: S, config: ThrottleConfig) -> Throttle<S> {
        Throttle {
            sampler,
            config,
            paused: false,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Se llama tras cada archivo, con lo que se tardó en leerlo (`Duration::ZERO` al
    /// volver a preguntar durante una pausa).
    pub fn after_work(&mut self, worked: Duration) -> Pace {
        if let Some(load) = self.sampler.busy_percent() {
            self.paused = if self.paused {
                load >= self.config.resume_below
            } else {
                load >= self.config.pause_above
            };
        }
        if self.paused {
            return Pace::Pause;
        }
        let duty = self.config.max_duty.clamp(0.05, 1.0);
        let rest = worked.mul_f64((1.0 - duty) / duty);
        Pace::Go {
            rest: rest.min(self.config.max_rest),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Devuelve las cargas que se le dan, en orden.
    struct Loads(Vec<u8>);

    impl LoadSampler for Loads {
        fn busy_percent(&mut self) -> Option<u8> {
            (!self.0.is_empty()).then(|| self.0.remove(0))
        }
    }

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn rests_in_proportion_to_the_work_done() {
        let mut t = Throttle::new(NoLoad, ThrottleConfig::default()); // 50 %
        assert_eq!(t.after_work(MS(30)), Pace::Go { rest: MS(30) });
        let quarter = ThrottleConfig {
            max_duty: 0.25,
            ..ThrottleConfig::default()
        };
        let mut t = Throttle::new(NoLoad, quarter);
        assert_eq!(t.after_work(MS(10)), Pace::Go { rest: MS(30) });
    }

    #[test]
    fn a_huge_file_does_not_mean_a_huge_rest() {
        let mut t = Throttle::new(NoLoad, ThrottleConfig::default());
        assert_eq!(t.after_work(MS(20_000)), Pace::Go { rest: MS(500) });
    }

    #[test]
    fn pauses_when_the_pc_is_busy_and_waits_for_it_to_calm_down() {
        let mut t = Throttle::new(
            Loads(vec![20, 80, 60, 70, 40, 60]),
            ThrottleConfig::default(),
        );
        assert!(matches!(t.after_work(MS(10)), Pace::Go { .. })); // 20 %: tranquilo
        assert_eq!(t.after_work(MS(10)), Pace::Pause); // 80 %: ocupado
        // 60 % y 70 %: ya no supera el umbral de pausa, pero aún no baja del de volver.
        assert_eq!(t.after_work(Duration::ZERO), Pace::Pause);
        assert_eq!(t.after_work(Duration::ZERO), Pace::Pause);
        assert!(t.is_paused());
        assert_eq!(
            t.after_work(Duration::ZERO),
            Pace::Go {
                rest: Duration::ZERO
            }
        ); // 40 %
        // De vuelta, un 60 % no lo para: hace falta volver a pasar del 75 %.
        assert!(matches!(t.after_work(MS(10)), Pace::Go { .. }));
    }

    #[test]
    fn without_a_reading_it_keeps_its_state() {
        let mut t = Throttle::new(Loads(vec![90]), ThrottleConfig::default());
        assert_eq!(t.after_work(MS(1)), Pace::Pause);
        assert_eq!(t.after_work(Duration::ZERO), Pace::Pause); // sin lectura: sigue igual
    }
}
