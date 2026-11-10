//! Materiales de ventana de Windows: el "vidrio" del sistema de diseño Vidrio.
//!
//! **Un único vidrio para toda la app**: Acrylic con el mismo tinte en la barra, en
//! Ajustes y en cualquier ventana (ver ADR 0005 y docs/design-system.md).
//!
//! El problema: en Windows 10 v1903+ (y Windows 11 22000), Acrylic va a tirones mientras
//! la ventana se **arrastra o redimensiona** (documentado en `window-vibrancy`). La barra
//! nunca se mueve, pero Ajustes sí. Solución: mientras una ventana se mueve, se cambia a
//! *blur* con el mismo tinte (que no da tirones), y 200 ms después de soltarla vuelve el
//! Acrylic. En reposo, que es lo que se ve, el aspecto es idéntico al de la barra.
//!
//! Si Acrylic no está disponible se usa blur; si nada, la ventana queda opaca y la UI
//! pinta su fondo sólido (`data-glass="false"`). Lo mismo con «Desactivar vidrio».

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::appearance::{self, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Material {
    Acrylic,
    Blur,
    /// Sin material: fondo opaco.
    None,
}

/// Tiempo sin moverse tras el que una ventana vuelve a Acrylic.
const SETTLE: Duration = Duration::from_millis(200);

/// Tinte (RGBA) bajo el desenfoque: **el mismo** en todas las ventanas y materiales.
/// Bastante opaco: el texto tiene que leerse sobre cualquier fondo de escritorio.
pub fn tint(theme: Theme) -> (u8, u8, u8, u8) {
    match theme {
        Theme::Dark => (32, 32, 36, 200),
        Theme::Light => (243, 243, 245, 200),
    }
}

/// Orden en que se prueban los materiales en reposo.
pub const CANDIDATES: [Material; 2] = [Material::Acrylic, Material::Blur];

/// Aplica el vidrio (el primer material que funcione) y devuelve cuál, o `None`.
pub fn apply(window: &WebviewWindow, theme: Theme) -> Material {
    for material in CANDIDATES {
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

/// Quita el material de una ventana (vuelve a opaca).
pub fn clear(window: &WebviewWindow, material: Material) {
    #[cfg(windows)]
    {
        let result = match material {
            Material::Acrylic => window_vibrancy::clear_acrylic(window),
            Material::Blur => window_vibrancy::clear_blur(window),
            Material::None => Ok(()),
        };
        if let Err(e) = result {
            tracing::warn!(window = window.label(), error = %e, "no se pudo quitar el material");
        }
    }
    #[cfg(not(windows))]
    let _ = (window, material);
}

#[derive(Debug, Clone, Copy)]
struct WindowGlass {
    /// Material en reposo (lo que se ve).
    material: Material,
    /// Cuándo se movió por última vez, si ahora lleva blur por estar moviéndose.
    moving_since: Option<Instant>,
}

/// Qué material tiene cada ventana (por etiqueta), para que su UI elija fondo y para
/// el cambio a blur mientras se mueve.
#[derive(Default)]
pub struct Materials(Mutex<HashMap<String, WindowGlass>>);

impl Materials {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, WindowGlass>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn set(&self, label: &str, material: Material) {
        self.lock().insert(
            label.to_owned(),
            WindowGlass {
                material,
                moving_since: None,
            },
        );
    }

    /// Material en reposo de una ventana (`None` si no tiene).
    pub fn get(&self, label: &str) -> Material {
        self.lock()
            .get(label)
            .map_or(Material::None, |g| g.material)
    }
}

/// Pone o quita el vidrio en una ventana y lo apunta. Devuelve el material resultante.
pub fn set_glass(app: &AppHandle, window: &WebviewWindow, enabled: bool) -> Material {
    let materials = app.state::<Materials>();
    let current = materials.get(window.label());
    let next = match (enabled, current) {
        (true, Material::None) => apply(window, appearance::theme()),
        (false, m) if m != Material::None => {
            clear(window, m);
            Material::None
        }
        (_, m) => m,
    };
    materials.set(window.label(), next);
    next
}

/// La ventana se está moviendo o redimensionando: Acrylic → blur (mismo tinte) hasta que
/// pare. Se llama con cada evento `Moved`/`Resized` de las ventanas que se pueden mover.
pub fn on_moved(app: &AppHandle, window: &WebviewWindow) {
    let label = window.label().to_owned();
    let start_watch = {
        let materials = app.state::<Materials>();
        let mut map = materials.lock();
        let Some(glass) = map.get_mut(&label) else {
            return;
        };
        if glass.material != Material::Acrylic {
            return;
        }
        let first = glass.moving_since.is_none();
        glass.moving_since = Some(Instant::now());
        first
    };
    if !start_watch {
        return; // ya está en blur y vigilado: solo se ha actualizado la hora
    }
    let theme = appearance::theme();
    if let Err(e) = try_apply(window, Material::Blur, theme) {
        tracing::debug!(error = %e, "no se pudo pasar a blur al mover");
        return;
    }
    // Un hilo espera a que la ventana lleve `SETTLE` quieta y devuelve el Acrylic.
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(SETTLE / 2);
            let settled = {
                let materials = app.state::<Materials>();
                let mut map = materials.lock();
                match map.get_mut(&label) {
                    Some(g) if g.moving_since.is_some_and(|t| t.elapsed() >= SETTLE) => {
                        g.moving_since = None;
                        g.material == Material::Acrylic
                    }
                    Some(g) if g.moving_since.is_some() => continue,
                    _ => false, // la ventana ya no existe o cambió de material
                }
            };
            if settled {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(w) = handle.get_webview_window(&label) {
                        let _ = try_apply(&w, Material::Acrylic, appearance::theme());
                    }
                });
            }
            break;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_glass_for_the_whole_app() {
        assert_eq!(CANDIDATES[0], Material::Acrylic);
        // El mismo tinte para todos: la barra y Ajustes se ven idénticos en reposo.
        for theme in [Theme::Dark, Theme::Light] {
            assert!(
                tint(theme).3 >= 160,
                "el texto tiene que leerse sobre cualquier fondo"
            );
        }
    }

    #[test]
    fn unknown_windows_have_no_material() {
        let m = Materials::default();
        assert_eq!(m.get("x"), Material::None);
        m.set("x", Material::Acrylic);
        assert_eq!(m.get("x"), Material::Acrylic);
    }
}
