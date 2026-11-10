//! La ventana del lanzador: se crea una sola vez al arrancar (oculta) y después solo se
//! muestra y se oculta. Crear una ventana con WebView2 cuesta cientos de milisegundos;
//! mostrar una que ya existe, unos pocos. Así se cumple el objetivo de < 100 ms.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};
use tauri_specta::Event;

use crate::appearance::{self, Theme};
use crate::commands::{LauncherHidden, LauncherShown};
use crate::latency::{LatencyStats, LatencySummary};
use crate::materials::{self, Material, Materials, Surface};

/// Etiqueta de la ventana del lanzador en `tauri.conf.json`.
pub const LABEL: &str = "main";

/// Fracción de la altura del monitor a la que queda el borde superior de la barra.
/// Como Spotlight o PowerToys Run: algo por encima del centro, donde se mira primero.
const TOP_FRACTION: f64 = 0.22;

/// Estado compartido del lanzador (lo gestiona Tauri con `app.manage`).
#[derive(Default)]
pub struct LauncherState {
    acrylic: AtomicBool,
    /// Tema con el que se aplicó el Acrylic (para reaplicarlo si cambias el de Windows).
    acrylic_theme: Mutex<Option<Theme>>,
    /// Cuándo se pulsó el atajo que está mostrando la ventana ahora mismo.
    pressed_at: Mutex<Option<Instant>>,
    stats: Mutex<LatencyStats>,
}

impl LauncherState {
    pub fn acrylic(&self) -> bool {
        self.acrylic.load(Ordering::Relaxed)
    }

    pub fn latency(&self) -> Option<LatencySummary> {
        self.stats
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .summary()
    }
}

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Aplica el material de la barra (Acrylic, ver `materials.rs`) y guarda si funcionó.
/// Si no está disponible, la UI pinta un fondo opaco y la app funciona igual.
pub fn init_acrylic(app: &AppHandle) {
    let theme = appearance::theme();
    let material = window(app).map_or(Material::None, |w| {
        materials::apply(&w, Surface::Flyout, theme)
    });
    app.state::<Materials>().set(LABEL, material);
    let state = app.state::<LauncherState>();
    *state
        .acrylic_theme
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some(theme);
    state
        .acrylic
        .store(material != Material::None, Ordering::Relaxed);
}

/// Si el tema de Windows cambió desde la última vez, reaplica el material con el tinte
/// nuevo. Leer el tema del registro cuesta microsegundos; reaplicar, algo más, por eso
/// solo se hace cuando cambia.
fn refresh_acrylic(app: &AppHandle, window: &WebviewWindow) {
    let state = app.state::<LauncherState>();
    if !state.acrylic() {
        return;
    }
    let theme = appearance::theme();
    let mut current = state
        .acrylic_theme
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if *current != Some(theme) {
        materials::apply(window, Surface::Flyout, theme);
        *current = Some(theme);
    }
}

/// Coloca la ventana centrada en horizontal en el monitor donde está el ratón.
/// Así aparece en la pantalla en la que estás trabajando, no siempre en la principal.
fn position_on_active_monitor(app: &AppHandle, window: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };
    let area = monitor.work_area(); // sin la barra de tareas
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + (f64::from(area.size.height) * TOP_FRACTION) as i32;
    if let Err(e) = window.set_position(PhysicalPosition::new(x, y)) {
        tracing::warn!(error = %e, "no se pudo colocar la ventana");
    }
}

pub fn show(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    refresh_acrylic(app, &window);
    position_on_active_monitor(app, &window);
    match window.show().and_then(|()| window.set_focus()) {
        Ok(()) => {
            #[cfg(windows)]
            if let Ok(hwnd) = window.hwnd() {
                foreground::force(hwnd.0);
            }
            tracing::debug!("lanzador mostrado");
            if let Err(e) = LauncherShown.emit(app) {
                tracing::warn!(error = %e, "no se pudo avisar a la UI");
            }
        }
        Err(e) => tracing::warn!(error = %e, "no se pudo mostrar el lanzador"),
    }
}

/// Lo que hace el atajo: si el lanzador está delante, lo oculta; si no, lo muestra.
/// `pressed_at` es el instante de la pulsación, para medir la latencia.
pub fn toggle(app: &AppHandle, pressed_at: Instant) {
    let Some(window) = window(app) else { return };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    if visible && focused {
        hide(app);
    } else {
        *app.state::<LauncherState>()
            .pressed_at
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(pressed_at);
        show(app);
    }
}

/// La UI ha pintado el lanzador: si lo abrió el atajo, se apunta cuánto ha tardado.
pub fn painted(app: &AppHandle) {
    let state = app.state::<LauncherState>();
    let Some(pressed_at) = state
        .pressed_at
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
    else {
        return; // se abrió desde la bandeja o al arrancar: no cuenta
    };
    let elapsed = pressed_at.elapsed();
    let mut stats = state.stats.lock().unwrap_or_else(|p| p.into_inner());
    stats.record(elapsed);
    if let Some(s) = stats.summary() {
        tracing::info!(
            ms = s.last_ms,
            p50 = s.p50_ms,
            p95 = s.p95_ms,
            n = s.samples,
            "atajo → lanzador pintado"
        );
    }
}

pub fn hide(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    match window.hide() {
        Ok(()) => {
            tracing::debug!("lanzador oculto");
            let _ = LauncherHidden.emit(app);
        }
        Err(e) => tracing::warn!(error = %e, "no se pudo ocultar el lanzador"),
    }
}

/// Traer una ventana al frente de forma fiable en Windows.
///
/// Windows no deja que cualquier proceso robe el foco (`SetForegroundWindow` falla en
/// silencio si no cumples sus reglas). Con `RegisterHotKey` el sistema nos da permiso,
/// pero con el hook de teclado no siempre. La técnica clásica: unir un momento nuestra
/// cola de entrada a la de la ventana que está delante (`AttachThreadInput`); para
/// Windows pasamos a ser "la misma entrada" y nos deja activar nuestra ventana.
#[cfg(windows)]
#[allow(unsafe_code)] // Llamadas a Win32 sobre handles de ventana válidos.
mod foreground {
    use std::ffi::c_void;

    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };

    pub fn force(hwnd: *mut c_void) {
        // SAFETY: `hwnd` es la ventana del lanzador, viva mientras la app exista; las
        // demás llamadas solo consultan o cambian el estado de foco de Windows.
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground == hwnd {
                return; // ya estamos delante
            }
            let ours = GetCurrentThreadId();
            let theirs = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
            let attached = theirs != 0 && theirs != ours && AttachThreadInput(ours, theirs, 1) != 0;
            BringWindowToTop(hwnd);
            let ok = SetForegroundWindow(hwnd) != 0;
            if attached {
                AttachThreadInput(ours, theirs, 0);
            }
            if !ok {
                tracing::warn!("Windows no dejó poner el lanzador en primer plano");
            }
        }
    }
}
