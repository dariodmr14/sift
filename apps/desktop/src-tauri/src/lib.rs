//! Punto de entrada de la app Tauri.

mod launcher;
mod logging;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let log_guard = logging::init(&app.path().app_log_dir()?)?;
            app.manage(log_guard); // vive mientras viva la app
            tracing::info!(version = sift_core::VERSION, "Sift arrancando");

            if let Some(window) = launcher::window(app.handle()) {
                launcher::apply_acrylic(&window);
            }
            launcher::show(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Alt+F4 sobre el lanzador lo oculta en vez de destruirlo: si se cerrara,
            // la próxima vez habría que crearlo de nuevo (lento) o la app terminaría.
            if window.label() == launcher::LABEL
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                launcher::hide(window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Tauri");
}
