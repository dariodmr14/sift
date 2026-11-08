//! Punto de entrada de la app Tauri.

mod hotkey;
mod launcher;
mod logging;

use std::path::PathBuf;
use std::sync::Mutex;

use sift_core::{Hotkey, Settings};
use tauri::{AppHandle, Manager};

use crate::hotkey::HotkeyListener;

/// Con `SIFT_FORCE_KEYBOARD_HOOK=1` se salta `RegisterHotKey` y se usa el hook de teclado,
/// para poder probar la vía alternativa aunque la principal funcione.
const FORCE_HOOK_ENV: &str = "SIFT_FORCE_KEYBOARD_HOOK";

/// El atajo activo. En un `Mutex` porque los ajustes pueden cambiarlo en marcha.
#[derive(Default)]
struct HotkeyState(Mutex<Option<HotkeyListener>>);

fn settings_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    let loaded = settings_path(app).and_then(|p| Ok(Settings::load(&p)?));
    loaded.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "no se pudieron cargar los ajustes; se usan los de por defecto");
        Settings::default()
    })
}

fn start_hotkey(app: &AppHandle, hotkey: Hotkey) -> anyhow::Result<HotkeyListener> {
    let force_hook = std::env::var(FORCE_HOOK_ENV).is_ok_and(|v| v == "1");
    let app = app.clone();
    HotkeyListener::start(hotkey, force_hook, move || {
        // Estamos en el hilo del atajo (o dentro del hook): no se bloquea aquí.
        // Se encola el trabajo en el hilo principal y se vuelve enseguida.
        let handle = app.clone();
        if let Err(e) = app.run_on_main_thread(move || launcher::toggle(&handle)) {
            tracing::error!(error = %e, "no se pudo encolar la acción del atajo");
        }
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Que tao NO registre el teclado como Raw Input. Con ese registro activo, Windows
        // deja de llamar a nuestro hook de teclado de bajo nivel (la vía alternativa del
        // atajo) dentro de este proceso. No usamos esos eventos de dispositivo, así que
        // además ahorramos CPU. Ojo: en tao `Always` significa "filtrarlos siempre".
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .manage(HotkeyState::default())
        .setup(|app| {
            let log_guard = logging::init(&app.path().app_log_dir()?)?;
            app.manage(log_guard); // vive mientras viva la app
            tracing::info!(version = sift_core::VERSION, "Sift arrancando");

            let handle = app.handle();
            if let Some(window) = launcher::window(handle) {
                launcher::apply_acrylic(&window);
            }

            let settings = load_settings(handle);
            match start_hotkey(handle, settings.hotkey) {
                Ok(listener) => {
                    *app.state::<HotkeyState>()
                        .0
                        .lock()
                        .unwrap_or_else(|p| p.into_inner()) = Some(listener)
                }
                // Sin atajo la app sigue siendo usable desde la bandeja y los ajustes.
                Err(e) => tracing::error!(error = %e, "la app arranca sin atajo global"),
            }

            launcher::show(handle);
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
