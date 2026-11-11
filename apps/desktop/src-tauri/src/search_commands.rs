//! Comandos IPC de la búsqueda de archivos y de Ajustes → Carpetas.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sift_indexer::IgnoreRules;
use sift_search::{Frecency, Lexical, SearchResults};
use sift_spaces::Opener;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::commands::AppError;
use crate::indexing::{IndexStatus, Indexing};
use crate::latency::{LatencyStats, LatencySummary};
use crate::shell::WindowsOpener;
use crate::spaces_commands::Db;

/// Cuántos archivos devuelve la búsqueda (la barra muestra unos 8 a la vez).
const MAX_FILE_HITS: usize = 12;

/// Tiempos de búsqueda en Rust, para Ajustes → Estado.
#[derive(Default)]
pub struct SearchStats(pub Mutex<LatencyStats>);

impl SearchStats {
    pub fn summary(&self) -> Option<LatencySummary> {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).summary()
    }
}

/// Cada cuánto se recalcula la frecencia aunque no abras nada (las aperturas envejecen).
const FRECENCY_TTL: Duration = Duration::from_secs(60 * 60);

/// La frecencia en memoria: se calcula de la tabla `opens` al buscar por primera vez,
/// cuando abres algo y cada hora. Así cada búsqueda no consulta SQLite.
#[derive(Default)]
pub struct FrecencyCache(Mutex<Option<(Instant, Arc<Frecency>)>>);

impl FrecencyCache {
    fn get(&self, app: &AppHandle) -> Arc<Frecency> {
        let mut cached = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((built, f)) = cached.as_ref()
            && built.elapsed() < FRECENCY_TTL
        {
            return f.clone();
        }
        let f = match db(app).file_open_ages(sift_search::frecency::MAX_AGE_DAYS) {
            Ok(opens) => Arc::new(Frecency::from_opens(opens)),
            Err(e) => {
                // Sin frecencia se busca igual: no es motivo para fallar.
                tracing::warn!("no se pudo leer la frecencia: {e}");
                Arc::new(Frecency::default())
            }
        };
        *cached = Some((Instant::now(), f.clone()));
        f
    }

    fn invalidate(&self) {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    /// El número que mandó la UI: si llega una respuesta vieja, la descarta.
    pub seq: u32,
    pub results: SearchResults,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RootView {
    pub id: i32,
    pub path: String,
    pub files: u32,
    pub errors: u32,
    pub last_indexed_at: Option<String>,
    /// La carpeta ya no existe (o el disco no está conectado).
    pub missing: bool,
}

fn db(app: &AppHandle) -> std::sync::MutexGuard<'_, sift_storage::Database> {
    // `state()` vive lo que vive `app`; el guard, lo que dure la llamada.
    app.state::<Db>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// Busca archivos por nombre y contenido. Se mide el tiempo en Rust (objetivo < 50 ms).
#[tauri::command]
#[specta::specta]
pub async fn search_files(
    app: AppHandle,
    query: String,
    seq: u32,
) -> Result<SearchResponse, AppError> {
    let indexing = app.state::<Indexing>();
    let lexical = Lexical(&indexing.index);
    // Con la búsqueda inteligente lista, también por significado (ADR 0009); si no, solo
    // por palabras, como siempre.
    let ready = indexing.smart.ready();
    let semantic = ready.as_ref().map(|r| sift_search::Semantic {
        vectors: &r.vectors,
        text: &indexing.index,
        embedder: r.query.as_ref(),
        min_similarity: r.card.min_similarity,
    });
    let mut retrievers: Vec<&dyn sift_search::Retriever> = vec![&lexical];
    if let Some(semantic) = &semantic {
        retrievers.push(semantic);
    }
    let frecency = app.state::<FrecencyCache>().get(&app);
    let results = sift_search::search_with(
        &indexing.index,
        &retrievers,
        &query,
        MAX_FILE_HITS,
        &frecency,
    )
    .map_err(|e| AppError::System(e.to_string()))?;
    if !query.trim().is_empty() {
        app.state::<SearchStats>()
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .record(std::time::Duration::from_secs_f64(results.took_ms / 1000.0));
    }
    Ok(SearchResponse { seq, results })
}

#[tauri::command]
#[specta::specta]
pub fn list_roots(app: AppHandle) -> Result<Vec<RootView>, AppError> {
    Ok(db(&app)
        .list_roots()?
        .into_iter()
        .map(|r| RootView {
            id: r.id as i32,
            missing: !r.path.is_dir(),
            path: r.path.display().to_string(),
            files: r.files as u32,
            errors: r.errors as u32,
            last_indexed_at: r.last_indexed_at,
        })
        .collect())
}

/// Archivos que no se pudieron leer por un mismo motivo, con unos pocos ejemplos.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnreadGroupView {
    /// `unsupported`, `tooLarge`, `noPermission`, `protected`, `damaged` u `other`.
    pub kind: String,
    pub count: u32,
    pub examples: Vec<String>,
}

/// Cuántos ejemplos se enseñan por motivo: los justos para hacerse una idea.
const UNREAD_EXAMPLES: usize = 3;

/// Los archivos de una carpeta que no se pudieron leer, agrupados por motivo (no se
/// listan todos).
#[tauri::command]
#[specta::specta]
pub fn unread_files(app: AppHandle, root_id: i32) -> Result<Vec<UnreadGroupView>, AppError> {
    Ok(db(&app)
        .unread_summary(i64::from(root_id), UNREAD_EXAMPLES)?
        .into_iter()
        .map(|g| UnreadGroupView {
            // Un código que esta versión no conozca cuenta como «otros».
            kind: sift_extract::UnreadReason::from_code(&g.kind)
                .code()
                .to_owned(),
            count: g.count as u32,
            examples: g.examples,
        })
        .collect())
}

/// Elige una carpeta con el diálogo de Windows y empieza a indexarla.
#[tauri::command]
#[specta::specta]
pub async fn add_root(app: AppHandle) -> Result<Option<String>, AppError> {
    let handle = app.clone();
    let picked: Option<PathBuf> = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
    })
    .await
    .map_err(|e| AppError::System(e.to_string()))?;
    let Some(path) = picked else { return Ok(None) };
    db(&app).add_root(&path)?;
    tracing::info!(root = %path.display(), "carpeta añadida");
    app.state::<Indexing>().sync();
    Ok(Some(path.display().to_string()))
}

/// Deja de indexar una carpeta y borra del índice todo lo suyo (los archivos no se tocan).
#[tauri::command]
#[specta::specta]
pub fn remove_root(app: AppHandle, id: i32) -> Result<(), AppError> {
    let indexing = app.state::<Indexing>();
    indexing.cancel_current();
    let removed = db(&app).remove_root(id.into())?;
    indexing.index.remove_files(&removed);
    indexing
        .index
        .commit_and_reload()
        .map_err(|e| AppError::System(e.to_string()))?;
    tracing::info!(root = id, files = removed.len(), "carpeta quitada");
    indexing.sync();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_ignore_patterns(app: AppHandle) -> Result<Vec<String>, AppError> {
    Ok(db(&app).ignore_patterns()?)
}

/// Guarda los patrones (si son válidos) y resincroniza: lo que ahora se ignora sale del
/// índice y lo que ya no, entra.
#[tauri::command]
#[specta::specta]
pub fn set_ignore_patterns(app: AppHandle, patterns: Vec<String>) -> Result<Vec<String>, AppError> {
    IgnoreRules::new(&patterns).map_err(|e| AppError::Invalid(e.to_string()))?;
    let saved = db(&app).set_ignore_patterns(&patterns)?;
    let indexing = app.state::<Indexing>();
    indexing.cancel_current();
    indexing.sync();
    Ok(saved)
}

/// ¿Está activada la búsqueda inteligente (por significado)?
#[tauri::command]
#[specta::specta]
pub fn get_smart_search(app: AppHandle) -> bool {
    !crate::load_settings(&app).indexing.disable_smart_search
}

/// Activa o desactiva la búsqueda inteligente. Desactivarla libera el modelo de memoria;
/// los vectores ya calculados se quedan en disco por si se vuelve a activar.
#[tauri::command]
#[specta::specta]
pub fn set_smart_search(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    crate::update_settings(&app, |s| s.indexing.disable_smart_search = !enabled)
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    app.state::<Indexing>().smart_changed();
    Ok(enabled)
}

/// ¿Se buscan imágenes y vídeos por su nombre? (Ajustes → Carpetas → Avanzado)
#[tauri::command]
#[specta::specta]
pub fn get_index_media(app: AppHandle) -> bool {
    !crate::load_settings(&app).indexing.skip_media
}

/// Activa o desactiva buscar imágenes y vídeos por nombre, y revisa las carpetas: los
/// que sobran salen del índice y los que faltan entran.
#[tauri::command]
#[specta::specta]
pub fn set_index_media(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    crate::update_settings(&app, |s| s.indexing.skip_media = !enabled)
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    let indexing = app.state::<Indexing>();
    indexing.cancel_current();
    indexing.sync();
    Ok(enabled)
}

#[tauri::command]
#[specta::specta]
pub fn index_status(app: AppHandle) -> IndexStatus {
    app.state::<Indexing>().status()
}

#[tauri::command]
#[specta::specta]
pub fn reindex(app: AppHandle) {
    app.state::<Indexing>().sync();
}

/// Abre un archivo encontrado. Solo rutas que estén en el índice: la UI no puede pedir
/// abrir cualquier cosa (abrir un .exe sería ejecutarlo).
#[tauri::command]
#[specta::specta]
pub async fn open_file(app: AppHandle, path: String) -> Result<(), AppError> {
    let path = PathBuf::from(path);
    if !db(&app).is_indexed(&path)? {
        return Err(AppError::NotFound(
            "ese archivo no está en el índice".into(),
        ));
    }
    if let Err(e) = db(&app).record_file_open(&path) {
        tracing::warn!("no se pudo apuntar la apertura: {e}");
    }
    app.state::<FrecencyCache>().invalidate();
    tauri::async_runtime::spawn_blocking(move || WindowsOpener.open_path(&path))
        .await
        .map_err(|e| AppError::System(e.to_string()))?
        .map_err(AppError::System)
}

/// Abre la carpeta que contiene el archivo, con él seleccionado (como «Abrir ubicación»).
#[tauri::command]
#[specta::specta]
pub fn reveal_file(app: AppHandle, path: String) -> Result<(), AppError> {
    let path = PathBuf::from(path);
    if !db(&app).is_indexed(&path)? {
        return Err(AppError::NotFound(
            "ese archivo no está en el índice".into(),
        ));
    }
    crate::shell::reveal_in_explorer(&path).map_err(AppError::System)
}
