//! Comandos IPC de la búsqueda de archivos y de Ajustes → Carpetas.
//!
//! Los que leen o escriben la base de datos son `#[tauri::command(async)]`: si fueran
//! síncronos correrían en el hilo principal, que es el que atiende el atajo, y esperarían al
//! candado de la base de datos mientras el indexador lo tiene (auditoría 1.0).

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
    filter: Option<String>,
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
    // Filtro rápido de la barra («pdf …»): un id que no existe se ignora.
    let exts = filter
        .as_deref()
        .and_then(sift_extract::type_filter)
        .unwrap_or(&[]);
    let mut results = sift_search::search_with(
        &indexing.index,
        &retrievers,
        &query,
        MAX_FILE_HITS,
        &frecency,
        exts,
    )
    .map_err(|e| AppError::System(e.to_string()))?;
    // La página de los PDF, para enseñarla («pág. 37») y abrir ahí. Son pocos: una
    // consulta por resultado a la clave primaria.
    let pdf_chunks: Vec<i64> = results
        .hits
        .iter()
        .filter(|h| h.ext.eq_ignore_ascii_case("pdf"))
        .map(|h| h.chunk_id)
        .collect();
    if !pdf_chunks.is_empty() {
        match db(&app).chunk_pages(&pdf_chunks) {
            Ok(pages) => {
                for hit in &mut results.hits {
                    hit.page = pages.get(&hit.chunk_id).map(|p| *p as u32);
                }
            }
            Err(e) => tracing::warn!("no se pudieron leer las páginas: {e}"),
        }
    }
    if !query.trim().is_empty() {
        app.state::<SearchStats>()
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .record(std::time::Duration::from_secs_f64(results.took_ms / 1000.0));
    }
    Ok(SearchResponse { seq, results })
}

/// Caracteres y líneas del fragmento ampliado de la vista previa.
const PREVIEW_CHARS: usize = 900;
const PREVIEW_LINES: usize = 12;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    /// El fragmento ampliado, con lo que encaja marcado (vacío si el archivo no tiene texto).
    pub text: Vec<sift_search::Segment>,
    /// Código: se enseña en fuente fija y con sus saltos de línea.
    pub monospace: bool,
    pub page: Option<u32>,
}

/// La vista previa de un resultado de la barra: un fragmento más largo que el de la
/// lista, sacado de SQLite (no se lee el archivo). Solo se pide con la vista previa abierta.
#[tauri::command(async)]
#[specta::specta]
pub fn preview_file(
    app: AppHandle,
    file_id: i32,
    chunk_id: i32,
    query: String,
) -> Result<FilePreview, AppError> {
    // El guard de la base de datos se suelta al acabar esta línea: resaltar no la bloquea.
    let chunk = db(&app).preview_chunk(file_id.into(), chunk_id.into())?;
    let chunk = chunk.ok_or_else(|| {
        AppError::NotFound(
            "Sift ya no tiene este archivo: puede que se haya movido o borrado.".into(),
        )
    })?;
    Ok(build_preview(&chunk, &query))
}

/// Lo que enseña la vista previa de un fragmento: el texto con lo que encaja marcado, si va
/// en fuente fija (código) y la página (solo en los PDF). Aparte del comando para probarlo.
fn build_preview(chunk: &sift_storage::PreviewChunk, query: &str) -> FilePreview {
    let ext = chunk.ext.to_lowercase();
    let monospace =
        sift_extract::type_filter("code").is_some_and(|code| code.contains(&ext.as_str()));
    // Se conservan los saltos de línea (párrafos, listas, tablas de Markdown) salvo en los
    // PDF: su texto viene cortado por la maquetación de la página y se lee mejor seguido.
    let keep_lines = ext != "pdf";
    let text = if chunk.text.is_empty() {
        vec![]
    } else {
        sift_search::snippet::preview(
            &chunk.text,
            &sift_search::ParsedQuery::parse(query),
            PREVIEW_CHARS,
            PREVIEW_LINES,
            keep_lines,
        )
    };
    let page = if ext == "pdf" {
        chunk.page.and_then(|p| u32::try_from(p).ok())
    } else {
        None
    };
    FilePreview {
        text,
        monospace,
        page,
    }
}

#[tauri::command(async)]
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
#[tauri::command(async)]
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
    add_root_path(&app, &path)?;
    Ok(Some(path.display().to_string()))
}

/// Añade una carpeta a las que se indexan y pide una revisión. La comparten «Añadir
/// carpeta» y las carpetas sugeridas de la bienvenida.
fn add_root_path(app: &AppHandle, path: &std::path::Path) -> Result<(), AppError> {
    db(app).add_root(path)?;
    tracing::info!(root = %path.display(), "carpeta añadida");
    app.state::<Indexing>().sync();
    Ok(())
}

/// Carpetas que la bienvenida propone (la UI nunca manda rutas: solo elige entre estas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum SuggestedKind {
    Documents,
    Desktop,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedFolder {
    pub kind: SuggestedKind,
    pub label: String,
    pub path: String,
    /// Ya se indexa (la carpeta, o una que la contiene).
    pub added: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WelcomeView {
    pub done: bool,
    /// El atajo real configurado, p. ej. «Alt+Space».
    pub hotkey: String,
    pub suggested: Vec<SuggestedFolder>,
}

fn suggested_path(app: &AppHandle, kind: SuggestedKind) -> Option<PathBuf> {
    if let Some(dev) = crate::dev::suggested_folder(matches!(kind, SuggestedKind::Documents)) {
        return dev;
    }
    let dir = match kind {
        SuggestedKind::Documents => app.path().document_dir(),
        SuggestedKind::Desktop => app.path().desktop_dir(),
    };
    dir.ok().filter(|p| p.is_dir())
}

/// ¿`path` ya está cubierta por alguna de las carpetas indexadas? Sin distinguir
/// mayúsculas, como Windows.
fn is_covered(path: &std::path::Path, roots: &[PathBuf]) -> bool {
    let norm = |p: &std::path::Path| {
        let s = p.display().to_string().replace('/', "\\").to_lowercase();
        format!("{}\\", s.trim_end_matches('\\'))
    };
    let target = norm(path);
    roots.iter().any(|r| target.starts_with(&norm(r)))
}

/// Datos de la bienvenida: si ya se vio, el atajo y las carpetas sugeridas.
#[tauri::command(async)]
#[specta::specta]
pub fn get_welcome(app: AppHandle) -> Result<WelcomeView, AppError> {
    let settings = crate::load_settings(&app);
    let roots: Vec<PathBuf> = db(&app).list_roots()?.into_iter().map(|r| r.path).collect();
    let suggested = [
        (SuggestedKind::Documents, "Documentos"),
        (SuggestedKind::Desktop, "Escritorio"),
    ]
    .into_iter()
    .filter_map(|(kind, label)| {
        let path = suggested_path(&app, kind)?;
        Some(SuggestedFolder {
            kind,
            label: label.to_owned(),
            added: is_covered(&path, &roots),
            path: path.display().to_string(),
        })
    })
    .collect();
    Ok(WelcomeView {
        done: settings.welcome_done,
        hotkey: settings.hotkey.to_string(),
        suggested,
    })
}

/// Añade Documentos o Escritorio (la bienvenida).
#[tauri::command(async)]
#[specta::specta]
pub fn add_suggested_folder(app: AppHandle, kind: SuggestedKind) -> Result<(), AppError> {
    let path = suggested_path(&app, kind)
        .ok_or_else(|| AppError::NotFound("la carpeta no existe en este PC".into()))?;
    add_root_path(&app, &path)
}

/// Termina la bienvenida. Con `open_launcher`, cierra Ajustes y muestra la barra.
#[tauri::command]
#[specta::specta]
pub fn finish_welcome(app: AppHandle, open_launcher: bool) -> Result<(), AppError> {
    crate::update_settings(&app, |s| s.welcome_done = true)
        .map_err(|e| AppError::System(format!("{e:#}")))?;
    if open_launcher {
        let handle = app.clone();
        app.run_on_main_thread(move || {
            if let Some(window) = handle.get_webview_window(crate::settings_window::LABEL)
                && let Err(e) = window.close()
            {
                tracing::warn!(error = %e, "no se pudo cerrar la ventana de ajustes");
            }
            crate::launcher::show(&handle);
        })
        .map_err(|e| AppError::System(e.to_string()))?;
    }
    Ok(())
}

/// El usuario acepta descargar el modelo de búsqueda inteligente (unos 120 MB).
#[tauri::command(async)]
#[specta::specta]
pub fn allow_model_download(app: AppHandle) -> Result<(), AppError> {
    crate::update_settings(&app, |s| {
        s.indexing.model_download_allowed = true;
        s.indexing.disable_smart_search = false;
    })
    .map_err(|e| AppError::System(format!("{e:#}")))?;
    app.state::<Indexing>().smart_changed();
    Ok(())
}

/// Deja de indexar una carpeta y borra del índice todo lo suyo (los archivos no se tocan).
#[tauri::command(async)]
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

#[tauri::command(async)]
#[specta::specta]
pub fn get_ignore_patterns(app: AppHandle) -> Result<Vec<String>, AppError> {
    Ok(db(&app).ignore_patterns()?)
}

/// Guarda los patrones (si son válidos) y resincroniza: lo que ahora se ignora sale del
/// índice y lo que ya no, entra.
#[tauri::command(async)]
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

/// Un archivo abierto hace poco desde Sift (para la barra vacía).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub file_id: i32,
    pub path: String,
    pub name: String,
    pub ext: String,
}

/// «Abierto recientemente»: lo último que abriste desde Sift. Se pide al abrir la barra,
/// después de pintarla, así que no cuenta en los < 100 ms.
#[tauri::command(async)]
#[specta::specta]
pub fn recent_files(app: AppHandle) -> Result<Vec<RecentFile>, AppError> {
    /// Los que caben en la barra con los Espacios.
    const LIMIT: usize = 8;
    Ok(db(&app)
        .recent_files(LIMIT)?
        .into_iter()
        .map(|(id, path, name, ext)| RecentFile {
            file_id: id as i32,
            path,
            name,
            ext,
        })
        .collect())
}

/// Rutas del índice que encajan con lo que se escribe en el editor de acciones (por
/// nombre; ver `sift_search::paths`). Lo que más abres va primero.
#[tauri::command]
#[specta::specta]
pub async fn suggest_paths(
    app: AppHandle,
    query: String,
    kind: crate::spaces_commands::PathKind,
) -> Result<Vec<String>, AppError> {
    /// Las que caben en la lista sin desplazarse.
    const LIMIT: usize = 8;
    let files = db(&app).file_paths()?;
    let frecency = app.state::<FrecencyCache>().get(&app);
    let kind = match kind {
        crate::spaces_commands::PathKind::File => sift_search::paths::PathKind::File,
        crate::spaces_commands::PathKind::Folder => sift_search::paths::PathKind::Folder,
    };
    tauri::async_runtime::spawn_blocking(move || {
        sift_search::paths::suggest_paths(&files, &query, kind, &frecency, LIMIT)
    })
    .await
    .map_err(|e| AppError::System(e.to_string()))
}

/// Abre un archivo encontrado. Solo rutas que estén en el índice: la UI no puede pedir
/// abrir cualquier cosa (abrir un .exe sería ejecutarlo).
#[tauri::command]
#[specta::specta]
pub async fn open_file(app: AppHandle, path: String, page: Option<u32>) -> Result<(), AppError> {
    let path = PathBuf::from(path);
    if !db(&app).is_indexed(&path)? {
        return Err(AppError::NotFound(
            "Sift ya no tiene este archivo: puede que se haya movido o borrado.".into(),
        ));
    }
    if let Err(e) = db(&app).record_file_open(&path) {
        tracing::warn!("no se pudo apuntar la apertura: {e}");
    }
    app.state::<FrecencyCache>().invalidate();
    // Un script o un acceso directo indexado (un `.bat`, un `.js`…) se **ejecutaría** al
    // abrirlo: en vez de eso se enseña en su carpeta, seleccionado (auditoría 1.0). Para
    // ejecutar algo está la acción «comando» de los Espacios, que pide confirmación.
    if sift_spaces::is_executable(&path) {
        return crate::shell::reveal_in_explorer(&path).map_err(AppError::System);
    }
    // Un PDF con página, en esa página si el lector lo permite (si no, normal).
    let is_pdf = sift_extract::extension(&path) == "pdf";
    tauri::async_runtime::spawn_blocking(move || match page {
        Some(page) if is_pdf => crate::pdf_page::open_pdf_at(&path, page),
        _ => WindowsOpener.open_path(&path),
    })
    .await
    .map_err(|e| AppError::System(e.to_string()))?
    .map_err(AppError::System)
}

/// Abre la carpeta que contiene el archivo, con él seleccionado (como «Abrir ubicación»).
#[tauri::command(async)]
#[specta::specta]
pub fn reveal_file(app: AppHandle, path: String) -> Result<(), AppError> {
    let path = PathBuf::from(path);
    if !db(&app).is_indexed(&path)? {
        return Err(AppError::NotFound(
            "Sift ya no tiene este archivo: puede que se haya movido o borrado.".into(),
        ));
    }
    crate::shell::reveal_in_explorer(&path).map_err(AppError::System)
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    fn chunk(ext: &str, text: &str, page: Option<i64>) -> sift_storage::PreviewChunk {
        sift_storage::PreviewChunk {
            ext: ext.into(),
            text: text.into(),
            page,
        }
    }

    fn plain(p: &FilePreview) -> String {
        p.text.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn a_pdf_keeps_its_page_and_joins_the_lines_cut_by_the_layout() {
        let p = build_preview(&chunk("PDF", "la clave\nprimaria", Some(37)), "primaria");
        assert_eq!(p.page, Some(37));
        assert!(!p.monospace);
        assert!(!plain(&p).contains('\n'), "{:?}", plain(&p));
        assert!(p.text.iter().any(|s| s.hit && s.text.contains("primaria")));
    }

    #[test]
    fn other_files_keep_line_breaks_and_have_no_page() {
        let p = build_preview(&chunk("md", "- uno\n- dos", Some(3)), "dos");
        assert_eq!(p.page, None, "la página solo tiene sentido en un PDF");
        assert!(plain(&p).contains('\n'));
    }

    #[test]
    fn code_goes_in_a_fixed_width_font() {
        assert!(build_preview(&chunk("rs", "fn main() {}", None), "main").monospace);
        assert!(!build_preview(&chunk("txt", "hola", None), "hola").monospace);
    }

    #[test]
    fn a_file_without_text_has_an_empty_preview() {
        let p = build_preview(&chunk("jpg", "", None), "foto");
        assert!(p.text.is_empty());
    }
}

#[cfg(test)]
mod welcome_tests {
    use super::*;

    #[test]
    fn a_folder_is_covered_by_itself_or_by_a_parent_ignoring_case() {
        let roots = vec![PathBuf::from(r"C:\Users\Ana\Documents")];
        assert!(is_covered(
            std::path::Path::new(r"c:\users\ana\documents"),
            &roots
        ));
        assert!(is_covered(
            std::path::Path::new(r"C:\Users\Ana\Documents\Uni"),
            &roots
        ));
        assert!(!is_covered(
            std::path::Path::new(r"C:\Users\Ana\Desktop"),
            &roots
        ));
        assert!(!is_covered(
            std::path::Path::new(r"C:\Users\Ana\Documents2"),
            &roots
        ));
    }
}
