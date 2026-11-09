//! Medición de "atajo → ventana pintada".
//!
//! El reloj empieza en el hilo del atajo, en cuanto se detecta la pulsación, y se para
//! cuando la UI avisa de que ya ha pintado el primer fotograma con la ventana visible.
//! Así se mide lo que percibe el usuario, no solo lo que tarda `window.show()`.

use std::collections::VecDeque;
use std::time::Duration;

use serde::Serialize;

/// Cuántas medidas recientes se guardan para calcular percentiles.
const CAPACITY: usize = 100;

#[derive(Debug, Default)]
pub struct LatencyStats {
    samples: VecDeque<Duration>,
}

/// Resumen que se manda a la UI y se escribe en el log.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LatencySummary {
    pub last_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub samples: u32,
}

impl LatencyStats {
    pub fn record(&mut self, sample: Duration) {
        if self.samples.len() == CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn summary(&self) -> Option<LatencySummary> {
        let last = *self.samples.back()?;
        let mut sorted: Vec<Duration> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        Some(LatencySummary {
            last_ms: ms(last),
            p50_ms: ms(percentile(&sorted, 50)),
            p95_ms: ms(percentile(&sorted, 95)),
            samples: sorted.len() as u32,
        })
    }
}

/// Percentil por el método del "rango más cercano": el menor valor que deja al menos
/// el `p` % de las medidas por debajo o igual. `sorted` no puede estar vacío.
fn percentile(sorted: &[Duration], p: usize) -> Duration {
    let rank = (p * sorted.len()).div_ceil(100).max(1);
    sorted[rank - 1]
}

fn ms(d: Duration) -> f64 {
    (d.as_secs_f64() * 10_000.0).round() / 10.0 // milisegundos con un decimal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms_dur(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn empty_has_no_summary() {
        assert_eq!(LatencyStats::default().summary(), None);
    }

    #[test]
    fn percentiles_use_nearest_rank() {
        let mut s = LatencyStats::default();
        for v in 1..=20 {
            s.record(ms_dur(v)); // 1..20 ms, en orden
        }
        let sum = s.summary().unwrap();
        assert_eq!(sum.samples, 20);
        assert_eq!(sum.last_ms, 20.0);
        assert_eq!(sum.p50_ms, 10.0);
        assert_eq!(sum.p95_ms, 19.0);
    }

    #[test]
    fn single_sample_is_every_percentile() {
        let mut s = LatencyStats::default();
        s.record(Duration::from_micros(12_345));
        let sum = s.summary().unwrap();
        assert_eq!((sum.last_ms, sum.p50_ms, sum.p95_ms), (12.3, 12.3, 12.3));
    }

    #[test]
    fn keeps_only_the_most_recent_samples() {
        let mut s = LatencyStats::default();
        for _ in 0..CAPACITY {
            s.record(ms_dur(500));
        }
        for _ in 0..CAPACITY {
            s.record(ms_dur(5));
        }
        let sum = s.summary().unwrap();
        assert_eq!(sum.samples as usize, CAPACITY);
        assert_eq!(sum.p95_ms, 5.0); // los 500 ms antiguos ya no cuentan
    }
}
