//! Ejecutar las acciones de un Espacio.
//!
//! La lógica (qué se lanza, en qué orden, qué hay que confirmar) está separada de *cómo*
//! se lanza en Windows mediante el trait [`Opener`]. La app usa uno real (ShellExecute y
//! `std::process::Command`); los tests, uno falso que solo apunta lo que se le pide.

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::model::{Action, CommandSpec};
use crate::validate::validate_action;

/// Lo que sabe hacer el sistema operativo. Cada método lanza una sola cosa.
pub trait Opener {
    fn open_path(&self, path: &Path) -> Result<(), String>;
    fn open_url(&self, url: &str) -> Result<(), String>;
    fn open_app(&self, app_id: &str) -> Result<(), String>;
    /// Lanza `program` con `args` **como lista**, sin shell.
    fn spawn(&self, command: &CommandSpec) -> Result<(), String>;
}

/// Resultado de una acción, para mostrarlo en la UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ActionOutcome {
    /// Posición de la acción (desde 1).
    pub index: u32,
    pub label: String,
    /// `None` si fue bien; si no, el motivo.
    pub error: Option<String>,
}

/// Comandos de `actions` que aún no se han aprobado, sin repetidos y en orden.
pub fn pending_approvals<'a>(
    actions: impl IntoIterator<Item = &'a Action>,
    is_approved: impl Fn(&CommandSpec) -> bool,
) -> Vec<CommandSpec> {
    let mut seen = HashSet::new();
    actions
        .into_iter()
        .filter_map(|a| match a {
            Action::RunCommand(c) if !is_approved(c) && seen.insert(c.clone()) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// Ejecuta las acciones en orden. Si una falla, sigue con las demás: que no abra la web
/// no es motivo para no abrir la carpeta. Devuelve qué pasó con cada una.
///
/// Antes de llamar a esto, la app comprueba con [`pending_approvals`] que no quede
/// ningún comando sin confirmar.
pub fn run_actions(actions: &[Action], opener: &dyn Opener) -> Vec<ActionOutcome> {
    actions
        .iter()
        .enumerate()
        .map(|(i, action)| ActionOutcome {
            index: i as u32 + 1,
            label: label(action),
            error: run_action(action, opener).err(),
        })
        .collect()
}

/// Ejecuta una acción. Se vuelve a validar aquí, justo antes de lanzarla, aunque ya se
/// validó al guardar: es la última puerta antes del sistema operativo.
pub fn run_action(action: &Action, opener: &dyn Opener) -> Result<(), String> {
    let mut checked = action.clone();
    validate_action(&mut checked)?;
    match &checked {
        Action::OpenFolder { path } | Action::OpenFile { path } => {
            if !path.exists() {
                return Err(format!("no existe: {}", path.display()));
            }
            opener.open_path(path)
        }
        Action::OpenUrl { url } => opener.open_url(url),
        Action::OpenApp { app_id, .. } => opener.open_app(app_id),
        Action::RunCommand(cmd) => {
            if let Some(cwd) = &cmd.cwd
                && !cwd.is_dir()
            {
                return Err(format!(
                    "no existe la carpeta de trabajo: {}",
                    cwd.display()
                ));
            }
            opener.spawn(cmd)
        }
    }
}

/// Texto corto que describe una acción ("Abrir carpeta Bases de Datos").
pub fn label(action: &Action) -> String {
    let file_name = |p: &Path| {
        p.file_name().map_or_else(
            || p.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    };
    match action {
        Action::OpenFolder { path } => format!("Abrir carpeta {}", file_name(path)),
        Action::OpenFile { path } => format!("Abrir {}", file_name(path)),
        Action::OpenUrl { url } => format!("Abrir {url}"),
        Action::OpenApp { name, .. } => format!("Abrir {name}"),
        Action::RunCommand(c) => format!("Ejecutar {}", display_command(c)),
    }
}

/// Cómo se muestra un comando en la confirmación: cada argumento por separado y
/// entrecomillado si tiene espacios, para que se vea exactamente qué se ejecutará.
pub fn display_command(c: &CommandSpec) -> String {
    let quote = |s: &str| {
        if s.is_empty() || s.contains(char::is_whitespace) {
            format!("\"{s}\"")
        } else {
            s.to_owned()
        }
    };
    std::iter::once(quote(&c.program))
        .chain(c.args.iter().map(|a| quote(a)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;

    /// Opener falso: apunta lo que se le pide en vez de lanzarlo.
    #[derive(Default)]
    struct Recorder {
        calls: RefCell<Vec<String>>,
        spawned: RefCell<Vec<CommandSpec>>,
        fail_urls: bool,
    }

    impl Opener for Recorder {
        fn open_path(&self, path: &Path) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("path:{}", path.display()));
            Ok(())
        }
        fn open_url(&self, url: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("url:{url}"));
            if self.fail_urls {
                Err("sin navegador".into())
            } else {
                Ok(())
            }
        }
        fn open_app(&self, app_id: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("app:{app_id}"));
            Ok(())
        }
        fn spawn(&self, command: &CommandSpec) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("spawn:{}", command.program));
            self.spawned.borrow_mut().push(command.clone());
            Ok(())
        }
    }

    fn cmd(program: &str, args: &[&str]) -> CommandSpec {
        CommandSpec {
            program: program.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: None,
        }
    }

    #[test]
    fn runs_in_order_and_keeps_going_after_errors() {
        let dir = tempfile::tempdir().unwrap();
        let rec = Recorder {
            fail_urls: true,
            ..Default::default()
        };
        let actions = vec![
            Action::OpenUrl {
                url: "https://campus.es".into(),
            },
            Action::OpenFolder {
                path: dir.path().to_path_buf(),
            },
            Action::OpenApp {
                app_id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into(),
                name: "Calculadora".into(),
            },
        ];
        let out = run_actions(&actions, &rec);
        assert_eq!(rec.calls.borrow().len(), 3);
        assert_eq!(out[0].error.as_deref(), Some("sin navegador"));
        assert_eq!(out[1].error, None);
        assert_eq!(out[2].label, "Abrir Calculadora");
        assert_eq!(
            out.iter().map(|o| o.index).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn missing_paths_are_reported_without_calling_the_os() {
        let rec = Recorder::default();
        let r = run_action(
            &Action::OpenFile {
                path: PathBuf::from(r"C:\no\existe.pdf"),
            },
            &rec,
        );
        assert!(r.unwrap_err().contains("no existe"));
        assert!(rec.calls.borrow().is_empty());
    }

    #[test]
    fn invalid_actions_never_reach_the_os() {
        // Aunque un JSON importado traiga algo peligroso, se bloquea antes de lanzarlo.
        let rec = Recorder::default();
        assert!(
            run_action(
                &Action::OpenUrl {
                    url: "file:///C:/x.exe".into()
                },
                &rec
            )
            .is_err()
        );
        assert!(
            run_action(
                &Action::OpenFile {
                    path: PathBuf::from(r"C:\x\a.exe")
                },
                &rec
            )
            .is_err()
        );
        assert!(rec.calls.borrow().is_empty());
    }

    #[test]
    fn command_arguments_stay_separate() {
        // Un argumento con "&" o comillas llega tal cual, como UN argumento: no hay
        // shell que lo interprete como "y ahora ejecuta esto otro".
        let rec = Recorder::default();
        let tricky = cmd("docker", &["compose", "up", "a & del C:\\*", "\"x\""]);
        run_action(&Action::RunCommand(tricky.clone()), &rec).unwrap();
        assert_eq!(rec.spawned.borrow()[0], tricky);
        assert_eq!(rec.spawned.borrow()[0].args.len(), 4);
    }

    #[test]
    fn missing_working_directory_is_an_error() {
        let rec = Recorder::default();
        let mut c = cmd("docker", &[]);
        c.cwd = Some(PathBuf::from(r"C:\no\existe"));
        assert!(
            run_action(&Action::RunCommand(c), &rec)
                .unwrap_err()
                .contains("carpeta de trabajo")
        );
        assert!(rec.spawned.borrow().is_empty());
    }

    #[test]
    fn pending_approvals_skips_approved_and_duplicates() {
        let up = cmd("docker", &["compose", "up"]);
        let down = cmd("docker", &["compose", "down"]);
        let actions = vec![
            Action::RunCommand(up.clone()),
            Action::OpenUrl {
                url: "https://x.es".into(),
            },
            Action::RunCommand(down.clone()),
            Action::RunCommand(up.clone()),
        ];
        assert_eq!(
            pending_approvals(&actions, |_| false),
            vec![up.clone(), down.clone()]
        );
        assert_eq!(pending_approvals(&actions, |c| *c == up), vec![down]);
        assert!(pending_approvals(&actions, |_| true).is_empty());
    }

    #[test]
    fn displays_commands_unambiguously() {
        let c = cmd(r"C:\Program Files\x.exe", &["--name", "mi carpeta", ""]);
        assert_eq!(
            display_command(&c),
            r#""C:\Program Files\x.exe" --name "mi carpeta" """#
        );
    }
}
