//! Patrones a ignorar, como en `.gitignore` (simplificado):
//! - Sin barra (`node_modules`, `*.tmp`, `~$*`): se comparan con **cada nombre** de la
//!   ruta, así que ignoran esa carpeta o archivo esté donde esté.
//! - Con barra (`Uni/Antiguo`, `**/build/*.log`): se comparan con la ruta relativa a la
//!   carpeta indexada.
//!
//! Sin distinguir mayúsculas, como Windows.

use std::path::Path;

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

#[derive(Debug, thiserror::Error)]
#[error("patrón no válido «{pattern}»: {reason}")]
pub struct PatternError {
    pub pattern: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct IgnoreRules {
    by_name: GlobSet,
    by_path: GlobSet,
}

fn glob(pattern: &str) -> Result<Glob, PatternError> {
    GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()
        .map_err(|e| PatternError {
            pattern: pattern.to_owned(),
            reason: e.kind().to_string(),
        })
}

impl IgnoreRules {
    pub fn new(patterns: &[String]) -> Result<IgnoreRules, PatternError> {
        let (mut by_name, mut by_path) = (GlobSetBuilder::new(), GlobSetBuilder::new());
        for p in patterns.iter().map(|p| p.trim()).filter(|p| !p.is_empty()) {
            let p = p.replace('\\', "/");
            let p = p.trim_matches('/');
            if p.contains('/') {
                by_path.add(glob(p)?);
            } else {
                by_name.add(glob(p)?);
            }
        }
        let build = |b: GlobSetBuilder| {
            b.build().map_err(|e| PatternError {
                pattern: String::new(),
                reason: e.to_string(),
            })
        };
        Ok(IgnoreRules {
            by_name: build(by_name)?,
            by_path: build(by_path)?,
        })
    }

    /// ¿Se ignora esta ruta (relativa a la carpeta indexada)?
    pub fn is_ignored(&self, relative: &Path) -> bool {
        if relative
            .components()
            .any(|c| self.by_name.is_match(c.as_os_str()))
        {
            return true;
        }
        let slashed = relative.to_string_lossy().replace('\\', "/");
        self.by_path.is_match(slashed.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(p: &[&str]) -> IgnoreRules {
        IgnoreRules::new(&p.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn names_match_anywhere_in_the_path() {
        let r = rules(&["node_modules", "*.tmp", "~$*"]);
        assert!(r.is_ignored(Path::new(r"web\node_modules\react\index.js")));
        assert!(r.is_ignored(Path::new(r"node_modules")));
        assert!(r.is_ignored(Path::new(r"BD\copia.TMP")));
        assert!(r.is_ignored(Path::new(r"BD\~$Tema 4.docx"))); // temporales de Office
        assert!(!r.is_ignored(Path::new(r"BD\Tema 4.docx")));
        assert!(!r.is_ignored(Path::new(r"BD\node_modules_notas.md")));
    }

    #[test]
    fn paths_match_relative_to_the_root() {
        let r = rules(&["Antiguo/*", "**/build/*.log"]);
        assert!(r.is_ignored(Path::new(r"Antiguo\tema.md")));
        assert!(!r.is_ignored(Path::new(r"BD\Antiguo\tema.md")));
        assert!(r.is_ignored(Path::new(r"x\y\build\out.log")));
    }

    #[test]
    fn is_case_insensitive_like_windows() {
        assert!(rules(&[".GIT"]).is_ignored(Path::new(r"proyecto\.git\HEAD")));
    }

    #[test]
    fn invalid_patterns_are_reported() {
        let err = IgnoreRules::new(&["[".to_string()]).unwrap_err();
        assert_eq!(err.pattern, "[");
    }
}
