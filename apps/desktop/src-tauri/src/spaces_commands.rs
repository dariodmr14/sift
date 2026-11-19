//! Comandos IPC de los Espacios: listar, editar, ejecutar (con confirmación de comandos),
//! selector de apps, iconos de Windows, diálogos de archivo y exportar/importar.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sift_spaces::{
    Action, ActionOutcome, Icon, NoProgress, PlacementState, Progress, Space, SpaceDraft, WaitSpec,
};
use sift_storage::{Database, StorageError};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_specta::Event;

use crate::commands::AppError;
use crate::shell::{self, IconCache, InstalledApp, WindowsOpener};

/// La base de datos, compartida entre comandos. `rusqlite::Connection` no se puede usar
/// desde dos hilos a la vez, de ahí el `Mutex`.
pub struct Db(pub Mutex<Database>);

impl Db {
    fn lock(&self) -> std::sync::MutexGuard<'_, Database> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::Invalid(v) => AppError::Invalid(v.to_string()),
            StorageError::NotFound(_) => AppError::NotFound(e.to_string()),
            other => AppError::System(other.to_string()),
        }
    }
}

/// Un Espacio tal como lo pinta la UI: sus datos y, si su icono es "automático", el
/// icono real de Windows de su primera app, archivo o carpeta.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpaceView {
    pub space: Space,
    pub auto_icon: Option<String>,
    /// Frecencia: por cuánto se multiplica su puntuación en la barra (1 si nunca lo
    /// abriste; como mucho 1,25). La misma regla que los archivos (ADR 0010).
    pub boost: f64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum RunReport {
    /// Había comandos nuevos y, en el diálogo de confirmación, se eligió no ejecutarlos.
    /// No se ha lanzado nada.
    Declined,
    /// Se lanzaron las acciones; cada una con su resultado.
    Done { outcomes: Vec<ActionOutcome> },
}

/// Evento: un Espacio en marcha va por este paso (y, si es una espera, lleva tantos
/// segundos esperando). La barra lo enseña con «Esc para cancelar».
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct SpaceProgress {
    pub step: u32,
    pub total: u32,
    pub label: String,
    pub waited: Option<u32>,
}

/// Evento: en qué va la colocación de la ventana de un paso ya lanzado. Va en segundo
/// plano (ADR 0014): llega cuando aparece la ventana, también después de otros pasos.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct SpacePlacement {
    pub step: u32,
    pub state: PlacementState,
}

/// El Espacio que se está ejecutando (uno a la vez) y la orden de cancelarlo.
#[derive(Default)]
pub struct SpaceRun {
    running: AtomicBool,
    cancel: AtomicBool,
}

/// Cuenta el progreso a la UI y mira si se ha cancelado.
struct EventProgress<'a> {
    app: &'a AppHandle,
    total: u32,
    cancel: &'a AtomicBool,
}

impl Progress for EventProgress<'_> {
    fn step(&self, index: u32, label: &str, waited: Option<u32>) {
        let _ = SpaceProgress {
            step: index,
            total: self.total,
            label: label.to_owned(),
            waited,
        }
        .emit(self.app);
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    fn placement(&self, index: u32, state: PlacementState) {
        let _ = SpacePlacement { step: index, state }.emit(self.app);
    }
}

#[derive(Debug, Clone, Copy, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PathKind {
    File,
    Folder,
}

#[tauri::command]
#[specta::specta]
pub async fn list_spaces(app: AppHandle) -> Result<Vec<SpaceView>, AppError> {
    let (spaces, opens) = {
        let db = app.state::<Db>();
        let db = db.lock();
        (
            db.list_spaces()?,
            db.space_open_ages(sift_search::frecency::MAX_AGE_DAYS)?,
        )
    };
    let frecency = sift_search::Frecency::from_opens(opens);
    // Los iconos de Windows tardan un poco la primera vez: fuera del hilo de la IPC.
    run_blocking(move || {
        let cache = app.state::<IconCache>();
        Ok(spaces
            .into_iter()
            .map(|space| {
                let auto_icon = match space.draft.icon {
                    Icon::Auto => auto_icon_key(&space.draft).and_then(|k| cache.get_or_load(&k)),
                    _ => None,
                };
                SpaceView {
                    boost: frecency.boost(space.id),
                    space,
                    auto_icon,
                }
            })
            .collect())
    })
    .await
}

/// Clave del icono automático: la primera acción que tenga un icono de Windows.
fn auto_icon_key(draft: &SpaceDraft) -> Option<String> {
    draft.actions.iter().find_map(|a| match a {
        Action::OpenApp { app_id, .. } => Some(shell::app_icon_key(app_id)),
        Action::OpenFile { path, .. } | Action::OpenFolder { path, .. } => {
            Some(path.display().to_string())
        }
        _ => None,
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_space(app: AppHandle, draft: SpaceDraft) -> Result<Space, AppError> {
    Ok(app.state::<Db>().lock().create_space(draft)?)
}

#[tauri::command]
#[specta::specta]
pub fn update_space(app: AppHandle, id: i32, draft: SpaceDraft) -> Result<Space, AppError> {
    Ok(app.state::<Db>().lock().update_space(id.into(), draft)?)
}

#[tauri::command]
#[specta::specta]
pub fn duplicate_space(app: AppHandle, id: i32) -> Result<Space, AppError> {
    Ok(app.state::<Db>().lock().duplicate_space(id.into())?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_space(app: AppHandle, id: i32) -> Result<(), AppError> {
    Ok(app.state::<Db>().lock().delete_space(id.into())?)
}

#[tauri::command]
#[specta::specta]
pub fn reorder_spaces(app: AppHandle, ids: Vec<i32>) -> Result<(), AppError> {
    let ids: Vec<i64> = ids.into_iter().map(i64::from).collect();
    Ok(app.state::<Db>().lock().reorder_spaces(&ids)?)
}

/// Ejecuta un Espacio guardado. Si tiene comandos que nunca se han ejecutado, Rust pide
/// confirmarlos en un diálogo de Windows antes de lanzar nada (ver `run_with_approval`).
#[tauri::command]
#[specta::specta]
pub async fn run_space(app: AppHandle, id: i32) -> Result<RunReport, AppError> {
    let draft = app.state::<Db>().lock().get_space(id.into())?.draft;
    tracing::info!(space = id, "ejecutando Espacio");
    let run = app.state::<SpaceRun>();
    if run.running.swap(true, Ordering::SeqCst) {
        return Err(AppError::Invalid(
            "ya se está abriendo otro Espacio: espera a que acabe o cancélalo".into(),
        ));
    }
    run.cancel.store(false, Ordering::SeqCst);
    let report = run_with_approval(app.clone(), draft.actions, &draft.name, true).await;
    app.state::<SpaceRun>()
        .running
        .store(false, Ordering::SeqCst);
    let report = report?;
    // Cuenta como abierto si se lanzó (aunque alguna acción fallara), no si se canceló la
    // confirmación.
    if matches!(report, RunReport::Done { .. })
        && let Err(e) = app.state::<Db>().lock().record_space_open(id.into())
    {
        tracing::warn!("no se pudo apuntar la apertura del Espacio: {e}");
    }
    Ok(report)
}

/// El botón "probar" del editor: ejecuta una sola acción (aún sin guardar), con la misma
/// regla de confirmación para los comandos.
#[tauri::command]
#[specta::specta]
pub async fn test_action(app: AppHandle, action: Action) -> Result<RunReport, AppError> {
    run_with_approval(app, vec![action], "Esta acción", false).await
}

/// Ejecuta acciones. Si hay comandos que nunca se han ejecutado, primero los enseña en un
/// **diálogo de Windows** y solo sigue si se elige «Ejecutar». La confirmación no la da la
/// interfaz (un `approve: true` desde el WebView): así, nada que corriera dentro de la
/// interfaz podría aprobar comandos por su cuenta (auditoría, A4).
async fn run_with_approval(
    app: AppHandle,
    actions: Vec<Action>,
    owner: &str,
    with_progress: bool,
) -> Result<RunReport, AppError> {
    let pending = {
        let db = app.state::<Db>();
        let db = db.lock();
        // Un error al consultar cuenta como "no aprobado": ante la duda, se pregunta.
        sift_spaces::pending_approvals(&actions, |c| db.is_command_approved(c).unwrap_or(false))
    };
    if !pending.is_empty() {
        let message = sift_spaces::approval_message(owner, &pending);
        let dialog = app.clone();
        let approved = run_blocking(move || {
            use tauri_plugin_dialog::{MessageDialogButtons, MessageDialogKind};
            Ok(dialog
                .dialog()
                .message(message)
                .title("Sift: confirmar comandos")
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Ejecutar".into(),
                    "Cancelar".into(),
                ))
                .blocking_show())
        })
        .await?;
        if !approved {
            tracing::info!("comandos nuevos no confirmados: no se ejecuta nada");
            return Ok(RunReport::Declined);
        }
        let db = app.state::<Db>();
        let mut db = db.lock();
        for command in &pending {
            db.approve_command(command)?;
            tracing::info!(command = %sift_spaces::display_command(command), "comando aprobado");
        }
    }
    // En otro hilo: con esperas puede tardar minutos.
    let outcomes = run_blocking(move || {
        let waiter = crate::waits::LocalWaiter::new();
        let run = app.state::<SpaceRun>();
        let events = EventProgress {
            app: &app,
            total: actions.len() as u32,
            cancel: &run.cancel,
        };
        let progress: &dyn Progress = if with_progress { &events } else { &NoProgress };
        Ok(sift_spaces::run_actions(
            &actions,
            &WindowsOpener,
            &waiter,
            Some(&crate::window_manager::Win32Windows),
            progress,
        ))
    })
    .await?;
    for o in outcomes.iter().filter(|o| o.error.is_some()) {
        tracing::warn!(action = %o.label, error = ?o.error, "acción fallida");
    }
    Ok(RunReport::Done { outcomes })
}

/// Las pantallas conectadas, para elegir dónde colocar una ventana en el editor.
#[tauri::command]
#[specta::specta]
pub async fn list_monitors() -> Result<Vec<sift_spaces::placement::MonitorInfo>, AppError> {
    use sift_spaces::placement::WindowManager;
    run_blocking(|| Ok(crate::window_manager::Win32Windows.monitors())).await
}

/// Cancela el Espacio en marcha (Esc en la barra). Lo ya abierto o lanzado sigue abierto;
/// lo que faltaba no se ejecuta.
#[tauri::command]
#[specta::specta]
pub fn cancel_space_run(app: AppHandle) {
    app.state::<SpaceRun>().cancel.store(true, Ordering::SeqCst);
}

/// «Probar» de una espera: comprueba su condición una vez, ahora, sin ejecutar nada.
/// `Ok` con lo que se ha visto; error con el motivo si no se cumple.
#[tauri::command]
#[specta::specta]
pub async fn check_wait(spec: WaitSpec) -> Result<String, AppError> {
    run_blocking(move || {
        sift_spaces::check_wait_now(&spec, &crate::waits::LocalWaiter::new())
            .map_err(AppError::Invalid)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_apps() -> Result<Vec<InstalledApp>, AppError> {
    run_blocking(|| shell::list_apps().map_err(AppError::System)).await
}

/// Iconos de Windows para varias claves a la vez (`app:<id>` o una ruta).
#[tauri::command]
#[specta::specta]
pub async fn get_icons(app: AppHandle, keys: Vec<String>) -> Result<Vec<Option<String>>, AppError> {
    run_blocking(move || {
        let cache = app.state::<IconCache>();
        Ok(keys.iter().map(|k| cache.get_or_load(k)).collect())
    })
    .await
}

/// Diálogo nativo para elegir un archivo o una carpeta. `None` si se cancela.
#[tauri::command]
#[specta::specta]
pub async fn pick_path(app: AppHandle, kind: PathKind) -> Result<Option<String>, AppError> {
    run_blocking(move || {
        let dialog = app.dialog().file();
        let picked = match kind {
            PathKind::File => dialog.blocking_pick_file(),
            PathKind::Folder => dialog.blocking_pick_folder(),
        };
        Ok(picked
            .and_then(|p| p.into_path().ok())
            .map(|p| p.display().to_string()))
    })
    .await
}

/// Elige una imagen propia para el icono y la devuelve ya como `Icon::Image`.
#[tauri::command]
#[specta::specta]
pub async fn pick_icon_image(app: AppHandle) -> Result<Option<Icon>, AppError> {
    run_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .add_filter("Imagen", &["png", "svg", "ico"])
            .blocking_pick_file()
            .and_then(|p| p.into_path().ok())
        else {
            return Ok(None);
        };
        image_icon(&path).map(Some)
    })
    .await
}

fn image_icon(path: &std::path::Path) -> Result<Icon, AppError> {
    use base64::Engine;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => {
            return Err(AppError::Invalid(
                "la imagen tiene que ser PNG, SVG o ICO".into(),
            ));
        }
    };
    let bytes = std::fs::read(path).map_err(|e| AppError::System(e.to_string()))?;
    let icon = Icon::Image {
        mime: mime.into(),
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
    };
    // Reutiliza la validación del dominio (tipo y tamaño).
    let probe = SpaceDraft {
        name: "x".into(),
        aliases: vec![],
        icon,
        actions: vec![],
    };
    Ok(sift_spaces::validate(probe)
        .map_err(|e| AppError::Invalid(e.to_string()))?
        .icon)
}

/// Exporta todos los Espacios a un JSON que elige el usuario. Devuelve la ruta.
#[tauri::command]
#[specta::specta]
pub async fn export_spaces(app: AppHandle) -> Result<Option<String>, AppError> {
    let drafts: Vec<SpaceDraft> = app
        .state::<Db>()
        .lock()
        .list_spaces()?
        .into_iter()
        .map(|s| s.draft)
        .collect();
    run_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .set_file_name("espacios-sift.json")
            .blocking_save_file()
            .and_then(|p| p.into_path().ok())
        else {
            return Ok(None);
        };
        std::fs::write(&path, sift_spaces::export::to_json(drafts))
            .map_err(|e| AppError::System(e.to_string()))?;
        tracing::info!(path = %path.display(), "Espacios exportados");
        Ok(Some(path.display().to_string()))
    })
    .await
}

/// Importa Espacios de un JSON (se añaden al final). Devuelve cuántos.
#[tauri::command]
#[specta::specta]
pub async fn import_spaces(app: AppHandle) -> Result<Option<u32>, AppError> {
    let handle = app.clone();
    let picked: Option<PathBuf> = run_blocking(move || {
        Ok(handle
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .blocking_pick_file()
            .and_then(|p| p.into_path().ok()))
    })
    .await?;
    let Some(path) = picked else { return Ok(None) };
    let text = std::fs::read_to_string(&path).map_err(|e| AppError::System(e.to_string()))?;
    let drafts =
        sift_spaces::export::from_json(&text).map_err(|e| AppError::Invalid(e.to_string()))?;
    let count = app.state::<Db>().lock().import_spaces(drafts)?;
    tracing::info!(count, path = %path.display(), "Espacios importados");
    Ok(Some(count as u32))
}

/// Ejecuta trabajo bloqueante (COM, diálogos, lanzar procesos) en un hilo aparte para no
/// ocupar los hilos que atienden la IPC.
async fn run_blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::System(format!("tarea interrumpida: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_icon_uses_the_first_action_with_an_icon() {
        let draft = SpaceDraft {
            name: "x".into(),
            aliases: vec![],
            icon: Icon::Auto,
            actions: vec![
                Action::OpenUrl {
                    url: "https://x.es".into(),
                    place: None,
                },
                Action::OpenApp {
                    app_id: "Code".into(),
                    name: "VS Code".into(),
                    place: None,
                },
                Action::OpenFolder {
                    path: PathBuf::from(r"C:\Uni"),
                    place: None,
                },
            ],
        };
        assert_eq!(auto_icon_key(&draft).as_deref(), Some("app:Code"));
    }

    #[test]
    fn image_icons_are_typed_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("a.png");
        std::fs::write(&png, b"\x89PNG").unwrap();
        assert!(matches!(image_icon(&png), Ok(Icon::Image { mime, .. }) if mime == "image/png"));
        let gif = dir.path().join("a.gif");
        std::fs::write(&gif, b"GIF").unwrap();
        assert!(matches!(image_icon(&gif), Err(AppError::Invalid(_))));
    }
}
