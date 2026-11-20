//! Carga del sistema y prioridad del hilo de indexado (lo que depende de Windows del
//! «que no se note»; la decisión de cuándo parar está en `sift_indexer::throttle`).

use sift_indexer::LoadSampler;

/// Uso de CPU de todo el sistema, con `GetSystemTimes`: Windows da el tiempo acumulado
/// en reposo, en modo núcleo y en modo usuario; la diferencia entre dos lecturas dice
/// qué parte del intervalo estuvo ocupada la CPU.
///
/// La carga se mide sobre **al menos [`MIN_WINDOW`]**: entre dos archivos pasan unos pocos
/// milisegundos, y en una ventana tan corta cualquier pico de otro programa pasa del 75 %. Con
/// el PC al 15 % el indexado se quedaba casi siempre en pausa: 100 000 archivos tardaban horas
/// (pruebas de aguante 1.0). Mientras no ha pasado ese tiempo no hay lectura nueva (`None`) y
/// el freno mantiene lo que decidió la última vez.
#[derive(Default)]
pub struct SystemLoad {
    /// (reposo, total) de la lectura anterior, en unidades de 100 ns.
    previous: Option<(u64, u64)>,
}

/// Lo mínimo que tiene que durar una medida de la carga (en unidades de 100 ns: 1 s).
const MIN_WINDOW: u64 = 10_000_000;

/// Porcentaje ocupado entre dos lecturas de (reposo, total).
fn busy_between(before: (u64, u64), now: (u64, u64)) -> Option<u8> {
    let total = now.1.checked_sub(before.1)?;
    let idle = now.0.checked_sub(before.0)?;
    if total == 0 {
        return None; // dos lecturas demasiado seguidas
    }
    let busy = total.saturating_sub(idle);
    Some((busy * 100 / total).min(100) as u8)
}

impl LoadSampler for SystemLoad {
    fn busy_percent(&mut self) -> Option<u8> {
        let now = system_times()?;
        let Some(before) = self.previous else {
            self.previous = Some(now);
            return None;
        };
        // El tiempo total crece a «núcleos × tiempo real»: con varios núcleos, 1 s de ventana
        // se alcanza antes, y eso basta para una medida estable.
        if now.1.saturating_sub(before.1) < MIN_WINDOW {
            return None;
        }
        self.previous = Some(now);
        busy_between(before, now)
    }
}

#[cfg(windows)]
#[allow(unsafe_code)] // GetSystemTimes escribe en tres FILETIME locales, válidos durante la llamada.
fn system_times() -> Option<(u64, u64)> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;

    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: los tres punteros apuntan a variables locales inicializadas.
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    // El tiempo de núcleo ya incluye el de reposo.
    Some((ticks(idle), ticks(kernel) + ticks(user)))
}

#[cfg(not(windows))]
fn system_times() -> Option<(u64, u64)> {
    None
}

/// Pone el hilo actual en «modo segundo plano» de Windows: baja su prioridad de CPU **y
/// de disco**, que es lo que más se nota al indexar. Es lo que usan el indexador de
/// Windows y los antivirus para sus tareas de fondo.
#[cfg(windows)]
#[allow(unsafe_code)] // SetThreadPriority sobre el pseudo-handle del hilo actual.
pub fn enter_background_mode() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
    };
    // SAFETY: GetCurrentThread devuelve un pseudo-handle siempre válido para este hilo.
    let ok = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) };
    if ok == 0 {
        tracing::warn!("no se pudo bajar la prioridad del hilo de indexado");
    }
}

#[cfg(not(windows))]
pub fn enter_background_mode() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_is_the_part_of_the_interval_that_was_not_idle() {
        assert_eq!(busy_between((0, 0), (25, 100)), Some(75));
        assert_eq!(busy_between((100, 400), (200, 500)), Some(0)); // todo reposo
        assert_eq!(busy_between((0, 0), (0, 0)), None);
    }

    #[cfg(windows)]
    #[test]
    fn reads_the_real_system_load_over_at_least_a_second() {
        let mut load = SystemLoad::default();
        assert_eq!(load.busy_percent(), None); // la primera lectura no tiene con qué comparar
        // Enseguida, aún no hay ventana suficiente: sin lectura nueva (el freno no cambia).
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(load.busy_percent(), None);
        // Al rato, sí.
        let started = std::time::Instant::now();
        let reading = loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(p) = load.busy_percent() {
                break Some(p);
            }
            if started.elapsed() > std::time::Duration::from_millis(1500) {
                break None;
            }
        };
        assert!(reading.is_some_and(|p| p <= 100), "{reading:?}");
    }
}
