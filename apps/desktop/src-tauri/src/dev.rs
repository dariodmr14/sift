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

use std::path::{Path, PathBuf};

use sift_core::Hotkey;
use sift_spaces::{Action, Icon, SpaceDraft};

pub const IDENTIFIER: &str = "dev.sift.app-dev";
pub const PRODUCT_NAME: &str = "Sift (desarrollo)";
pub const HOTKEY: &str = "Ctrl+Alt+Space";
/// Arrancar la versión de desarrollo sin datos de prueba (ver [`seed`]).
const NO_SEED_ENV: &str = "SIFT_DEV_NO_SEED";

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

/// Datos de prueba para una base de datos de desarrollo vacía: la carpeta `docs/` del
/// repositorio y un Espacio de ejemplo. Si ya hay algo, no toca nada.
///
/// Con `SIFT_DEV_NO_SEED=1` no se rellena: así se puede ver la bienvenida, que solo sale
/// sin datos (lo usa `scripts/capturas-final.mjs`). Solo en desarrollo.
pub fn seed(db: &mut sift_storage::Database) {
    if !is_dev() || std::env::var(NO_SEED_ENV).is_ok_and(|v| v == "1") {
        return;
    }
    let empty =
        db.list_roots().is_ok_and(|r| r.is_empty()) && db.list_spaces().is_ok_and(|s| s.is_empty());
    if !empty {
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
