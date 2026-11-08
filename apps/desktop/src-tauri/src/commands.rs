//! Comandos IPC (UI → Rust) y eventos (Rust → UI).
//!
//! Cada comando lleva `#[tauri::command]` (lo expone a la UI) y `#[specta::specta]`
//! (genera su tipo en TypeScript). Los tipos se exportan a `src/bindings.ts`: si aquí
//! cambia una firma, la UI deja de compilar en vez de fallar en ejecución.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_specta::{Event, collect_commands, collect_events};

use crate::hotkey::Backend;
use crate::latency::LatencySummary;
use crate::{HotkeyState, launcher};

/// Rust avisa a la UI de que el lanzador acaba de mostrarse: hay que limpiar la búsqueda,
/// poner el foco en la caja y avisar de vuelta cuando se haya pintado.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
pub struct LauncherShown;

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
}

#[tauri::command]
#[specta::specta]
fn hide_launcher(app: AppHandle) {
    launcher::hide(&app);
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
    }
}

/// Todos los comandos y eventos. Se usa al arrancar y en el test que comprueba que
/// `bindings.ts` está al día.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            hide_launcher,
            launcher_painted,
            get_status
        ])
        .events(collect_events![LauncherShown])
}

/// Ruta de los tipos generados, relativa a este crate (`src-tauri/`).
pub const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

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
