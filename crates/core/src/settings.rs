//! Ajustes de usuario persistidos en un JSON (`%APPDATA%\dev.sift.app\settings.json`).

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::hotkey::Hotkey;

/// Ajustes de Sift. `#[serde(default)]` hace que un fichero antiguo al que le falten
/// campos nuevos siga cargando: los que faltan toman su valor por defecto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Atajo global que muestra u oculta el lanzador.
    pub hotkey: Hotkey,
    /// "Solo texto": oculta todos los iconos de la barra.
    pub icons_hidden: bool,
    /// Apariencia. Bloque propio para que crezca (tema, intensidad del vidrio…) sin
    /// llenar de campos sueltos el nivel de arriba.
    pub appearance: AppearanceSettings,
    /// Qué se indexa además de textos, código y documentos.
    pub indexing: IndexingSettings,
    /// Actualizaciones automáticas (ADR 0018).
    pub updates: UpdateSettings,
    /// La bienvenida ya se vio. Sin fichero de ajustes (primera vez) es `false`; un
    /// fichero de antes que no tiene el campo cuenta como ya visto.
    #[serde(default = "yes")]
    pub welcome_done: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct IndexingSettings {
    /// No indexar imágenes ni vídeos. Por defecto se pueden buscar por su nombre (su
    /// contenido nunca se lee).
    pub skip_media: bool,
    /// Sin «búsqueda inteligente» (por significado): ni se descarga el modelo ni se
    /// calculan vectores. Por defecto está activada.
    pub disable_smart_search: bool,
    /// El usuario ha dicho que sí a descargar el modelo (unos 135 MB). Sin esto nunca se
    /// descarga; si ya está en disco, se usa igual.
    pub model_download_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateSettings {
    /// Mirar al arrancar si hay una versión nueva en GitHub. Encendido por defecto
    /// (ADR 0018): solo se pide el número de la última versión, y descargarla e instalarla
    /// siempre se pregunta aparte. Se apaga en Ajustes → General.
    pub check_on_startup: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_on_startup: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AppearanceSettings {
    /// «Desactivar vidrio»: toda la app opaca (barra, Ajustes y diálogos). Por defecto
    /// no: Sift entera es de vidrio, con un único aspecto.
    pub disable_glass: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("no se pudo leer o escribir el fichero de ajustes: {0}")]
    Io(#[from] io::Error),
    #[error("el fichero de ajustes no es válido: {0}")]
    Format(#[from] serde_json::Error),
}

impl Settings {
    /// Carga los ajustes. Si el fichero no existe todavía, devuelve los de por defecto.
    pub fn load(path: &Path) -> Result<Settings, SettingsError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(serde_json::from_str(&text)?),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Guarda de forma atómica: escribe un temporal y lo renombra encima del original.
    /// Si el proceso muere a mitad, queda el fichero viejo entero, nunca uno a medias.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Settings::load(&dir.path().join("no-existe.json")).unwrap();
        assert_eq!(s, Settings::default());
        assert_eq!(s.hotkey, Hotkey::DEFAULT);
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        let s = Settings {
            hotkey: "Ctrl+Shift+K".parse().unwrap(),
            icons_hidden: true,
            appearance: AppearanceSettings {
                disable_glass: true,
            },
            indexing: IndexingSettings {
                skip_media: true,
                disable_smart_search: true,
                model_download_allowed: true,
            },
            updates: UpdateSettings {
                check_on_startup: true,
            },
            welcome_done: true,
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn update_checks_are_on_unless_turned_off() {
        // Un fichero de antes (sin el bloque) y uno nuevo: se mira si hay versión nueva.
        let old: Settings = serde_json::from_str(r#"{"hotkey":"Alt+Space"}"#).unwrap();
        assert!(old.updates.check_on_startup);
        assert!(Settings::default().updates.check_on_startup);
        // Quien lo apagó lo sigue teniendo apagado.
        let off: Settings =
            serde_json::from_str(r#"{"updates":{"checkOnStartup":false}}"#).unwrap();
        assert!(!off.updates.check_on_startup);
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{ "campoDelFuturo": 1 }"#).unwrap();
        let s = Settings::load(&path).unwrap();
        // Un fichero existente sin `welcomeDone` es de un usuario de antes: ya visto.
        assert!(s.welcome_done);
        assert!(!s.indexing.model_download_allowed);
        assert_eq!(
            s,
            Settings {
                welcome_done: true,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn default_settings_have_not_seen_the_welcome() {
        assert!(!Settings::default().welcome_done);
        assert!(!Settings::default().indexing.model_download_allowed);
    }

    #[test]
    fn files_from_phase_1_still_load() {
        // Un settings.json de la Fase 1 no tenía `iconsHidden`: toma su valor por defecto.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{ "hotkey": "Ctrl+K" }"#).unwrap();
        let s = Settings::load(&path).unwrap();
        assert_eq!(s.hotkey.to_string(), "Ctrl+K");
        assert!(!s.icons_hidden);
        assert!(!s.appearance.disable_glass); // vidrio por defecto
    }

    #[test]
    fn invalid_hotkey_is_a_format_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{ "hotkey": "Space" }"#).unwrap();
        assert!(matches!(
            Settings::load(&path),
            Err(SettingsError::Format(_))
        ));
    }
}
