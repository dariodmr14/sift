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
    if let Err(e) = window.show().and_then(|()| window.set_focus()) {
        tracing::warn!(error = %e, "no se pudo mostrar el lanzador");
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = window(app)
        && let Err(e) = window.hide()
    {
        tracing::warn!(error = %e, "no se pudo ocultar el lanzador");
    }
}
