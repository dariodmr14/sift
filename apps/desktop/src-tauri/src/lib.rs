//! Punto de entrada de la app Tauri.

mod appearance;
mod commands;
mod hotkey;
mod indexing;
mod latency;
mod launcher;
mod load;
mod logging;
mod materials;
mod models;
mod search_commands;
mod settings_window;
mod shell;
mod smart;
mod spaces_commands;
mod tray;
mod waits;
mod watcher;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use sift_core::{Hotkey, Settings};
use tauri::{AppHandle, Manager};

use crate::hotkey::HotkeyListener;

/// Con `SIFT_FORCE_KEYBOARD_HOOK=1` se salta `RegisterHotKey` y se usa el hook de teclado,
/// para poder probar la vía alternativa aunque la principal funcione.
const FORCE_HOOK_ENV: &str = "SIFT_FORCE_KEYBOARD_HOOK";

/// Argumento con el que Windows lanza Sift al iniciar sesión (lo registra el plugin de
/// autostart). En ese caso no se muestra el lanzador: la app arranca en silencio.
const AUTOSTART_ARG: &str = "--autostart";

/// Si la base de datos no se pudo abrir, el motivo. La app sigue funcionando con una base
/// de datos en memoria (para que la barra no deje de abrirse), pero avisa en Ajustes.
pub(crate) struct StorageStatus(pub Option<String>);

/// El atajo activo. En un `Mutex` porque los ajustes pueden cambiarlo en marcha.
#[derive(Default)]
struct HotkeyState(Mutex<Option<HotkeyListener>>);

fn settings_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("settings.json"))
}

pub(crate) fn load_settings(app: &AppHandle) -> Settings {
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
        // El reloj de la latencia empieza aquí, en cuanto se detecta la pulsación.
        let pressed_at = Instant::now();
        // Estamos en el hilo del atajo (o dentro del hook): no se bloquea aquí.
        // Se encola el trabajo en el hilo principal y se vuelve enseguida.
        let handle = app.clone();
        if let Err(e) = app.run_on_main_thread(move || launcher::toggle(&handle, pressed_at)) {
            tracing::error!(error = %e, "no se pudo encolar la acción del atajo");
        }
    })
}

/// Lee, cambia y guarda los ajustes.
pub(crate) fn update_settings(
    app: &AppHandle,
    change: impl FnOnce(&mut Settings),
) -> anyhow::Result<()> {
    let path = settings_path(app)?;
    let mut settings = Settings::load(&path).unwrap_or_default();
    change(&mut settings);
    settings.save(&path)?;
    Ok(())
}

fn open_database(app: &AppHandle) -> (sift_storage::Database, Option<String>) {
    let opened = app
        .path()
        .app_data_dir()
        .map_err(anyhow::Error::from)
        .and_then(|dir| Ok(sift_storage::Database::open(&dir.join("sift.db"))?));
    match opened {
        Ok(db) => (db, None),
        Err(e) => {
            tracing::error!(error = %e, "no se pudo abrir la base de datos; se usa una en memoria");
            let fallback = sift_storage::Database::open_in_memory()
                .expect("SQLite en memoria no puede fallar salvo sin memoria");
            (fallback, Some(format!("{e:#}")))
        }
    }
}

/// Abre el índice de búsqueda (derivado: si falla, uno en memoria que se rellena desde
/// SQLite, para que la búsqueda funcione igual aunque no se guarde en disco).
fn open_search_index(app: &AppHandle) -> std::sync::Arc<sift_search::TextIndex> {
    let opened = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())
        .and_then(|dir| {
            sift_search::TextIndex::open(&dir.join("index").join("text")).map_err(|e| e.to_string())
        });
    let index = opened.unwrap_or_else(|e| {
        tracing::error!(error = %e, "no se pudo abrir el índice en disco; se usa uno en memoria");
        sift_search::TextIndex::in_memory().expect("un índice en memoria no puede fallar")
    });
    std::sync::Arc::new(index)
}

/// Cambia el atajo en marcha. Si el nuevo no se puede registrar, se recupera el anterior
/// y se devuelve el error: nunca te quedas sin atajo por probar uno ocupado.
pub(crate) fn change_hotkey(app: &AppHandle, new: Hotkey) -> anyhow::Result<()> {
    let state = app.state::<HotkeyState>();
    let mut slot = state.0.lock().unwrap_or_else(|p| p.into_inner());
    let old = slot.as_ref().map(HotkeyListener::hotkey);
    if old == Some(new) {
        return Ok(());
    }
    // Primero se suelta el anterior (si no, registrar el mismo u otro choca con él).
    drop(slot.take());
    match start_hotkey(app, new) {
        Ok(listener) => {
            *slot = Some(listener);
            update_settings(app, |s| s.hotkey = new)
        }
        Err(e) => {
            if let Some(old) = old {
                *slot = start_hotkey(app, old).ok();
            }
            Err(e)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = commands::specta_builder();
    // En desarrollo se regeneran los tipos de TypeScript en cada arranque.
    #[cfg(debug_assertions)]
    if let Err(e) = specta.export(commands::typescript(), commands::BINDINGS_PATH) {
        eprintln!("no se pudieron exportar los tipos a bindings.ts: {e}");
    }

    let autostarted = std::env::args().any(|a| a == AUTOSTART_ARG);

    tauri::Builder::default()
        // Tiene que ser el primer plugin. Si Sift ya está abierto y se lanza otra vez
        // (p. ej. desde el menú Inicio), el segundo proceso se cierra y el primero muestra
        // el lanzador. Así no hay dos procesos peleándose por el mismo atajo.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            launcher::show(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent, // solo afecta a macOS
            Some(vec![AUTOSTART_ARG]),
        ))
        // Que tao NO registre el teclado como Raw Input. Con ese registro activo, Windows
        // deja de llamar a nuestro hook de teclado de bajo nivel (la vía alternativa del
        // atajo) dentro de este proceso. No usamos esos eventos de dispositivo, así que
        // además ahorramos CPU. Ojo: en tao `Always` significa "filtrarlos siempre".
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .manage(HotkeyState::default())
        .plugin(tauri_plugin_dialog::init())
        .manage(launcher::LauncherState::default())
        .manage(shell::IconCache::default())
        .manage(materials::Materials::default())
        .manage(settings_window::ReturnToLauncher::default())
        .manage(search_commands::SearchStats::default())
        .manage(search_commands::FrecencyCache::default())
        .manage(spaces_commands::SpaceRun::default())
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            let log_guard = logging::init(&app.path().app_log_dir()?)?;
            app.manage(log_guard); // vive mientras viva la app
            tracing::info!(version = sift_core::VERSION, "Sift arrancando");

            let handle = app.handle();
            let (db, storage_error) = open_database(handle);
            app.manage(spaces_commands::Db(Mutex::new(db)));
            app.manage(StorageStatus(storage_error));
            let index = open_search_index(handle);
            // `pdfium.dll` viaja con la app como recurso (ver ADR 0008).
            let pdfium_dir = app.path().resource_dir().ok().map(|d| d.join("pdfium"));
            app.manage(indexing::Indexing::start(handle, index, pdfium_dir));
            launcher::init_glass(handle);
            tray::create(handle)?;

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

            // Al arrancar a mano se muestra el lanzador, para que se vea que Sift está
            // funcionando. Al arrancar con Windows, no: espera en la bandeja.
            if !autostarted {
                launcher::show(handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Ajustes se puede mover: mientras se arrastra o redimensiona, el Acrylic
            // pasa a blur con el mismo tinte (ver `materials::on_moved`).
            if window.label() == settings_window::LABEL {
                if let tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) = event
                    && let Some(w) = window.app_handle().get_webview_window(window.label())
                {
                    materials::on_moved(window.app_handle(), &w);
                }
                // Cerrados los ajustes: si se abrieron desde la barra, la barra vuelve.
                if let tauri::WindowEvent::Destroyed = event {
                    settings_window::on_closed(window.app_handle());
                }
                return;
            }
            if window.label() != launcher::LABEL {
                return;
            }
            match event {
                // Alt+F4 sobre el lanzador lo oculta en vez de destruirlo: si se cerrara,
                // la próxima vez habría que crearlo de nuevo (lento) o la app terminaría.
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    launcher::hide(window.app_handle());
                }
                // Si haces clic en otra ventana, el lanzador se aparta (como Spotlight).
                tauri::WindowEvent::Focused(false) => launcher::hide(window.app_handle()),
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Tauri");
}
