//! Tema (claro/oscuro) y color de acento de Windows, para que Sift los siga.
//!
//! Windows guarda en el registro una paleta de 7 tonos de tu color de acento
//! (`AccentPalette`: 3 claros, el base y 3 oscuros). Como hace Fluent, sobre fondo oscuro
//! se usa un tono claro ("Light 2") y sobre fondo claro uno oscuro ("Dark 1"): así el
//! acento siempre contrasta con lo que hay detrás.
//!
//! El tema puede no ser el de Windows: en Ajustes → General → Apariencia se elige «Como
//! Windows», «Claro» u «Oscuro» (`ThemePreference`). La elección se guarda en una
//! variable global para que leer el tema siga costando microsegundos (se lee al mostrar
//! la barra).

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};
use sift_core::ThemePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// Luminancia relativa (WCAG), de 0 (negro) a 1 (blanco).
    fn luminance(self) -> f64 {
        let lin = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
    }

    /// Contraste WCAG entre dos colores, de 1 (iguales) a 21 (negro sobre blanco).
    pub fn contrast(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Mezcla con `to` en proporción `t` (0 = este color, 1 = `to`).
    fn mix(self, to: Rgb, t: f64) -> Rgb {
        let ch = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Rgb(ch(self.0, to.0), ch(self.1, to.1), ch(self.2, to.2))
    }

    /// Texto que se lee mejor encima de este color: negro o blanco (mayor contraste).
    pub fn readable_text(self) -> Rgb {
        let l = self.luminance();
        let contrast_black = (l + 0.05) / 0.05;
        let contrast_white = 1.05 / (l + 0.05);
        if contrast_black >= contrast_white {
            Rgb(0, 0, 0)
        } else {
            Rgb(255, 255, 255)
        }
    }
}

/// Lo que la UI necesita para pintarse como Windows.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub theme: Theme,
    /// Acento para este tema, en `#RRGGBB`.
    pub accent: String,
    /// Texto legible encima del acento (botones principales).
    pub accent_text: String,
    /// El acento cuando él mismo es el texto o el trazo (enlaces, anillo de foco, la
    /// marca de lo que encaja): el mismo tono, oscurecido o aclarado hasta leerse bien
    /// sobre el fondo de la app (auditoría de la Fase 7, F9).
    pub accent_ink: String,
}

/// Fondo opaco de la app en cada tema (`--surface-solid` en index.css).
const SURFACE_DARK: Rgb = Rgb(32, 32, 36);
const SURFACE_LIGHT: Rgb = Rgb(243, 243, 245);
/// El fondo más difícil con vidrio: el vidrio sobre un escritorio blanco (tema oscuro) o
/// negro (claro), con una fila seleccionada encima. Lo calcula y lo comprueba
/// src/glass-contrast.test.ts a partir de index.css y materials.rs.
const GLASS_WORST_DARK: Rgb = Rgb(74, 74, 77);
const GLASS_WORST_LIGHT: Rgb = Rgb(201, 201, 203);
/// Contraste mínimo de texto normal (WCAG AA). Un trazo pide 3:1; con 4,5 vale para los dos.
const INK_CONTRAST: f64 = 4.5;

/// El acento como «tinta» legible: si no contrasta lo bastante con el fondo (el opaco y
/// el peor del vidrio), se acerca a negro (tema claro) o a blanco (oscuro) en pasos del
/// 5 % hasta llegar a 4,5:1. Así sigue siendo tu color, solo más oscuro o más claro.
pub fn accent_ink(theme: Theme, accent: Rgb) -> Rgb {
    let (surfaces, toward) = match theme {
        Theme::Dark => ([SURFACE_DARK, GLASS_WORST_DARK], Rgb(255, 255, 255)),
        Theme::Light => ([SURFACE_LIGHT, GLASS_WORST_LIGHT], Rgb(0, 0, 0)),
    };
    (0..=20)
        .map(|step| accent.mix(toward, f64::from(step) * 0.05))
        .find(|c| surfaces.iter().all(|s| c.contrast(*s) >= INK_CONTRAST))
        .unwrap_or(toward)
}

/// Acentos por defecto de Windows (azul) si no se puede leer el del usuario.
const DEFAULT_DARK: Rgb = Rgb(0x60, 0xCD, 0xFF);
const DEFAULT_LIGHT: Rgb = Rgb(0x00, 0x5F, 0xB8);

/// `AccentPalette` son 8 colores de 4 bytes (R, G, B, A): 0–2 claros, 3 base, 4–6
/// oscuros (el 8.º no se usa). Devuelve `None` si el valor no tiene ese formato.
pub fn parse_palette(bytes: &[u8]) -> Option<[Rgb; 7]> {
    if bytes.len() < 28 {
        return None;
    }
    let mut out = [Rgb(0, 0, 0); 7];
    for (i, c) in out.iter_mut().enumerate() {
        *c = Rgb(bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2]);
    }
    Some(out)
}

/// `DWM\AccentColor` es un DWORD en orden 0xAABBGGRR.
pub fn parse_dwm_color(value: u32) -> Rgb {
    Rgb(
        (value & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        ((value >> 16) & 0xFF) as u8,
    )
}

/// Elige el acento para el tema: de la paleta si la hay; si no, el color de DWM; si no,
/// el azul de Windows.
pub fn accent_for(theme: Theme, palette: Option<[Rgb; 7]>, dwm: Option<Rgb>) -> Rgb {
    match (palette, theme) {
        (Some(p), Theme::Dark) => p[1],  // Light 2
        (Some(p), Theme::Light) => p[4], // Dark 1
        (None, _) => dwm.unwrap_or(match theme {
            Theme::Dark => DEFAULT_DARK,
            Theme::Light => DEFAULT_LIGHT,
        }),
    }
}

pub fn current() -> Appearance {
    let theme = theme();
    let accent = accent_for(theme, platform::palette(), platform::dwm_accent());
    Appearance {
        theme,
        accent: accent.hex(),
        accent_text: accent.readable_text().hex(),
        accent_ink: accent_ink(theme, accent).hex(),
    }
}

/// «Tema» en Ajustes, tal como lo ve la UI (`ThemePreference` de sift-core no depende
/// de specta).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl From<ThemePreference> for ThemeChoice {
    fn from(p: ThemePreference) -> Self {
        match p {
            ThemePreference::System => Self::System,
            ThemePreference::Light => Self::Light,
            ThemePreference::Dark => Self::Dark,
        }
    }
}

impl From<ThemeChoice> for ThemePreference {
    fn from(c: ThemeChoice) -> Self {
        match c {
            ThemeChoice::System => Self::System,
            ThemeChoice::Light => Self::Light,
            ThemeChoice::Dark => Self::Dark,
        }
    }
}

/// La elección de Ajustes (0 = como Windows, 1 = claro, 2 = oscuro).
static PREFERENCE: AtomicU8 = AtomicU8::new(0);

/// Apunta la elección de Ajustes (al arrancar y al cambiarla).
pub fn set_preference(preference: ThemePreference) {
    let value = match preference {
        ThemePreference::System => 0,
        ThemePreference::Light => 1,
        ThemePreference::Dark => 2,
    };
    PREFERENCE.store(value, Ordering::Relaxed);
}

/// El tema que resulta de la elección: la de Ajustes o, con «Como Windows», el de Windows.
pub fn resolve(preference: ThemePreference, windows: Theme) -> Theme {
    match preference {
        ThemePreference::System => windows,
        ThemePreference::Light => Theme::Light,
        ThemePreference::Dark => Theme::Dark,
    }
}

/// El tema de Sift ahora mismo.
pub fn theme() -> Theme {
    let preference = match PREFERENCE.load(Ordering::Relaxed) {
        1 => ThemePreference::Light,
        2 => ThemePreference::Dark,
        _ => ThemePreference::System,
    };
    resolve(preference, platform::theme())
}

/// ¿Tiene Windows activados los «Efectos de transparencia»?
pub fn transparency_enabled() -> bool {
    platform::transparency_enabled()
}

/// El tema de la barra de tareas, que puede no ser el de las apps (en Windows 10, por
/// defecto, las apps van en claro y la barra de tareas en oscuro). Para el icono de la
/// bandeja.
pub fn taskbar_theme() -> Theme {
    platform::taskbar_theme()
}

#[cfg(windows)]
#[allow(unsafe_code)] // Lectura del registro con RegGetValueW.
mod platform {
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD, RegGetValueW,
    };

    use super::{Rgb, Theme, parse_dwm_color, parse_palette};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Lee un valor del registro de HKCU en `buf`. Devuelve los bytes leídos.
    fn read(key: &str, value: &str, flags: u32, buf: &mut [u8]) -> Option<usize> {
        let (key, value) = (wide(key), wide(value));
        let mut size = buf.len() as u32;
        // SAFETY: cadenas UTF-16 terminadas en NUL y un búfer con su tamaño real; la
        // función escribe como mucho `size` bytes y actualiza `size` con lo escrito.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                flags,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut size,
            )
        };
        (status == 0).then_some(size as usize)
    }

    fn dword(key: &str, value: &str) -> Option<u32> {
        let mut buf = [0u8; 4];
        read(key, value, RRF_RT_REG_DWORD, &mut buf).map(|_| u32::from_le_bytes(buf))
    }

    pub fn theme() -> Theme {
        // 1 = las apps usan tema claro. Si no existe la clave, Windows usa claro.
        match dword(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "AppsUseLightTheme",
        ) {
            Some(0) => Theme::Dark,
            _ => Theme::Light,
        }
    }

    /// «Efectos de transparencia» (Configuración → Personalización → Colores). Sin la
    /// clave, activados.
    pub fn transparency_enabled() -> bool {
        dword(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "EnableTransparency",
        ) != Some(0)
    }

    pub fn taskbar_theme() -> Theme {
        // 1 = la barra de tareas va en claro. Sin la clave (Windows antiguo), oscura.
        match dword(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "SystemUsesLightTheme",
        ) {
            Some(1) => Theme::Light,
            _ => Theme::Dark,
        }
    }

    pub fn palette() -> Option<[Rgb; 7]> {
        let mut buf = [0u8; 32];
        let n = read(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent",
            "AccentPalette",
            RRF_RT_REG_BINARY,
            &mut buf,
        )?;
        parse_palette(&buf[..n])
    }

    pub fn dwm_accent() -> Option<Rgb> {
        dword(r"Software\Microsoft\Windows\DWM", "AccentColor").map(parse_dwm_color)
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{Rgb, Theme};
    pub fn theme() -> Theme {
        Theme::Dark
    }
    pub fn taskbar_theme() -> Theme {
        Theme::Dark
    }
    pub fn transparency_enabled() -> bool {
        true
    }
    pub fn palette() -> Option<[Rgb; 7]> {
        None
    }
    pub fn dwm_accent() -> Option<Rgb> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La `AccentPalette` real de este PC (acento lavanda).
    const PALETTE: [u8; 32] = [
        0xD7, 0xDB, 0xE4, 0xFF, 0xB5, 0xC3, 0xE3, 0xFF, 0x9F, 0xB3, 0xDF, 0xFF, 0x96, 0xA8, 0xD0,
        0xFF, 0x84, 0x99, 0xC8, 0xFF, 0x68, 0x87, 0xCA, 0xFF, 0x50, 0x72, 0xBE, 0xFF, 0x88, 0x17,
        0x98, 0x00,
    ];

    #[test]
    fn parses_the_palette_and_picks_a_tone_per_theme() {
        let p = parse_palette(&PALETTE).unwrap();
        assert_eq!(p[3].hex(), "#96A8D0"); // el base coincide con DWM\AccentColor
        assert_eq!(accent_for(Theme::Dark, Some(p), None).hex(), "#B5C3E3");
        assert_eq!(accent_for(Theme::Light, Some(p), None).hex(), "#8499C8");
    }

    #[test]
    fn dwm_color_is_abgr() {
        assert_eq!(parse_dwm_color(0xFFD0_A896).hex(), "#96A8D0");
    }

    #[test]
    fn falls_back_to_dwm_and_then_to_windows_blue() {
        let dwm = Some(Rgb(1, 2, 3));
        assert_eq!(accent_for(Theme::Dark, None, dwm), Rgb(1, 2, 3));
        assert_eq!(accent_for(Theme::Dark, None, None), DEFAULT_DARK);
        assert_eq!(accent_for(Theme::Light, None, None), DEFAULT_LIGHT);
        assert!(parse_palette(&[0; 10]).is_none());
    }

    #[test]
    fn accent_ink_reads_well_on_the_background_and_keeps_the_hue() {
        let p = parse_palette(&PALETTE).unwrap();
        // El lavanda de este PC en tema claro (#8499C8) solo da ~2,8:1 sobre el fondo.
        let light = accent_for(Theme::Light, Some(p), None);
        assert!(light.contrast(SURFACE_LIGHT) < 3.0);
        let ink = accent_ink(Theme::Light, light);
        assert!(ink.contrast(SURFACE_LIGHT) >= INK_CONTRAST);
        assert!(ink.2 > ink.0, "sigue siendo azulado: {}", ink.hex());
        // Lo que ya se lee bien no se toca.
        let dark = accent_for(Theme::Dark, Some(p), None);
        assert_eq!(accent_ink(Theme::Dark, dark), dark);
        assert_eq!(accent_ink(Theme::Dark, DEFAULT_DARK), DEFAULT_DARK);
        // Un acento imposible acaba en negro o blanco, nunca ilegible.
        let yellow = Rgb(255, 255, 0);
        assert!(accent_ink(Theme::Light, yellow).contrast(SURFACE_LIGHT) >= INK_CONTRAST);
    }

    #[test]
    fn the_accent_ink_also_reads_on_the_worst_glass() {
        // El azul de Windows por defecto se lee sobre el fondo opaco, pero no sobre el
        // vidrio encima de un escritorio negro: se oscurece un poco.
        assert!(DEFAULT_LIGHT.contrast(GLASS_WORST_LIGHT) < INK_CONTRAST);
        let ink = accent_ink(Theme::Light, DEFAULT_LIGHT);
        assert!(ink.contrast(GLASS_WORST_LIGHT) >= INK_CONTRAST);
        assert!(ink.contrast(SURFACE_LIGHT) >= INK_CONTRAST);
        for accent in [Rgb(255, 185, 0), Rgb(0, 120, 215), Rgb(232, 17, 35)] {
            for theme in [Theme::Light, Theme::Dark] {
                let ink = accent_ink(theme, accent);
                let worst = match theme {
                    Theme::Light => GLASS_WORST_LIGHT,
                    Theme::Dark => GLASS_WORST_DARK,
                };
                assert!(
                    ink.contrast(worst) >= INK_CONTRAST,
                    "{theme:?} {}",
                    ink.hex()
                );
            }
        }
    }

    #[test]
    fn the_theme_follows_windows_unless_one_is_chosen() {
        for windows in [Theme::Light, Theme::Dark] {
            assert_eq!(resolve(ThemePreference::System, windows), windows);
            assert_eq!(resolve(ThemePreference::Light, windows), Theme::Light);
            assert_eq!(resolve(ThemePreference::Dark, windows), Theme::Dark);
        }
        for choice in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
            assert_eq!(ThemeChoice::from(ThemePreference::from(choice)), choice);
        }
    }

    #[test]
    fn text_on_accent_has_the_best_contrast() {
        assert_eq!(Rgb(0xB5, 0xC3, 0xE3).readable_text(), Rgb(0, 0, 0)); // claro → negro
        assert_eq!(Rgb(0x00, 0x5F, 0xB8).readable_text(), Rgb(255, 255, 255)); // oscuro → blanco
    }
}
