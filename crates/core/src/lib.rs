//! Tipos de dominio, configuración y errores compartidos.
//!

pub mod hotkey;
pub mod settings;

pub use hotkey::{Hotkey, HotkeyParseError, Key, Modifiers};
pub use settings::{AppearanceSettings, Settings, SettingsError};

/// Versión de Sift, tomada de `Cargo.toml` en tiempo de compilación.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver() {
        // X.Y.Z, con una etiqueta de versión candidata opcional: «1.0.0-rc.1».
        let (core, pre) = VERSION.split_once('-').unwrap_or((VERSION, ""));
        let parts: Vec<&str> = core.split('.').collect();
        assert_eq!(parts.len(), 3, "la versión debe tener formato X.Y.Z");
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
        assert!(
            pre.is_empty()
                || pre
                    .split('.')
                    .all(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())),
            "la etiqueta de versión candidata no es válida: {pre}"
        );
    }
}
