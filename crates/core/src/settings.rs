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
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{ "campoDelFuturo": 1 }"#).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
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
