//! ¿Puede Windows pintar el vidrio ahora mismo?
//!
//! Hay casos en que Windows acepta el Acrylic sin dar error pero no lo dibuja: ni
//! desenfoque ni tinte, la ventana queda transparente y el texto, encima del escritorio.
//! Pasó en Windows Sandbox (sin aceleración gráfica): la introducción casi no se leía.
//! Windows no dice si lo está pintando, así que se miran las situaciones conocidas en las
//! que no lo hace bien y en ellas Sift va opaco, sin pedir vidrio. Ver ADR 0022.

use std::sync::OnceLock;

use serde::Serialize;

/// Por qué Sift no pone vidrio ahora mismo aunque esté activado. Ajustes lo explica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GlassBlocker {
    /// «Efectos de transparencia» desactivados en Windows.
    TransparencyOff,
    /// Windows Sandbox.
    Sandbox,
    /// Escritorio remoto: Windows no desenfoca a través de la conexión.
    RemoteSession,
    /// Sin aceleración gráfica: dibuja el procesador (máquinas virtuales, un PC sin el
    /// driver de su tarjeta gráfica…).
    NoGpu,
    /// Ahorro de batería: Windows quita las transparencias.
    BatterySaver,
}

/// Lo que se ha mirado del sistema, para decidir con una función pura (y probarla).
#[derive(Debug, Clone, Copy, Default)]
pub struct GlassEnvironment {
    pub transparency_off: bool,
    pub sandbox: bool,
    pub remote_session: bool,
    pub software_rendering: bool,
    pub battery_saver: bool,
}

/// El primer motivo que impide el vidrio, o `None` si se puede usar.
pub fn blocker_for(env: GlassEnvironment) -> Option<GlassBlocker> {
    [
        (env.transparency_off, GlassBlocker::TransparencyOff),
        (env.sandbox, GlassBlocker::Sandbox),
        (env.remote_session, GlassBlocker::RemoteSession),
        (env.software_rendering, GlassBlocker::NoGpu),
        (env.battery_saver, GlassBlocker::BatterySaver),
    ]
    .into_iter()
    .find_map(|(on, why)| on.then_some(why))
}

/// Motivo para no poner vidrio ahora, o `None`. Cuesta microsegundos (registro y un par
/// de llamadas al sistema), salvo la primera vez, que mira la tarjeta gráfica (unos ms; no
/// cambia mientras Sift está abierto). Se llama al mostrar la barra y al activar Ajustes,
/// así que el ahorro de batería o una conexión remota se notan en cuanto cambian.
pub fn blocker() -> Option<GlassBlocker> {
    static SOFTWARE: OnceLock<bool> = OnceLock::new();
    let env = GlassEnvironment {
        transparency_off: !crate::appearance::transparency_enabled(),
        sandbox: platform::sandbox(),
        remote_session: platform::remote_session(),
        software_rendering: *SOFTWARE.get_or_init(platform::software_rendering),
        battery_saver: platform::battery_saver(),
    };
    blocker_for(env)
}

#[cfg(windows)]
#[allow(unsafe_code)] // Consultas al sistema: energía, sesión y tarjeta gráfica (DXGI).
mod platform {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
    };
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};

    /// «Microsoft Basic Render Driver»: Windows dibuja con el procesador (WARP).
    const MICROSOFT_VENDOR: u32 = 0x1414;
    const BASIC_RENDER_DEVICE: u32 = 0x8c;

    pub fn battery_saver() -> bool {
        let mut status = SYSTEM_POWER_STATUS {
            ACLineStatus: 0,
            BatteryFlag: 0,
            BatteryLifePercent: 0,
            SystemStatusFlag: 0,
            BatteryLifeTime: 0,
            BatteryFullLifeTime: 0,
        };
        // SAFETY: la función solo escribe en la estructura que se le pasa.
        let ok = unsafe { GetSystemPowerStatus(&mut status) } != 0;
        ok && status.SystemStatusFlag == 1
    }

    pub fn remote_session() -> bool {
        // SAFETY: sin punteros; solo lee una métrica del sistema.
        unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
    }

    /// Windows Sandbox entra siempre con esta cuenta.
    pub fn sandbox() -> bool {
        std::env::var("USERNAME").is_ok_and(|u| u.eq_ignore_ascii_case("WDAGUtilityAccount"))
    }

    /// ¿La tarjeta gráfica principal (la primera que da DXGI, la del escritorio) es el
    /// renderizador por software? Si no se puede saber, se supone que no.
    pub fn software_rendering() -> bool {
        // SAFETY: llamadas COM de DXGI a través de los envoltorios de `windows`, que
        // liberan los objetos al salir del ámbito.
        let desc = unsafe {
            CreateDXGIFactory1::<IDXGIFactory1>()
                .and_then(|f| f.EnumAdapters1(0))
                .and_then(|a| a.GetDesc1())
        };
        match desc {
            Ok(d) => {
                let software = d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0
                    || (d.VendorId == MICROSOFT_VENDOR && d.DeviceId == BASIC_RENDER_DEVICE);
                tracing::info!(
                    vendor = d.VendorId,
                    device = d.DeviceId,
                    software,
                    "tarjeta gráfica principal"
                );
                software
            }
            Err(e) => {
                tracing::debug!(error = %e, "no se pudo mirar la tarjeta gráfica");
                false
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn battery_saver() -> bool {
        false
    }
    pub fn remote_session() -> bool {
        false
    }
    pub fn sandbox() -> bool {
        false
    }
    pub fn software_rendering() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn glass_is_only_used_where_windows_can_paint_it() {
        assert_eq!(blocker_for(GlassEnvironment::default()), None);
        let one = |f: fn(&mut GlassEnvironment)| {
            let mut env = GlassEnvironment::default();
            f(&mut env);
            blocker_for(env)
        };
        assert_eq!(
            one(|e| e.transparency_off = true),
            Some(GlassBlocker::TransparencyOff)
        );
        assert_eq!(
            one(|e| e.battery_saver = true),
            Some(GlassBlocker::BatterySaver)
        );
        assert_eq!(
            one(|e| e.remote_session = true),
            Some(GlassBlocker::RemoteSession)
        );
        assert_eq!(
            one(|e| e.software_rendering = true),
            Some(GlassBlocker::NoGpu)
        );
        // Windows Sandbox suele ir también sin aceleración gráfica: se dice «Sandbox».
        assert_eq!(
            one(|e| {
                e.sandbox = true;
                e.software_rendering = true;
            }),
            Some(GlassBlocker::Sandbox)
        );
    }

    /// Se mira cada vez que se muestra la barra (objetivo < 100 ms): tiene que ser rápido.
    #[test]
    fn looking_at_the_system_is_fast() {
        let _ = blocker(); // la primera vez mira la tarjeta gráfica
        let start = Instant::now();
        for _ in 0..100 {
            let _ = blocker();
        }
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "{:?}",
            start.elapsed()
        );
    }
}
