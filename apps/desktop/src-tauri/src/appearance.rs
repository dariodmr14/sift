//! Tema (claro/oscuro) y color de acento de Windows, para que Sift los siga.
//!
//! Windows guarda en el registro una paleta de 7 tonos de tu color de acento
//! (`AccentPalette`: 3 claros, el base y 3 oscuros). Como hace Fluent, sobre fondo oscuro
//! se usa un tono claro ("Light 2") y sobre fondo claro uno oscuro ("Dark 1"): así el
//! acento siempre contrasta con lo que hay detrás.

use serde::Serialize;

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
    let theme = platform::theme();
    let accent = accent_for(theme, platform::palette(), platform::dwm_accent());
    Appearance {
        theme,
        accent: accent.hex(),
        accent_text: accent.readable_text().hex(),
    }
}

pub fn theme() -> Theme {
    platform::theme()
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
    fn text_on_accent_has_the_best_contrast() {
        assert_eq!(Rgb(0xB5, 0xC3, 0xE3).readable_text(), Rgb(0, 0, 0)); // claro → negro
        assert_eq!(Rgb(0x00, 0x5F, 0xB8).readable_text(), Rgb(255, 255, 255)); // oscuro → blanco
    }
}
