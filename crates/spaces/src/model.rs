//! Qué es un Espacio: nombre, alias, icono y una lista ordenada de acciones.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Un Espacio guardado (con su id de la base de datos).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Space {
    /// Id de SQLite. En TypeScript es un `number` normal: nunca pasará de 2³¹.
    #[cfg_attr(feature = "specta", specta(type = i32))]
    pub id: i64,
    #[serde(flatten)]
    pub draft: SpaceDraft,
}

/// Un Espacio sin id: lo que se edita, se valida, se exporta y se importa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SpaceDraft {
    pub name: String,
    /// Otras palabras que lo activan ("bd", "bases").
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub icon: Icon,
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// Icono de un Espacio.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Icon {
    /// El icono real de Windows de la primera app, archivo o carpeta del Espacio.
    #[default]
    Auto,
    /// Sin icono.
    None,
    Emoji {
        value: String,
    },
    /// Icono de la librería (Lucide). Se guarda también su SVG para que la barra lo
    /// pinte sin cargar la librería entera.
    Library {
        name: String,
        svg: String,
    },
    /// Imagen propia (PNG, SVG o ICO), guardada dentro del Espacio en base64 para que
    /// viaje con él al exportar.
    Image {
        mime: String,
        data: String,
    },
}

/// Una acción de un Espacio. Se ejecutan en orden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Action {
    OpenFolder {
        path: PathBuf,
    },
    OpenFile {
        path: PathBuf,
    },
    OpenUrl {
        url: String,
    },
    /// Una app instalada, elegida en el selector. `app_id` es su identificador en la
    /// carpeta de aplicaciones de Windows (`shell:AppsFolder`): una ruta o un AUMID.
    #[serde(rename_all = "camelCase")]
    OpenApp {
        app_id: String,
        name: String,
    },
    RunCommand(CommandSpec),
}

/// Un comando: programa, argumentos y carpeta de trabajo, **siempre por separado**.
/// Nunca se junta en una sola cadena ni pasa por `cmd /c`: así ningún argumento puede
/// "salirse" y ejecutar otra cosa (inyección de comandos).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CommandSpec {
    /// Nombre de un `.exe` en el PATH (`docker`) o ruta completa. Los `.cmd`/`.bat`
    /// necesitan la extensión (`code.cmd`).
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

impl SpaceDraft {
    /// Comandos del Espacio, en orden.
    pub fn commands(&self) -> impl Iterator<Item = &CommandSpec> {
        self.actions.iter().filter_map(|a| match a {
            Action::RunCommand(c) => Some(c),
            _ => None,
        })
    }
}
