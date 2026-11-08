//! Tipos de dominio, configuración y errores compartidos.
//!
//! Vacío por ahora: se rellena en su fase (ver PLAN.md).

/// Versión de Sift, tomada de `Cargo.toml` en tiempo de compilación.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver() {
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "la versión debe tener formato X.Y.Z");
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
