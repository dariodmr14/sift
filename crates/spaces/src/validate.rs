//! Validación y normalización de un Espacio antes de guardarlo.
//!
//! Se valida en Rust (no solo en la UI) porque los datos también llegan por importación
//! de JSON, y un JSON se puede editar a mano.

use std::path::Path;

use crate::model::{Action, CommandSpec, Icon, SpaceDraft, WaitSpec, WaitUntil};

pub const MAX_NAME: usize = 80;
pub const MAX_ALIASES: usize = 20;
pub const MAX_ACTIONS: usize = 30;
/// Tamaño máximo de una imagen propia, ya en base64 (~256 KB de imagen).
pub const MAX_IMAGE_BASE64: usize = 350_000;
/// Lo más que se puede esperar en un paso: 10 minutos.
pub const MAX_WAIT_SECS: u32 = 600;
pub const IMAGE_MIMES: [&str; 3] = ["image/png", "image/svg+xml", "image/x-icon"];

/// Extensiones que, abiertas como "archivo", **ejecutarían** algo. Se rechazan: para
/// ejecutar está la acción "comando", que pide confirmación.
const EXECUTABLE_EXTENSIONS: [&str; 14] = [
    "exe", "com", "bat", "cmd", "ps1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "msi", "scr", "hta",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("el Espacio necesita un nombre")]
    EmptyName,
    #[error("el nombre es demasiado largo (máximo {MAX_NAME} caracteres)")]
    NameTooLong,
    #[error("demasiados alias (máximo {MAX_ALIASES})")]
    TooManyAliases,
    #[error("demasiadas acciones (máximo {MAX_ACTIONS})")]
    TooManyActions,
    #[error("acción {index}: {reason}")]
    Action { index: usize, reason: String },
    #[error("icono: {0}")]
    Icon(String),
}

/// Valida y normaliza: recorta espacios, quita alias vacíos o repetidos (sin distinguir
/// mayúsculas ni tildes) y comprueba cada acción. Devuelve la versión limpia.
pub fn validate(mut draft: SpaceDraft) -> Result<SpaceDraft, ValidationError> {
    draft.name = draft.name.trim().to_owned();
    if draft.name.is_empty() {
        return Err(ValidationError::EmptyName);
    }
    if draft.name.chars().count() > MAX_NAME {
        return Err(ValidationError::NameTooLong);
    }

    let mut seen = vec![fold(&draft.name)];
    draft.aliases = draft
        .aliases
        .into_iter()
        .map(|a| a.trim().to_owned())
        .filter(|a| !a.is_empty())
        .filter(|a| {
            let key = fold(a);
            let fresh = !seen.contains(&key);
            seen.push(key);
            fresh
        })
        .collect();
    if draft.aliases.len() > MAX_ALIASES {
        return Err(ValidationError::TooManyAliases);
    }

    if draft.actions.len() > MAX_ACTIONS {
        return Err(ValidationError::TooManyActions);
    }
    for (i, action) in draft.actions.iter_mut().enumerate() {
        validate_action(action).map_err(|reason| ValidationError::Action {
            index: i + 1,
            reason,
        })?;
    }
    // «Esperar a que termine el comando anterior» necesita un comando justo antes.
    for (i, action) in draft.actions.iter().enumerate() {
        if let Action::Wait(WaitSpec {
            until: WaitUntil::CommandFinished,
            ..
        }) = action
            && (i == 0 || !matches!(draft.actions[i - 1], Action::RunCommand(_)))
        {
            return Err(ValidationError::Action {
                index: i + 1,
                reason: concat!(
                    "«esperar a que termine el comando anterior» tiene que ir ",
                    "justo después de un comando"
                )
                .into(),
            });
        }
    }

    validate_icon(&draft.icon).map_err(ValidationError::Icon)?;
    Ok(draft)
}

/// Valida una acción suelta (también se usa al pulsar "probar" en el editor).
pub fn validate_action(action: &mut Action) -> Result<(), String> {
    match action {
        Action::OpenFolder { path, .. } => check_path(path),
        Action::OpenFile { path, .. } => {
            check_path(path)?;
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if EXECUTABLE_EXTENSIONS.contains(&ext.as_str()) {
                return Err(format!(
                    "«.{ext}» es un ejecutable; para ejecutar algo usa la acción «comando» \
                     (pide confirmación) o «aplicación»"
                ));
            }
            Ok(())
        }
        Action::OpenUrl { url, .. } => {
            *url = url.trim().to_owned();
            let lower = url.to_ascii_lowercase();
            // Solo web: con `ShellExecute`, otros esquemas (file:, ms-…:) podrían abrir o
            // ejecutar cosas que no son una página.
            if lower.starts_with("https://") || lower.starts_with("http://") {
                if url.len() > 2048 || url.chars().any(char::is_whitespace) {
                    return Err("la dirección web no es válida".into());
                }
                Ok(())
            } else {
                Err("la dirección web tiene que empezar por https:// o http://".into())
            }
        }
        Action::OpenApp { app_id, name, .. } => {
            if app_id.trim().is_empty() || name.trim().is_empty() {
                return Err("elige una aplicación".into());
            }
            Ok(())
        }
        Action::RunCommand(cmd) => validate_command(cmd),
        Action::Wait(spec) => validate_wait(spec),
    }
}

fn validate_command(cmd: &mut CommandSpec) -> Result<(), String> {
    cmd.program = cmd.program.trim().to_owned();
    if cmd.program.is_empty() {
        return Err("falta el programa".into());
    }
    // Un NUL partiría la cadena al pasarla a Windows: se rechaza en todo el comando.
    let has_nul = cmd.program.contains('\0') || cmd.args.iter().any(|a| a.contains('\0'));
    if has_nul {
        return Err("el comando contiene caracteres no válidos".into());
    }
    if let Some(cwd) = &cmd.cwd {
        if cwd.as_os_str().is_empty() {
            cmd.cwd = None;
        } else {
            check_path(cwd)?;
        }
    }
    Ok(())
}

fn check_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("falta la ruta".into());
    }
    if !path.is_absolute() {
        return Err("la ruta tiene que ser completa (p. ej. C:\\Users\\…)".into());
    }
    Ok(())
}

fn validate_icon(icon: &Icon) -> Result<(), String> {
    match icon {
        Icon::Auto | Icon::None => Ok(()),
        Icon::Emoji { value } => {
            let n = value.trim().chars().count();
            if n == 0 || n > 8 {
                return Err("el emoji no es válido".into());
            }
            Ok(())
        }
        Icon::Library { name, svg } => {
            if name.is_empty() || !svg.trim_start().starts_with("<svg") || svg.len() > 20_000 {
                return Err("el icono de la librería no es válido".into());
            }
            Ok(())
        }
        Icon::Image { mime, data } => {
            if !IMAGE_MIMES.contains(&mime.as_str()) {
                return Err("la imagen tiene que ser PNG, SVG o ICO".into());
            }
            if data.is_empty() || data.len() > MAX_IMAGE_BASE64 {
                return Err("la imagen es demasiado grande (máximo ~256 KB)".into());
            }
            Ok(())
        }
    }
}

/// Clave para comparar textos sin mayúsculas ni tildes ("Bases" == "bases" == "básés").
pub fn fold(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            other => other,
        })
        .collect()
}

/// Una espera: solo cosas de tu equipo y tiempos razonables.
fn validate_wait(spec: &mut WaitSpec) -> Result<(), String> {
    let in_range = |secs: u32| (1..=MAX_WAIT_SECS).contains(&secs);
    match &mut spec.until {
        WaitUntil::WebResponds { url } => {
            *url = url.trim().to_owned();
            if !is_local_web(url) {
                return Err(concat!(
                    "la web tiene que estar en tu equipo (empieza por http://localhost): ",
                    "Sift no se conecta a internet"
                )
                .into());
            }
        }
        WaitUntil::ServiceAccepts { port } => {
            if *port == 0 {
                return Err("escribe el puerto del servicio (un número de 1 a 65535)".into());
            }
        }
        WaitUntil::CommandFinished => {}
        WaitUntil::Seconds { secs } => {
            if !in_range(*secs) {
                return Err(format!("se puede esperar de 1 a {MAX_WAIT_SECS} segundos"));
            }
            return Ok(());
        }
    }
    if !in_range(spec.timeout_secs) {
        return Err(format!(
            "el tiempo límite tiene que ser de 1 a {MAX_WAIT_SECS} segundos"
        ));
    }
    Ok(())
}

/// ¿Es una web de este equipo? `http(s)://` + `localhost`, `127.0.0.1` o `[::1]`, con
/// puerto y ruta opcionales. Se mira el **host exacto**: «localhost.ejemplo.com» o
/// «localhost@ejemplo.com» no valen.
pub fn is_local_web(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
    else {
        return false;
    };
    if url.len() > 2048 || url.chars().any(char::is_whitespace) {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = if authority.starts_with('[') {
        authority.split_inclusive(']').next().unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    let port_ok = match authority.strip_prefix(host) {
        Some("") => true,
        Some(p) => p
            .strip_prefix(':')
            .is_some_and(|n| n.parse::<u16>().is_ok_and(|n| n > 0)),
        None => false,
    };
    port_ok && matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn draft(name: &str) -> SpaceDraft {
        SpaceDraft {
            name: name.into(),
            aliases: vec![],
            icon: Icon::Auto,
            actions: vec![],
        }
    }

    fn abs(p: &str) -> PathBuf {
        PathBuf::from(p)
    }

    #[test]
    fn trims_name_and_dedups_aliases() {
        let mut d = draft("  Bases de datos ");
        d.aliases = vec![
            " bd ".into(),
            "BD".into(),
            "".into(),
            "bases de datos".into(),
            "Básés".into(),
            "bases".into(),
        ];
        let v = validate(d).unwrap();
        assert_eq!(v.name, "Bases de datos");
        // "BD" repite "bd"; "bases de datos" repite el nombre; "bases" repite "Básés".
        assert_eq!(v.aliases, vec!["bd", "Básés"]);
    }

    #[test]
    fn rejects_empty_or_long_names() {
        assert_eq!(validate(draft("   ")), Err(ValidationError::EmptyName));
        assert_eq!(
            validate(draft(&"x".repeat(81))),
            Err(ValidationError::NameTooLong)
        );
    }

    #[test]
    fn urls_must_be_web() {
        let ok = |u: &str| {
            validate_action(&mut Action::OpenUrl {
                url: u.into(),
                place: None,
            })
            .is_ok()
        };
        assert!(ok(" https://campusvirtual.uniovi.es "));
        assert!(ok("http://localhost:8080"));
        assert!(!ok("file:///C:/Windows/System32/calc.exe"));
        assert!(!ok("javascript:alert(1)"));
        assert!(!ok("ms-settings:"));
        assert!(!ok("https://a b"));
    }

    #[test]
    fn open_file_rejects_executables() {
        let check = |p: &str| {
            validate_action(&mut Action::OpenFile {
                path: abs(p),
                place: None,
            })
        };
        assert!(check(r"C:\Apuntes\tema4.pdf").is_ok());
        assert!(check(r"C:\x\setup.EXE").unwrap_err().contains("ejecutable"));
        assert!(check(r"C:\x\run.bat").is_err());
        assert!(check(r"C:\x\script.ps1").is_err());
    }

    #[test]
    fn paths_must_be_absolute() {
        let mut a = Action::OpenFolder {
            path: abs(r"Documentos\Uni"),
            place: None,
        };
        assert!(validate_action(&mut a).unwrap_err().contains("completa"));
    }

    #[test]
    fn commands_need_a_program_and_no_nul() {
        let mut ok = Action::RunCommand(CommandSpec {
            program: " docker ".into(),
            args: vec!["compose".into(), "up".into()],
            cwd: Some(PathBuf::new()),
        });
        validate_action(&mut ok).unwrap();
        let Action::RunCommand(c) = ok else {
            unreachable!()
        };
        assert_eq!(c.program, "docker");
        assert_eq!(c.cwd, None); // carpeta vacía = sin carpeta

        let mut empty = Action::RunCommand(CommandSpec {
            program: " ".into(),
            args: vec![],
            cwd: None,
        });
        assert!(validate_action(&mut empty).is_err());
        let mut nul = Action::RunCommand(CommandSpec {
            program: "x".into(),
            args: vec!["a\0b".into()],
            cwd: None,
        });
        assert!(validate_action(&mut nul).is_err());
    }

    #[test]
    fn reports_which_action_failed() {
        let mut d = draft("X");
        d.actions = vec![
            Action::OpenUrl {
                url: "https://ok.es".into(),
                place: None,
            },
            Action::OpenUrl {
                url: "ftp://no".into(),
                place: None,
            },
        ];
        assert!(matches!(
            validate(d),
            Err(ValidationError::Action { index: 2, .. })
        ));
    }

    #[test]
    fn validates_icons() {
        let with = |icon| {
            let mut d = draft("X");
            d.icon = icon;
            validate(d)
        };
        assert!(
            with(Icon::Emoji {
                value: "🗄️".into()
            })
            .is_ok()
        );
        assert!(with(Icon::Emoji { value: "".into() }).is_err());
        assert!(
            with(Icon::Image {
                mime: "image/gif".into(),
                data: "AAAA".into()
            })
            .is_err()
        );
        assert!(
            with(Icon::Image {
                mime: "image/png".into(),
                data: "A".repeat(MAX_IMAGE_BASE64 + 1)
            })
            .is_err()
        );
        assert!(
            with(Icon::Library {
                name: "database".into(),
                svg: "<svg></svg>".into()
            })
            .is_ok()
        );
        assert!(
            with(Icon::Library {
                name: "x".into(),
                svg: "<script>".into()
            })
            .is_err()
        );
    }

    #[test]
    fn only_local_web_addresses_can_be_waited_for() {
        for ok in [
            "http://localhost",
            "http://localhost:8080",
            "https://127.0.0.1:8443/salud?x=1",
            "http://[::1]:3000/",
            "HTTP://LOCALHOST:5173",
        ] {
            assert!(is_local_web(ok), "{ok}");
        }
        for bad in [
            "http://campus.uniovi.es",
            "http://localhost.ejemplo.com",
            "http://localhost@ejemplo.com",
            "http://user@localhost",
            "http://localhost:0",
            "http://localhost:99999",
            "http://localhost:abc",
            "ftp://localhost",
            "localhost:8080",
            "http://127.0.0.2",
        ] {
            assert!(!is_local_web(bad), "{bad}");
        }
    }

    fn wait(until: WaitUntil, timeout_secs: u32) -> Action {
        Action::Wait(WaitSpec {
            until,
            timeout_secs,
            on_timeout: crate::model::OnTimeout::Stop,
        })
    }

    #[test]
    fn waits_need_sensible_values() {
        let check = |mut a: Action| validate_action(&mut a);
        assert!(check(wait(WaitUntil::ServiceAccepts { port: 5432 }, 60)).is_ok());
        assert!(check(wait(WaitUntil::ServiceAccepts { port: 0 }, 60)).is_err());
        assert!(check(wait(WaitUntil::ServiceAccepts { port: 5432 }, 0)).is_err());
        assert!(check(wait(WaitUntil::ServiceAccepts { port: 5432 }, 601)).is_err());
        assert!(check(wait(WaitUntil::Seconds { secs: 5 }, 0)).is_ok());
        assert!(check(wait(WaitUntil::Seconds { secs: 0 }, 60)).is_err());
        let mut a = wait(
            WaitUntil::WebResponds {
                url: "  http://localhost:8080 ".into(),
            },
            60,
        );
        validate_action(&mut a).unwrap();
        assert_eq!(
            a,
            wait(
                WaitUntil::WebResponds {
                    url: "http://localhost:8080".into()
                },
                60
            )
        );
    }

    #[test]
    fn waiting_for_the_previous_command_needs_a_command_right_before() {
        let command = Action::RunCommand(CommandSpec {
            program: "docker".into(),
            args: vec!["compose".into(), "up".into(), "-d".into()],
            cwd: None,
        });
        let mut d = draft("Contenedor");
        d.actions = vec![command.clone(), wait(WaitUntil::CommandFinished, 120)];
        assert!(validate(d.clone()).is_ok());
        d.actions = vec![wait(WaitUntil::CommandFinished, 120)];
        let err = validate(d.clone()).unwrap_err();
        assert!(matches!(err, ValidationError::Action { index: 1, .. }));
        assert!(
            !err.to_string().contains("  "),
            "sin espacios repetidos: {err}"
        );
        d.actions = vec![
            command,
            wait(WaitUntil::Seconds { secs: 2 }, 60),
            wait(WaitUntil::CommandFinished, 120),
        ];
        assert!(matches!(
            validate(d),
            Err(ValidationError::Action { index: 3, .. })
        ));
    }
}
