//! La ventana del lanzador: se crea una sola vez al arrancar (oculta) y después solo se
//! muestra y se oculta. Crear una ventana con WebView2 cuesta cientos de milisegundos;
//! mostrar una que ya existe, unos pocos. Así se cumple el objetivo de < 100 ms.

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

/// Etiqueta de la ventana del lanzador en `tauri.conf.json`.
pub const LABEL: &str = "main";

/// Tinte del Acrylic (RGBA). Un gris azulado oscuro con bastante opacidad, para que el
/// texto se lea bien sobre cualquier fondo.
const ACRYLIC_TINT: (u8, u8, u8, u8) = (24, 24, 30, 170);

/// Fracción de la altura del monitor a la que queda el borde superior de la barra.
/// Como Spotlight o PowerToys Run: algo por encima del centro, donde se mira primero.
const TOP_FRACTION: f64 = 0.22;

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Aplica el efecto Acrylic. Devuelve `false` si el sistema no lo soporta (o falla):
/// en ese caso la UI pinta un fondo opaco y la app funciona igual.
pub fn apply_acrylic(window: &WebviewWindow) -> bool {
    #[cfg(windows)]
    {
        match window_vibrancy::apply_acrylic(window, Some(ACRYLIC_TINT)) {
            Ok(()) => {
                tracing::info!("efecto Acrylic aplicado");
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "Acrylic no disponible; se usa fondo opaco");
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (window, ACRYLIC_TINT);
        false
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
    position_on_active_monitor(app, &window);
    match window.show().and_then(|()| window.set_focus()) {
        Ok(()) => {
            #[cfg(windows)]
            if let Ok(hwnd) = window.hwnd() {
                foreground::force(hwnd.0);
            }
            tracing::debug!("lanzador mostrado");
        }
        Err(e) => tracing::warn!(error = %e, "no se pudo mostrar el lanzador"),
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

/// Lo que hace el atajo: si el lanzador está delante, lo oculta; si no, lo muestra.
pub fn toggle(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    if visible && focused {
        hide(app);
    } else {
        show(app);
    }
}

pub fn hide(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    match window.hide() {
        Ok(()) => tracing::debug!("lanzador oculto"),
        Err(e) => tracing::warn!(error = %e, "no se pudo ocultar el lanzador"),
    }
}
