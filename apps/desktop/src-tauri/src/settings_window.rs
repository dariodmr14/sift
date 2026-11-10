//! Ventana de ajustes. Al revés que el lanzador, se crea solo cuando se abre y se
//! destruye al cerrarla: se usa poco y así no ocupa memoria el resto del tiempo.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

use crate::appearance;
use crate::commands::NewSpaceRequested;
use crate::materials::{self, Materials, Surface};

pub const LABEL: &str = "settings";

/// Aplica o quita el vidrio en la ventana de ajustes si está abierta (al cambiar la
/// opción). Devuelve si ha quedado con material.
pub fn set_glass(app: &AppHandle, enabled: bool) -> bool {
    let Some(window) = app.get_webview_window(LABEL) else {
        return false;
    };
    let materials = app.state::<Materials>();
    let current = materials.get(LABEL);
    if enabled && current == materials::Material::None {
        let m = materials::apply(&window, Surface::AppWindow, appearance::theme());
        materials.set(LABEL, m);
    } else if !enabled && current != materials::Material::None {
        materials::clear(&window, current);
        materials.set(LABEL, materials::Material::None);
    }
    materials.get(LABEL) != materials::Material::None
}

/// Abre los ajustes. Con `new_space`, directamente en "crear Espacio".
pub fn open(app: &AppHandle, new_space: bool) {
    // Si ya está abierta, se trae al frente (y se le avisa si hay que crear un Espacio).
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        if new_space {
            let _ = NewSpaceRequested.emit(app);
        }
        return;
    }
    // Si no, se crea; el parámetro de la URL le dice que empiece creando un Espacio.
    let page = if new_space {
        "settings.html?new=1"
    } else {
        "settings.html"
    };
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(page.into()))
        .title("Ajustes de Sift")
        .inner_size(920.0, 680.0)
        .min_inner_size(760.0, 520.0)
        .center()
        // Transparente para que se vea el material de Windows (Mica o blur) detrás.
        .transparent(true)
        .build();
    match built {
        Ok(window) => {
            // Opaca por defecto; vidrio solo con «Vidrio en toda la app».
            if crate::load_settings(app).appearance.glass_everywhere {
                let material = materials::apply(&window, Surface::AppWindow, appearance::theme());
                app.state::<Materials>().set(LABEL, material);
            }
        }
        Err(e) => tracing::error!(error = %e, "no se pudo abrir la ventana de ajustes"),
    }
}
