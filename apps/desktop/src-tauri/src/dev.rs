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
//! inventados) y unos Espacios de ejemplo: para capturas y vídeos sin nada personal.

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
/// Nombre de la copia de `demo/` en Documentos públicos.
const DEMO_FOLDER: &str = "Sift (demostración)";

fn env_on(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1")
}

/// Los datos de demostración, copiados de `demo/` del repo a
/// `C:\Users\Public\Documents\Sift (demostración)`: una ruta sin nombre de usuario, para que
/// en capturas y vídeos no salga nada personal. Con `fresh`, la copia se rehace desde cero
/// (los datos vuelven a estar como en el repo). Solo en desarrollo con `SIFT_DEV_DEMO=1`;
/// si no, `None`.
pub fn demo_folder(fresh: bool) -> Option<PathBuf> {
    if !is_dev() || !env_on(DEMO_ENV) {
        return None;
    }
    let target = PathBuf::from(std::env::var_os("PUBLIC")?)
        .join("Documents")
        .join(DEMO_FOLDER);
    if fresh && target.exists() {
        std::fs::remove_dir_all(&target).ok()?;
    }
    if !target.exists() {
        copy_dir(&repo_root().join("demo"), &target).ok()?;
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
fn demo_spaces(demo: &Path) -> Vec<SpaceDraft> {
    let place = |zone| {
        Some(Placement {
            zone: Some(zone),
            monitor: MonitorChoice::Primary,
            reuse_window: true,
        })
    };
    let shop = demo.join("Proyectos").join("Web de la tienda");
    let db = demo.join("Estudios").join("Bases de datos");
    vec![
        SpaceDraft {
            name: "Tienda de Lucía".into(),
            aliases: vec!["tienda".into(), "cerámica".into()],
            icon: Icon::Auto,
            actions: vec![
                Action::OpenFolder {
                    path: shop.clone(),
                    place: place(Zone::LeftHalf),
                },
                Action::OpenFile {
                    path: shop.join("notas-reunion-cliente.md"),
                    place: place(Zone::RightHalf),
                },
                Action::Wait(WaitSpec {
                    until: WaitUntil::Seconds { secs: 2 },
                    timeout_secs: 60,
                    on_timeout: OnTimeout::Stop,
                }),
                Action::OpenUrl {
                    url: "https://example.com".into(),
                    place: None,
                },
            ],
        },
        SpaceDraft {
            name: "Estudiar bases de datos".into(),
            aliases: vec!["bd".into()],
            icon: Icon::Auto,
            actions: vec![
                Action::OpenFolder {
                    path: db.clone(),
                    place: place(Zone::LeftHalf),
                },
                Action::OpenFile {
                    path: db.join("apuntes-transacciones.pdf"),
                    place: place(Zone::RightHalf),
                },
            ],
        },
    ]
}

/// Con `SIFT_DEV_DEMO=1` (y sin `SIFT_DEV_NO_SEED`): los datos de demostración, recién
/// copiados, y sus Espacios. Devuelve `false` si no tocaba.
fn seed_demo(db: &mut sift_storage::Database) -> bool {
    let Some(demo) = demo_folder(true) else {
        return false;
    };
    if let Err(e) = db.add_root(&demo) {
        tracing::warn!("datos de demostración: no se pudo añadir la carpeta: {e}");
    }
    for draft in demo_spaces(&demo) {
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
    if env_on(DEMO_ENV) {
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
        let demo = repo_root().join("demo");
        let spaces = demo_spaces(&demo);
        assert_eq!(spaces.len(), 2);
        for draft in spaces {
            let space = sift_spaces::validate(draft).unwrap();
            for action in &space.actions {
                if let Action::OpenFolder { path, .. } | Action::OpenFile { path, .. } = action {
                    assert!(path.exists(), "no existe en demo/: {}", path.display());
                }
            }
        }
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
