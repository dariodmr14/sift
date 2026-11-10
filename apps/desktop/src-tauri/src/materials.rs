//! Materiales de ventana de Windows (el "vidrio" del sistema de diseño Vidrio).
//!
//! Como en Fluent, el material depende del tipo de superficie:
//! - **Flotante** (la barra): Acrylic. Nunca se arrastra ni se redimensiona.
//! - **Ventana de app** (Ajustes): Mica en Windows 11; en Windows 10, *blur* con tinte.
//!   Acrylic no vale aquí: en Windows 10 v1903+ y Windows 11 22000 va a tirones al
//!   arrastrar o redimensionar (documentado en `window-vibrancy`); el blur, en Windows 10,
//!   no.
//!
//! Si un material no está disponible, se prueba el siguiente; si ninguno, la ventana
//! queda opaca y la UI pinta su fondo sólido (CSS: `data-glass="false"`).

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::WebviewWindow;

use crate::appearance::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Material {
    Acrylic,
    Mica,
    Blur,
    /// Sin material: fondo opaco.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Flyout,
    AppWindow,
}

/// Tinte (RGBA) que Windows pone bajo el desenfoque. Bastante opaco: el texto tiene que
/// leerse sobre cualquier fondo de escritorio.
fn tint(theme: Theme) -> (u8, u8, u8, u8) {
    match theme {
        Theme::Dark => (32, 32, 36, 168),
        Theme::Light => (243, 243, 245, 176),
    }
}

/// Orden en que se prueban los materiales para cada superficie.
pub fn candidates(surface: Surface) -> &'static [Material] {
    match surface {
        Surface::Flyout => &[Material::Acrylic],
        Surface::AppWindow => &[Material::Mica, Material::Blur],
    }
}

/// Aplica el primer material que funcione y devuelve cuál (o `None`).
pub fn apply(window: &WebviewWindow, surface: Surface, theme: Theme) -> Material {
    for &material in candidates(surface) {
        match try_apply(window, material, theme) {
            Ok(()) => {
                tracing::info!(
                    window = window.label(),
                    ?material,
                    ?theme,
                    "material aplicado"
                );
                return material;
            }
            Err(e) => {
                tracing::debug!(window = window.label(), ?material, error = %e, "material no disponible")
            }
        }
    }
    tracing::warn!(
        window = window.label(),
        "sin material de Windows; fondo opaco"
    );
    Material::None
}

#[cfg(windows)]
fn try_apply(window: &WebviewWindow, material: Material, theme: Theme) -> Result<(), String> {
    let result = match material {
        Material::Acrylic => window_vibrancy::apply_acrylic(window, Some(tint(theme))),
        Material::Mica => window_vibrancy::apply_mica(window, Some(theme == Theme::Dark)),
        Material::Blur => window_vibrancy::apply_blur(window, Some(tint(theme))),
        Material::None => return Ok(()),
    };
    result.map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn try_apply(_: &WebviewWindow, _: Material, theme: Theme) -> Result<(), String> {
    let _ = tint(theme);
    Err("solo Windows".into())
}

/// Qué material tiene cada ventana (por etiqueta), para que su UI elija fondo.
#[derive(Default)]
pub struct Materials(Mutex<HashMap<String, Material>>);

impl Materials {
    pub fn set(&self, label: &str, material: Material) {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(label.to_owned(), material);
    }

    pub fn get(&self, label: &str) -> Material {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(label)
            .copied()
            .unwrap_or(Material::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flyouts_use_acrylic_and_app_windows_avoid_it() {
        assert_eq!(candidates(Surface::Flyout), &[Material::Acrylic]);
        // Acrylic va a tirones al arrastrar en Windows 10: las ventanas de app no lo usan.
        assert!(!candidates(Surface::AppWindow).contains(&Material::Acrylic));
        assert_eq!(candidates(Surface::AppWindow)[0], Material::Mica);
    }

    #[test]
    fn unknown_windows_have_no_material() {
        let m = Materials::default();
        assert_eq!(m.get("x"), Material::None);
        m.set("x", Material::Blur);
        assert_eq!(m.get("x"), Material::Blur);
    }

    #[test]
    fn tints_are_mostly_opaque() {
        for theme in [Theme::Dark, Theme::Light] {
            assert!(
                tint(theme).3 >= 160,
                "el texto tiene que leerse sobre cualquier fondo"
            );
        }
    }
}
