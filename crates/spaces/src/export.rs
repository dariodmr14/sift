//! Formato de exportación e importación de Espacios (JSON).
//!
//! Lleva un nombre de formato y una versión para poder cambiarlo en el futuro sin romper
//! los ficheros antiguos. Las aprobaciones de comandos **no** se exportan: si importas
//! un Espacio con comandos, Sift te pedirá confirmación la primera vez, como debe ser
//! con algo que viene de otro sitio.

use serde::{Deserialize, Serialize};

use crate::model::SpaceDraft;
use crate::validate::{ValidationError, validate};

pub const FORMAT: &str = "sift-spaces";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub format: String,
    pub version: u32,
    pub spaces: Vec<SpaceDraft>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("el archivo no es un JSON válido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("el archivo no es una exportación de Espacios de Sift")]
    WrongFormat,
    #[error("el archivo es de una versión más nueva de Sift (v{0})")]
    NewerVersion(u32),
    #[error("Espacio «{name}»: {error}")]
    Invalid {
        name: String,
        error: ValidationError,
    },
}

pub fn to_json(spaces: Vec<SpaceDraft>) -> String {
    let file = ExportFile {
        format: FORMAT.into(),
        version: VERSION,
        spaces,
    };
    // `to_string_pretty` de un tipo propio sin mapas con claves no-string no puede fallar.
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// Lee y valida una exportación. Si algún Espacio no es válido, no se importa ninguno:
/// mejor un error claro que una importación a medias.
pub fn from_json(text: &str) -> Result<Vec<SpaceDraft>, ImportError> {
    let file: ExportFile = serde_json::from_str(text)?;
    if file.format != FORMAT {
        return Err(ImportError::WrongFormat);
    }
    if file.version > VERSION {
        return Err(ImportError::NewerVersion(file.version));
    }
    file.spaces
        .into_iter()
        .map(|d| {
            let name = d.name.clone();
            validate(d).map_err(|error| ImportError::Invalid { name, error })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, CommandSpec, Icon};

    fn sample() -> SpaceDraft {
        SpaceDraft {
            name: "Bases de datos".into(),
            aliases: vec!["bd".into()],
            icon: Icon::Emoji {
                value: "🗄️".into()
            },
            actions: vec![
                Action::OpenUrl {
                    url: "https://campus.es".into(),
                    place: None,
                },
                Action::RunCommand(CommandSpec {
                    program: "docker".into(),
                    args: vec!["compose".into(), "up".into()],
                    cwd: None,
                }),
            ],
        }
    }

    #[test]
    fn roundtrips() {
        let json = to_json(vec![sample()]);
        assert!(json.contains("\"format\": \"sift-spaces\""));
        assert_eq!(from_json(&json).unwrap(), vec![sample()]);
    }

    #[test]
    fn rejects_other_files() {
        assert!(matches!(from_json("{"), Err(ImportError::Json(_))));
        assert!(matches!(
            from_json(r#"{"format":"otra-app","version":1,"spaces":[]}"#),
            Err(ImportError::WrongFormat)
        ));
        assert!(matches!(
            from_json(r#"{"format":"sift-spaces","version":99,"spaces":[]}"#),
            Err(ImportError::NewerVersion(99))
        ));
    }

    #[test]
    fn validates_every_space() {
        let json = r#"{"format":"sift-spaces","version":1,"spaces":[
            {"name":"Bien","actions":[]},
            {"name":"Mal","actions":[{"kind":"openUrl","url":"file:///C:/a.exe"}]}
        ]}"#;
        match from_json(json) {
            Err(ImportError::Invalid { name, .. }) => assert_eq!(name, "Mal"),
            other => panic!("se esperaba un error de validación: {other:?}"),
        }
    }

    #[test]
    fn missing_optional_fields_take_defaults() {
        let json = r#"{"format":"sift-spaces","version":1,"spaces":[{"name":"Solo nombre"}]}"#;
        let spaces = from_json(json).unwrap();
        assert_eq!(spaces[0].icon, Icon::Auto);
        assert!(spaces[0].aliases.is_empty() && spaces[0].actions.is_empty());
    }
}
