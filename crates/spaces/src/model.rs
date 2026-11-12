//! Qué es un Espacio: nombre, alias, icono y una lista ordenada de acciones.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::placement::Placement;

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
        /// Dónde colocar su ventana (Fase 6.5). `None` = donde la ponga Windows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<Placement>,
    },
    OpenFile {
        path: PathBuf,
        /// Dónde colocar su ventana (Fase 6.5). `None` = donde la ponga Windows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<Placement>,
    },
    OpenUrl {
        url: String,
        /// Dónde colocar su ventana (Fase 6.5). `None` = donde la ponga Windows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<Placement>,
    },
    /// Una app instalada, elegida en el selector. `app_id` es su identificador en la
    /// carpeta de aplicaciones de Windows (`shell:AppsFolder`): una ruta o un AUMID.
    #[serde(rename_all = "camelCase")]
    OpenApp {
        app_id: String,
        name: String,
        /// Dónde colocar su ventana (Fase 6.5). `None` = donde la ponga Windows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<Placement>,
    },
    RunCommand(CommandSpec),
    /// Un paso que espera a que algo esté listo antes de seguir (Fase 6, ADR 0012).
    Wait(WaitSpec),
}

/// «Esperar a que…» y qué hacer si no ocurre a tiempo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WaitSpec {
    pub until: WaitUntil,
    /// «Si no ocurre en X segundos…» (no cuenta en `Seconds`).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
    #[serde(default)]
    pub on_timeout: OnTimeout,
}

fn default_timeout() -> u32 {
    60
}

/// A qué se espera. Solo cosas **de tu equipo**: Sift no se conecta a internet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WaitUntil {
    /// Una web de tu equipo responde (`http://localhost:8080`).
    WebResponds { url: String },
    /// Un servicio de tu equipo acepta conexiones en un puerto (una base de datos…).
    ServiceAccepts { port: u16 },
    /// Termina el comando de justo antes (y sin error).
    CommandFinished,
    /// Unos segundos, sin comprobar nada.
    Seconds { secs: u32 },
}

/// Qué pasa si no ocurre a tiempo (o el comando falla).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum OnTimeout {
    /// No seguir y avisar (lo normal: abrir la web de un contenedor que no arrancó no
    /// sirve de nada).
    #[default]
    Stop,
    /// Seguir con lo demás igualmente.
    Continue,
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
