//! Espacios: el modelo (nombre, alias, icono, acciones), su validación, cómo se ejecutan
//! de forma segura y el formato de exportación.

pub mod export;
pub mod model;
pub mod run;
pub mod validate;

pub use model::{Action, CommandSpec, Icon, Space, SpaceDraft};
pub use run::{
    ActionOutcome, Opener, display_command, label, pending_approvals, run_action, run_actions,
};
pub use validate::{ValidationError, validate, validate_action};
