//! La versión de desarrollo (`npm run tauri dev`) vive aparte de la de uso diario.
//!
//! Toda compilación de depuración usa **otro identificador** de app
//! (`dev.sift.app-dev` en vez de `dev.sift.app`). De él salen:
//! - la carpeta de datos: base de datos, índices, modelo y ajustes
//!   (`%APPDATA%\dev.sift.app-dev`);
//! - los logs (`%LOCALAPPDATA%\dev.sift.app-dev\logs`);
//! - el candado de «una sola instancia»: con el mismo identificador, la de desarrollo le
//!   pasaría el control a la de verdad y se cerraría;
//! - la entrada de «arrancar con Windows» y los datos del WebView.
//!
//! Así, las pruebas y las migraciones de desarrollo nunca tocan los datos reales. No hay
//! forma de saltarse esto en una compilación de depuración: es a propósito.
//!
//! La primera vez, la de desarrollo se rellena con datos de prueba: la carpeta `docs/` del
//! propio repositorio (nada personal) y un Espacio de ejemplo. Su atajo es Ctrl+Alt+Space,
//! para no chocar con el de la versión diaria (Alt+Space) si están abiertas las dos.
//!
//! Con `SIFT_DEV_DEMO=1`, en vez de eso, los **datos de demostración** (`demo/` del repo,
//! inventados) y unos Espacios de ejemplo: para capturas y vídeos sin nada personal. Con
//! `SIFT_DEV_DEMO=en`, los mismos en inglés (`demo-en/`), para el vídeo y las capturas en
//! inglés (la interfaz sigue en español).

use std::path::{Path, PathBuf};

use sift_core::Hotkey;
use sift_spaces::placement::{MonitorChoice, Placement, Zone};
use sift_spaces::{Action, Icon, OnTimeout, SpaceDraft, WaitSpec, WaitUntil};

pub const IDENTIFIER: &str = "dev.sift.app-dev";
pub const PRODUCT_NAME: &str = "Sift (desarrollo)";
pub const HOTKEY: &str = "Ctrl+Alt+Space";
/// Arrancar la versión de desarrollo sin datos de prueba (ver [`seed`]).
const NO_SEED_ENV: &str = "SIFT_DEV_NO_SEED";
/// Arrancar la versión de desarrollo con los datos de demostración (ver [`demo_folder`]).
const DEMO_ENV: &str = "SIFT_DEV_DEMO";
/// Arrancar la versión de desarrollo leyendo solo esta carpeta (las pruebas de aguante,
/// `scripts/aguante/`, con sus archivos generados). Solo en desarrollo.
const ROOT_ENV: &str = "SIFT_DEV_ROOT";

/// Los datos de demostración: en español (`SIFT_DEV_DEMO=1`) o en inglés (`SIFT_DEV_DEMO=en`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemoLang {
    Es,
    En,
}

impl DemoLang {
    fn from_env() -> Option<Self> {
        match std::env::var(DEMO_ENV).as_deref() {
            Ok("1") => Some(Self::Es),
            Ok("en") => Some(Self::En),
            _ => None,
        }
    }

    /// La carpeta del repo con los archivos.
    fn source(self) -> &'static str {
        match self {
            Self::Es => "demo",
            Self::En => "demo-en",
        }
    }

    /// El nombre de su copia en Documentos públicos.
    fn folder(self) -> &'static str {
        match self {
            Self::Es => "Sift (demostración)",
            Self::En => "Sift (demo)",
        }
    }
}

fn env_on(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1")
}

/// Los datos de demostración, copiados de `demo/` (o `demo-en/`) del repo a
/// `C:\Users\Public\Documents\Sift (demostración)` (o `Sift (demo)`): una ruta sin nombre de
/// usuario, para que en capturas y vídeos no salga nada personal. Con `fresh`, la copia se
/// rehace desde cero (los datos vuelven a estar como en el repo). Solo en desarrollo con
/// `SIFT_DEV_DEMO=1` o `SIFT_DEV_DEMO=en`; si no, `None`.
pub fn demo_folder(fresh: bool) -> Option<PathBuf> {
    if !is_dev() {
        return None;
    }
    let lang = DemoLang::from_env()?;
    let target = PathBuf::from(std::env::var_os("PUBLIC")?)
        .join("Documents")
        .join(lang.folder());
    if fresh && target.exists() {
        std::fs::remove_dir_all(&target).ok()?;
    }
    if !target.exists() {
        copy_dir(&repo_root().join(lang.source()), &target).ok()?;
    }
    Some(target)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// Los Espacios de ejemplo de los datos de demostración: uno con ventanas colocadas y una
/// espera (para enseñar sus pasos) y otro sencillo.
fn demo_spaces(demo: &Path, lang: DemoLang) -> Vec<SpaceDraft> {
    let place = |zone| {
        Some(Placement {
            zone: Some(zone),
            monitor: MonitorChoice::Primary,
            reuse_window: true,
        })
    };
    let en = lang == DemoLang::En;
    let pick = |es: &str, en_text: &str| if en { en_text } else { es }.to_owned();
    let (shop, db) = if en {
        (
            demo.join("Projects").join("Shop website"),
            demo.join("Studies").join("Databases"),
        )
    } else {
        (
            demo.join("Proyectos").join("Web de la tienda"),
            demo.join("Estudios").join("Bases de datos"),
        )
    };
    vec![
        SpaceDraft {
            name: pick("Tienda de Lucía", "Lucy's shop"),
            aliases: vec![pick("tienda", "shop"), pick("cerámica", "ceramics")],
            icon: Icon::Auto,
            actions: vec![
                // Solo el Explorador y el Bloc de notas (un `.txt`): se puede abrir en una
                // grabación sin que salga ninguna app ni cuenta de quien graba.
                Action::OpenFolder {
                    path: shop.clone(),
                    place: place(Zone::LeftHalf),
                },
                Action::Wait(WaitSpec {
                    until: WaitUntil::Seconds { secs: 1 },
                    timeout_secs: 60,
                    on_timeout: OnTimeout::Stop,
                }),
                Action::OpenFile {
                    path: shop.join(pick("presupuesto.txt", "quote.txt")),
                    place: place(Zone::RightHalf),
                },
            ],
        },
        SpaceDraft {
            name: pick("Estudiar bases de datos", "Study databases"),
            aliases: vec![pick("bd", "db")],
            icon: Icon::Auto,
            actions: vec![
                Action::OpenFolder {
                    path: db.clone(),
                    place: place(Zone::LeftHalf),
                },
                Action::OpenFile {
                    path: db.join(pick("apuntes-transacciones.pdf", "transactions-notes.pdf")),
                    place: place(Zone::RightHalf),
                },
            ],
        },
    ]
}

/// Con `SIFT_DEV_DEMO=1` (y sin `SIFT_DEV_NO_SEED`): los datos de demostración, recién
/// copiados, y sus Espacios. Devuelve `false` si no tocaba.
fn seed_demo(db: &mut sift_storage::Database) -> bool {
    let (Some(lang), Some(demo)) = (DemoLang::from_env(), demo_folder(true)) else {
        return false;
    };
    if let Err(e) = db.add_root(&demo) {
        tracing::warn!("datos de demostración: no se pudo añadir la carpeta: {e}");
    }
    for draft in demo_spaces(&demo, lang) {
        if let Err(e) = db.create_space(draft) {
            tracing::warn!("datos de demostración: no se pudo crear un Espacio: {e}");
        }
    }
    tracing::info!(path = %demo.display(), "versión de desarrollo: datos de demostración");
    true
}

/// ¿Es la versión de desarrollo? Toda compilación de depuración lo es.
pub const fn is_dev() -> bool {
    cfg!(debug_assertions)
}

/// En desarrollo, cambia el identificador y el nombre antes de arrancar Tauri.
pub fn apply<R: tauri::Runtime>(context: &mut tauri::Context<R>) {
    if is_dev() {
        let config = context.config_mut();
        config.identifier = IDENTIFIER.to_owned();
        config.product_name = Some(PRODUCT_NAME.to_owned());
    }
}

/// El atajo por defecto: en desarrollo, uno que no choque con la versión diaria.
pub fn default_hotkey() -> Hotkey {
    if is_dev() {
        HOTKEY.parse().unwrap_or_default()
    } else {
        Hotkey::default()
    }
}

/// La raíz del repositorio (solo tiene sentido en desarrollo: la ruta es la de compilación).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

/// La carpeta sugerida en la introducción, en una versión de desarrollo arrancada **sin
/// datos de prueba** (`SIFT_DEV_NO_SEED=1`): en vez de tus Documentos de verdad, la
/// carpeta `docs/` del repositorio (nada personal), y ningún Escritorio. Así se puede
/// recorrer la introducción entera, y hacer sus capturas, sin leer archivos personales.
/// `None`: no hay nada que cambiar (la versión diaria, o desarrollo con datos de prueba).
pub fn suggested_folder(documents: bool) -> Option<Option<PathBuf>> {
    if !is_dev() || !env_on(NO_SEED_ENV) {
        return None;
    }
    // Con los datos de demostración, la carpeta sugerida es su copia.
    if DemoLang::from_env().is_some() {
        return Some(documents.then(|| demo_folder(false)).flatten());
    }
    Some(documents.then(|| repo_root().join("docs")).and_then(|p| {
        let p = p.canonicalize().ok()?;
        Some(PathBuf::from(
            p.display()
                .to_string()
                .trim_start_matches(r"\\?\")
                .to_owned(),
        ))
    }))
}

/// Datos de prueba para una base de datos de desarrollo vacía: la carpeta `docs/` del
/// repositorio y un Espacio de ejemplo. Si ya hay algo, no toca nada.
///
/// Con `SIFT_DEV_NO_SEED=1` no se rellena: así se puede ver la bienvenida, que solo sale
/// sin datos (lo usa `scripts/capturas-final.mjs`). Solo en desarrollo.
pub fn seed(db: &mut sift_storage::Database) {
    if !is_dev() || env_on(NO_SEED_ENV) {
        return;
    }
    let empty =
        db.list_roots().is_ok_and(|r| r.is_empty()) && db.list_spaces().is_ok_and(|s| s.is_empty());
    if !empty || seed_demo(db) {
        return;
    }
    if let Some(root) = std::env::var_os(ROOT_ENV).map(PathBuf::from) {
        if let Err(e) = db.add_root(&root) {
            tracing::warn!("pruebas de aguante: no se pudo añadir la carpeta: {e}");
        }
        return;
    }
    let repo = repo_root();
    let docs = repo.join("docs");
    let Ok(docs) = docs.canonicalize() else {
        return;
    };
    let repo = repo.canonicalize().unwrap_or(repo);
    // `canonicalize` en Windows da `\\?\C:\…`: se quita para que la ruta se vea normal.
    let plain = |p: &Path| {
        PathBuf::from(
            p.display()
                .to_string()
                .trim_start_matches(r"\\?\")
                .to_owned(),
        )
    };
    if let Err(e) = db.add_root(&plain(&docs)) {
        tracing::warn!("datos de prueba: no se pudo añadir docs/: {e}");
    }
    let draft = SpaceDraft {
        name: "Sift (desarrollo)".into(),
        aliases: vec!["dev".into()],
        icon: Icon::Auto,
        actions: vec![Action::OpenFolder {
            path: plain(&repo),
            place: None,
        }],
    };
    if let Err(e) = db.create_space(draft) {
        tracing::warn!("datos de prueba: no se pudo crear el Espacio: {e}");
    }
    tracing::info!("versión de desarrollo: datos de prueba creados");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_are_a_debug_build_so_they_get_the_dev_setup() {
        assert!(is_dev());
        assert_eq!(default_hotkey().to_string(), "Ctrl+Alt+Space");
    }

    #[test]
    fn demo_spaces_are_valid_and_open_files_that_exist() {
        for lang in [DemoLang::Es, DemoLang::En] {
            let demo = repo_root().join(lang.source());
            let spaces = demo_spaces(&demo, lang);
            assert_eq!(spaces.len(), 2);
            for draft in spaces {
                let space = sift_spaces::validate(draft).unwrap();
                for action in &space.actions {
                    if let Action::OpenFolder { path, .. } | Action::OpenFile { path, .. } = action {
                        assert!(path.exists(), "no existe: {}", path.display());
                    }
                }
            }
        }
        assert_eq!(demo_spaces(Path::new("x"), DemoLang::En)[0].name, "Lucy's shop");
    }

    #[test]
    fn seeds_only_an_empty_database() {
        let mut db = sift_storage::Database::open_in_memory().unwrap();
        seed(&mut db);
        let roots = db.list_roots().unwrap();
        assert_eq!(roots.len(), 1);
        assert!(roots[0].path.ends_with("docs"));
        assert!(!roots[0].path.display().to_string().starts_with(r"\\?\"));
        assert_eq!(db.list_spaces().unwrap().len(), 1);
        // Una segunda vez no duplica nada.
        seed(&mut db);
        assert_eq!(db.list_roots().unwrap().len(), 1);
        assert_eq!(db.list_spaces().unwrap().len(), 1);
    }
}
