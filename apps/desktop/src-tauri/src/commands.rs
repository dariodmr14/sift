//! Comandos IPC (UI → Rust) y eventos (Rust → UI).
//!
//! Cada comando lleva `#[tauri::command]` (lo expone a la UI) y `#[specta::specta]`
//! (genera su tipo en TypeScript). Los tipos se exportan a `src/bindings.ts`: si aquí
//! cambia una firma, la UI deja de compilar en vez de fallar en ejecución.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_specta::{Event, collect_commands, collect_events};

use sift_core::Hotkey;
use tauri_plugin_autostart::ManagerExt;

use crate::hotkey::Backend;
use crate::latency::LatencySummary;
use crate::search_commands as search;
use crate::spaces_commands as spaces;
use crate::{HotkeyState, StorageStatus, launcher, settings_window};

/// Errores que la UI puede mostrar. En TypeScript llega como `{ kind, message }`.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    /// El texto no es un atajo válido (p. ej. "Space" sin modificador).
    InvalidHotkey(String),
    /// El atajo es válido pero no se pudo activar.
    HotkeyUnavailable(String),
    /// Datos que no pasan la validación (un Espacio sin nombre, una URL no web…).
    Invalid(String),
    NotFound(String),
    /// Fallo del sistema (registro de Windows, disco…).
    System(String),
}

/// Ajustes editables desde la ventana de ajustes.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub hotkey: String,
    /// Fuente de verdad: el registro de Windows (no el JSON), para no desincronizarse
    /// si el usuario lo desactiva desde el Administrador de tareas.
    pub launch_at_startup: bool,
    /// "Solo texto": la barra no muestra iconos.
    pub icons_hidden: bool,
    /// «Desactivar vidrio»: toda la app opaca.
    pub disable_glass: bool,
    /// «Tema»: como Windows, claro u oscuro.
    pub theme: crate::appearance::ThemeChoice,
    /// Hay actualizaciones automáticas en esta compilación (la de las Releases). En las
    /// demás, Ajustes no enseña la opción.
    pub updates_available: bool,
    /// «Buscar actualizaciones al arrancar» (encendido por defecto: ADR 0018).
    pub check_updates: bool,
}

/// Rust avisa a la UI de que el lanzador acaba de mostrarse: hay que limpiar la búsqueda,
/// poner el foco en la caja y avisar de vuelta cuando se haya pintado.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct LauncherShown;

/// Rust avisa de que ha ocultado el lanzador (Esc, clic fuera, el atajo…): la UI se
/// reinicia mientras está invisible, así al volver a mostrarse no enseña el estado viejo.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct LauncherHidden;

/// Ha cambiado el tema o el vidrio: cada ventana vuelve a pedir su aspecto
/// (`applyAppearance` y `applyWindowMaterial` en src/appearance.ts).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct AppearanceChanged;

/// Se pidió "crear Espacio" desde la barra con la ventana de ajustes ya abierta.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct NewSpaceRequested;

/// Estado de la app para la UI (y para la pantalla de ajustes).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Si se pudo aplicar Acrylic. Si no, la UI pinta un fondo opaco.
    pub acrylic: bool,
    /// Atajo activo (p. ej. "Alt+Space"), o `null` si no se pudo registrar ninguno.
    pub hotkey: Option<String>,
    pub backend: Option<Backend>,
    pub latency: Option<LatencySummary>,
    /// Si la base de datos no se pudo abrir, el motivo (los cambios no se guardarán).
    pub storage_error: Option<String>,
    /// Tiempo de búsqueda de archivos en Rust (objetivo < 50 ms).
    pub search_latency: Option<LatencySummary>,
}

#[tauri::command]
#[specta::specta]
fn hide_launcher(app: AppHandle) {
    launcher::hide(&app);
}

/// Abre la ventana de ajustes desde la barra (`Ctrl+,`, el engranaje o "ajustes").
/// Con `new_space`, la abre directamente en "crear Espacio".
///
/// Tiene que ser `async`: los comandos síncronos se ejecutan en el hilo principal, y en
/// Windows crear una ventana WebView2 desde ahí se bloquea (la ventana se queda en
/// blanco). Documentado por Tauri en `WebviewWindowBuilder::new`.
#[tauri::command]
#[specta::specta]
async fn open_settings(app: AppHandle, new_space: bool) {
    launcher::hide(&app);
    settings_window::open(&app, new_space, settings_window::Origin::Launcher);
}

/// La UI de ajustes ya ha pintado: se puede mostrar su ventana (se crea oculta).
#[tauri::command]
#[specta::specta]
fn settings_ready(app: AppHandle) {
    settings_window::reveal(&app);
}

/// La UI lo llama tras pintar el primer fotograma con el lanzador visible.
#[tauri::command]
#[specta::specta]
fn launcher_painted(app: AppHandle) {
    launcher::painted(&app);
}

#[tauri::command]
#[specta::specta]
fn get_status(app: AppHandle) -> Status {
    let state = app.state::<HotkeyState>();
    let listener = state.0.lock().unwrap_or_else(|p| p.into_inner());
    let launcher = app.state::<launcher::LauncherState>();
    Status {
        acrylic: launcher.acrylic(),
        hotkey: listener.as_ref().map(|l| l.hotkey().to_string()),
        backend: listener.as_ref().map(|l| l.backend()),
        latency: launcher.latency(),
        storage_error: app.state::<StorageStatus>().0.clone(),
        search_latency: app.state::<search::SearchStats>().summary(),
    }
}

/// Material de Windows de la ventana que pregunta (Acrylic, blur o ninguno), para
/// que su UI elija fondo translúcido u opaco.
#[tauri::command]
#[specta::specta]
fn get_window_material(window: tauri::WebviewWindow) -> crate::materials::Material {
    window
        .state::<crate::materials::Materials>()
        .get(window.label())
}

/// Tema y color de acento de Windows, para que la UI los siga.
#[tauri::command]
#[specta::specta]
fn get_appearance() -> crate::appearance::Appearance {
    crate::appearance::current()
}

#[tauri::command]
#[specta::specta]
fn get_settings(app: AppHandle) -> SettingsView {
    SettingsView {
        hotkey: crate::load_settings(&app).hotkey.to_string(),
        launch_at_startup: app.autolaunch().is_enabled().unwrap_or(false),
        icons_hidden: crate::load_settings(&app).icons_hidden,
        disable_glass: crate::load_settings(&app).appearance.disable_glass,
        theme: crate::load_settings(&app).appearance.theme.into(),
        updates_available: crate::updates::configured(&app),
        check_updates: crate::load_settings(&app).updates.check_on_startup,
    }
}

/// «Buscar actualizaciones al arrancar». Se aplica en el siguiente arranque.
#[tauri::command]
#[specta::specta]
fn set_check_updates(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    crate::update_settings(&app, |s| s.updates.check_on_startup = enabled)
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    Ok(enabled)
}

/// «Desactivar vidrio»: lo guarda y lo aplica ya en todas las ventanas abiertas (la
/// barra y los ajustes). Devuelve si ha quedado desactivado.
#[tauri::command]
#[specta::specta]
fn set_glass_disabled(app: AppHandle, disabled: bool) -> Result<bool, AppError> {
    crate::update_settings(&app, |s| s.appearance.disable_glass = disabled)
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    crate::materials::set_wanted(!disabled);
    for label in [crate::launcher::LABEL, settings_window::LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            crate::materials::sync(&app, &window);
        }
    }
    if let Err(e) = AppearanceChanged.emit(&app) {
        tracing::warn!(error = %e, "no se pudo avisar a la UI del cambio de vidrio");
    }
    Ok(disabled)
}

/// «Tema»: lo guarda y lo aplica ya en toda la app. El vidrio se vuelve a poner con el
/// tinte del tema nuevo y cada ventana se repinta al recibir `AppearanceChanged`.
#[tauri::command]
#[specta::specta]
fn set_theme(
    app: AppHandle,
    theme: crate::appearance::ThemeChoice,
) -> Result<crate::appearance::ThemeChoice, AppError> {
    crate::update_settings(&app, |s| s.appearance.theme = theme.into())
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    crate::appearance::set_preference(theme.into());
    for label in [crate::launcher::LABEL, settings_window::LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            crate::materials::refresh(&app, &window);
        }
    }
    crate::launcher::theme_applied(&app);
    if let Err(e) = AppearanceChanged.emit(&app) {
        tracing::warn!(error = %e, "no se pudo avisar a la UI del cambio de tema");
    }
    Ok(theme)
}

/// Si Windows no puede pintar el vidrio ahora mismo, por qué (Ajustes lo explica);
/// `null` si puede.
#[tauri::command]
#[specta::specta]
fn get_glass_blocker() -> Option<crate::glass_support::GlassBlocker> {
    crate::glass_support::blocker()
}

#[tauri::command]
#[specta::specta]
fn set_icons_hidden(app: AppHandle, hidden: bool) -> Result<bool, AppError> {
    crate::update_settings(&app, |s| s.icons_hidden = hidden)
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    Ok(hidden)
}

#[tauri::command]
#[specta::specta]
fn set_hotkey(app: AppHandle, hotkey: String) -> Result<String, AppError> {
    let parsed: Hotkey = hotkey
        .parse()
        .map_err(|e: sift_core::HotkeyParseError| AppError::InvalidHotkey(e.to_string()))?;
    crate::change_hotkey(&app, parsed).map_err(|e| {
        tracing::warn!(error = format!("{e:#}"), "no se pudo activar el atajo");
        AppError::HotkeyUnavailable(format!(
            "No se pudo activar {parsed}: Windows no lo permite ahora mismo. \
             Prueba con otra combinación."
        ))
    })?;
    Ok(parsed.to_string()) // forma canónica: "ctrl + k" → "Ctrl+K"
}

#[tauri::command]
#[specta::specta]
fn set_launch_at_startup(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| AppError::System(e.to_string()))?;
    tracing::info!(enabled, "arranque con Windows cambiado");
    autolaunch
        .is_enabled()
        .map_err(|e| AppError::System(e.to_string()))
}

/// Todos los comandos y eventos. Se usa al arrancar y en el test que comprueba que
/// `bindings.ts` está al día.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            hide_launcher,
            open_settings,
            launcher_painted,
            get_status,
            get_settings,
            get_appearance,
            get_window_material,
            set_hotkey,
            set_launch_at_startup,
            set_icons_hidden,
            set_glass_disabled,
            set_theme,
            get_glass_blocker,
            set_check_updates,
            settings_ready,
            spaces::list_spaces,
            spaces::create_space,
            spaces::update_space,
            spaces::duplicate_space,
            spaces::delete_space,
            spaces::reorder_spaces,
            spaces::run_space,
            spaces::test_action,
            spaces::check_wait,
            spaces::cancel_space_run,
            spaces::list_monitors,
            spaces::identify_monitors,
            spaces::list_apps,
            spaces::get_icons,
            spaces::pick_path,
            spaces::pick_icon_image,
            spaces::export_spaces,
            spaces::import_spaces,
            search::search_files,
            search::preview_file,
            search::suggest_paths,
            search::recent_files,
            search::list_roots,
            search::unread_files,
            search::get_index_media,
            search::get_smart_search,
            search::set_smart_search,
            search::set_index_media,
            search::add_root,
            search::get_welcome,
            search::add_suggested_folder,
            search::finish_welcome,
            search::allow_model_download,
            search::remove_root,
            search::get_ignore_patterns,
            search::set_ignore_patterns,
            search::index_status,
            search::reindex,
            search::open_file,
            search::reveal_file
        ])
        .events(collect_events![
            LauncherShown,
            LauncherHidden,
            NewSpaceRequested,
            AppearanceChanged,
            crate::materials::GlassMoving,
            crate::indexing::IndexProgress,
            crate::spaces_commands::SpaceProgress,
            crate::spaces_commands::SpacePlacement
        ])
}

/// Ruta de los tipos generados, relativa a este crate (`src-tauri/`). Solo en desarrollo y
/// en tests: la app de release no regenera los tipos.
#[cfg(any(debug_assertions, test))]
pub const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

#[cfg(any(debug_assertions, test))]
pub fn typescript() -> specta_typescript::Typescript {
    specta_typescript::Typescript::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Falla si alguien cambia un comando en Rust y no regenera `bindings.ts`.
    /// Se regenera al arrancar en desarrollo (`npm run tauri dev`) o con
    /// `SIFT_UPDATE_BINDINGS=1 cargo test -p sift-desktop`.
    #[test]
    fn bindings_are_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        let fresh_path = dir.path().join("bindings.ts");
        specta_builder().export(typescript(), &fresh_path).unwrap();
        let fresh = std::fs::read_to_string(&fresh_path).unwrap();
        if std::env::var("SIFT_UPDATE_BINDINGS").is_ok() {
            std::fs::write(BINDINGS_PATH, &fresh).unwrap();
            return;
        }
        let committed = std::fs::read_to_string(BINDINGS_PATH).unwrap_or_default();
        // Se comparan sin tener en cuenta los finales de línea (CRLF/LF en Windows).
        let normalize = |s: &str| s.replace("\r\n", "\n");
        assert!(
            normalize(&fresh) == normalize(&committed),
            "src/bindings.ts está desactualizado: ejecuta \
             `SIFT_UPDATE_BINDINGS=1 cargo test -p sift-desktop` o arranca `npm run tauri dev`"
        );
    }
}
