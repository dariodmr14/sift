//! Combinaciones de teclas para el atajo global ("Alt+Space", "Ctrl+Shift+K"…).
//!
//! El tipo es independiente de la plataforma salvo [`Key::vk_code`], que da el código de
//! tecla virtual de Windows. Se guarda en los ajustes como texto, así que `FromStr` y
//! `Display` tienen que ser inversos: `parse(h.to_string()) == h`.

use std::fmt;
use std::str::FromStr;

/// Modificadores pulsados junto a la tecla principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

impl Modifiers {
    pub fn is_empty(self) -> bool {
        !(self.ctrl || self.alt || self.shift || self.win)
    }
}

/// Tecla principal del atajo. Solo las que tienen sentido para un lanzador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Letra A–Z (siempre en mayúscula).
    Letter(char),
    /// Dígito 0–9 de la fila superior.
    Digit(u8),
    /// Tecla de función F1–F24.
    F(u8),
    Space,
    Enter,
    Tab,
    /// La tecla a la izquierda del 1 (`º` en teclado español, `` ` `` en inglés).
    Backquote,
}

impl Key {
    /// Código de tecla virtual de Windows (`VK_*`).
    /// <https://learn.microsoft.com/windows/win32/inputdev/virtual-key-codes>
    pub fn vk_code(self) -> u16 {
        match self {
            // En Windows, VK de 'A'..'Z' y '0'..'9' coinciden con su código ASCII.
            Key::Letter(c) => c as u16,
            Key::Digit(d) => u16::from(b'0' + d),
            Key::F(n) => 0x70 + u16::from(n - 1), // VK_F1 = 0x70
            Key::Space => 0x20,
            Key::Enter => 0x0D,
            Key::Tab => 0x09,
            Key::Backquote => 0xC0, // VK_OEM_3
        }
    }

    fn parse(s: &str) -> Option<Key> {
        let upper = s.to_ascii_uppercase();
        let mut chars = upper.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if c.is_ascii_uppercase() {
                return Some(Key::Letter(c));
            }
            if let Some(d) = c.to_digit(10) {
                return Some(Key::Digit(d as u8));
            }
            if c == '`' {
                return Some(Key::Backquote);
            }
        }
        if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
            return (1..=24).contains(&n).then_some(Key::F(n));
        }
        match upper.as_str() {
            "SPACE" | "ESPACIO" => Some(Key::Space),
            "ENTER" | "RETURN" => Some(Key::Enter),
            "TAB" => Some(Key::Tab),
            "BACKQUOTE" => Some(Key::Backquote),
            _ => None,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Letter(c) => write!(f, "{c}"),
            Key::Digit(d) => write!(f, "{d}"),
            Key::F(n) => write!(f, "F{n}"),
            Key::Space => f.write_str("Space"),
            Key::Enter => f.write_str("Enter"),
            Key::Tab => f.write_str("Tab"),
            Key::Backquote => f.write_str("Backquote"),
        }
    }
}

/// Un atajo completo: modificadores + una tecla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl Hotkey {
    /// `Alt+Space`, el atajo por defecto.
    pub const DEFAULT: Hotkey = Hotkey {
        modifiers: Modifiers {
            ctrl: false,
            alt: true,
            shift: false,
            win: false,
        },
        key: Key::Space,
    };
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey::DEFAULT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyParseError {
    #[error("el atajo está vacío")]
    Empty,
    #[error("tecla desconocida: «{0}»")]
    UnknownKey(String),
    #[error("modificador repetido: «{0}»")]
    DuplicateModifier(String),
    #[error("el atajo necesita una tecla principal además de los modificadores")]
    MissingKey,
    #[error("el atajo solo puede tener una tecla principal (sobra «{0}»)")]
    TooManyKeys(String),
    #[error("el atajo necesita al menos un modificador (Ctrl, Alt, Shift o Win)")]
    NoModifier,
}

impl FromStr for Hotkey {
    type Err = HotkeyParseError;

    /// Acepta mayúsculas o minúsculas, espacios alrededor de `+` y alias habituales
    /// (`Control`, `Mayús`, `Super`…). El orden de los modificadores da igual.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err(HotkeyParseError::Empty);
        }
        let mut modifiers = Modifiers::default();
        let mut key = None;

        for part in s.split('+').map(str::trim) {
            let flag = match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Some(&mut modifiers.ctrl),
                "alt" => Some(&mut modifiers.alt),
                "shift" | "mayús" | "mayus" => Some(&mut modifiers.shift),
                "win" | "super" | "meta" | "cmd" => Some(&mut modifiers.win),
                _ => None,
            };
            match flag {
                Some(flag) if *flag => {
                    return Err(HotkeyParseError::DuplicateModifier(part.to_owned()));
                }
                Some(flag) => *flag = true,
                None => {
                    let parsed = Key::parse(part)
                        .ok_or_else(|| HotkeyParseError::UnknownKey(part.into()))?;
                    if key.replace(parsed).is_some() {
                        return Err(HotkeyParseError::TooManyKeys(part.to_owned()));
                    }
                }
            }
        }

        let key = key.ok_or(HotkeyParseError::MissingKey)?;
        if modifiers.is_empty() {
            return Err(HotkeyParseError::NoModifier);
        }
        Ok(Hotkey { modifiers, key })
    }
}

impl fmt::Display for Hotkey {
    /// Orden fijo (Ctrl, Alt, Shift, Win) para que el texto guardado sea estable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = self.modifiers;
        for (on, name) in [
            (m.ctrl, "Ctrl"),
            (m.alt, "Alt"),
            (m.shift, "Shift"),
            (m.win, "Win"),
        ] {
            if on {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", self.key)
    }
}

// En los ajustes se guarda como texto ("Alt+Space"), no como objeto.
impl serde::Serialize for Hotkey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Hotkey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default() {
        assert_eq!("Alt+Space".parse::<Hotkey>().unwrap(), Hotkey::DEFAULT);
    }

    #[test]
    fn is_lenient_with_case_spaces_and_aliases() {
        let h: Hotkey = " control + mayús + k ".parse().unwrap();
        assert!(h.modifiers.ctrl && h.modifiers.shift && !h.modifiers.alt);
        assert_eq!(h.key, Key::Letter('K'));
    }

    #[test]
    fn display_roundtrips_in_canonical_order() {
        let h: Hotkey = "Win+Shift+Alt+Ctrl+F12".parse().unwrap();
        assert_eq!(h.to_string(), "Ctrl+Alt+Shift+Win+F12");
        assert_eq!(h.to_string().parse::<Hotkey>().unwrap(), h);
    }

    #[test]
    fn rejects_invalid_combinations() {
        use HotkeyParseError::*;
        assert_eq!("".parse::<Hotkey>(), Err(Empty));
        assert_eq!("Space".parse::<Hotkey>(), Err(NoModifier));
        assert_eq!("Alt".parse::<Hotkey>(), Err(MissingKey));
        assert_eq!(
            "Alt+Alt+K".parse::<Hotkey>(),
            Err(DuplicateModifier("Alt".into()))
        );
        assert_eq!("Alt+K+J".parse::<Hotkey>(), Err(TooManyKeys("J".into())));
        assert_eq!("Alt+F25".parse::<Hotkey>(), Err(UnknownKey("F25".into())));
        assert_eq!("Alt+Ñ".parse::<Hotkey>(), Err(UnknownKey("Ñ".into())));
    }

    #[test]
    fn maps_windows_virtual_key_codes() {
        assert_eq!(Key::Space.vk_code(), 0x20);
        assert_eq!(Key::Letter('A').vk_code(), 0x41);
        assert_eq!(Key::Digit(7).vk_code(), 0x37);
        assert_eq!(Key::F(1).vk_code(), 0x70);
        assert_eq!(Key::F(24).vk_code(), 0x87);
    }

    #[test]
    fn serializes_as_string() {
        let json = serde_json::to_string(&Hotkey::DEFAULT).unwrap();
        assert_eq!(json, "\"Alt+Space\"");
        assert_eq!(
            serde_json::from_str::<Hotkey>(&json).unwrap(),
            Hotkey::DEFAULT
        );
        assert!(serde_json::from_str::<Hotkey>("\"Space\"").is_err());
    }
}
