//! Ventana de ajustes. Al revés que el lanzador, se crea solo cuando se abre y se
//! destruye al cerrarla: se usa poco y así no ocupa memoria el resto del tiempo.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

pub fn open(app: &AppHandle) {
    // Si ya está abierta, solo se trae al frente.
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
        .title("Ajustes de Sift")
        .inner_size(520.0, 520.0)
        .resizable(false)
        .maximizable(false)
        .center()
        .build();
    if let Err(e) = built {
        tracing::error!(error = %e, "no se pudo abrir la ventana de ajustes");
    }
}
