//! Extracción de texto de archivos.
//!
//! Fase 2: texto plano, Markdown y código. PDF y Word llegan en la Fase 3; mientras, se
//! indexan solo por nombre ([`Kind::NameOnly`]).
//!
//! La codificación importa: en Windows hay muchos apuntes en Windows-1252 (Bloc de notas
//! antiguo) y en UTF-16 (algunas herramientas). Leídos como UTF-8, «normalización» saldría
//! roto y no se encontraría.

use std::io::Read;
use std::path::Path;

/// Qué se hace con un archivo según su extensión.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Markdown,
    Code,
    /// Se indexa por nombre, sin contenido (por ahora: PDF, Word, imágenes…).
    NameOnly,
}

const TEXT: &[&str] = &[
    "txt", "text", "log", "csv", "tsv", "ini", "cfg", "conf", "rtf",
];
const MARKDOWN: &[&str] = &["md", "markdown", "mdx", "rst", "adoc", "org", "tex"];
const CODE: &[&str] = &[
    "rs",
    "ts",
    "tsx",
    "js",
    "jsx",
    "mjs",
    "cjs",
    "py",
    "java",
    "kt",
    "kts",
    "c",
    "h",
    "cpp",
    "hpp",
    "cc",
    "cs",
    "go",
    "rb",
    "php",
    "swift",
    "scala",
    "sql",
    "html",
    "htm",
    "css",
    "scss",
    "json",
    "yaml",
    "yml",
    "toml",
    "xml",
    "sh",
    "bash",
    "ps1",
    "bat",
    "cmd",
    "r",
    "m",
    "lua",
    "dart",
    "vue",
    "svelte",
    "gradle",
    "properties",
    "dockerfile",
    "makefile",
    "ipynb",
    "pl",
    "hs",
    "ml",
    "asm",
    "s",
    "v",
    "vhd",
    "vhdl",
];

pub fn kind_for(ext: &str) -> Kind {
    let ext = ext.to_ascii_lowercase();
    let ext = ext.as_str();
    if MARKDOWN.contains(&ext) {
        Kind::Markdown
    } else if CODE.contains(&ext) {
        Kind::Code
    } else if TEXT.contains(&ext) {
        Kind::Text
    } else {
        Kind::NameOnly
    }
}

/// Extensión de un archivo en minúsculas y sin punto. Archivos sin extensión con nombre
/// conocido (`Dockerfile`, `Makefile`) cuentan como código.
pub fn extension(path: &Path) -> String {
    match path.extension() {
        Some(e) => e.to_string_lossy().to_ascii_lowercase(),
        None => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase());
            match name.as_deref() {
                Some("dockerfile") => "dockerfile".into(),
                Some("makefile") => "makefile".into(),
                _ => String::new(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub text: String,
    /// blake3 del contenido en bytes: si no cambia, no hace falta reprocesar.
    pub hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("demasiado grande para leer su contenido ({0} bytes)")]
    TooLarge(u64),
    #[error("parece un archivo binario")]
    Binary,
    #[error("no se pudo leer: {0}")]
    Io(#[from] std::io::Error),
}

/// Lee el texto de un archivo si su tipo lo permite. `Ok(None)` = solo nombre.
pub fn extract(path: &Path, max_bytes: u64) -> Result<Option<Extracted>, ExtractError> {
    if kind_for(&extension(path)) == Kind::NameOnly {
        return Ok(None);
    }
    let file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    if len > max_bytes {
        return Err(ExtractError::TooLarge(len));
    }
    let mut bytes = Vec::with_capacity(len as usize);
    file.take(max_bytes).read_to_end(&mut bytes)?;
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let text = decode(&bytes)?;
    Ok(Some(Extracted { text, hash }))
}

/// Decodifica bytes a texto: marca BOM de UTF-8/UTF-16 si la hay; si no, UTF-8 si es
/// válido; y si no, Windows-1252 (lo habitual en archivos antiguos de Windows en español).
pub fn decode(bytes: &[u8]) -> Result<String, ExtractError> {
    if let Some((encoding, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[bom_len..]);
        return Ok(text.into_owned());
    }
    // Sin BOM, un byte NUL en el principio delata un binario (o UTF-16 sin BOM, raro).
    if bytes.iter().take(8192).any(|&b| b == 0) {
        return Err(ExtractError::Binary);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok(s.to_owned()),
        Err(_) => {
            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            Ok(text.into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_by_extension() {
        assert_eq!(kind_for("MD"), Kind::Markdown);
        assert_eq!(kind_for("rs"), Kind::Code);
        assert_eq!(kind_for("txt"), Kind::Text);
        assert_eq!(kind_for("pdf"), Kind::NameOnly);
        assert_eq!(kind_for(""), Kind::NameOnly);
    }

    #[test]
    fn extension_is_lowercase_and_knows_dockerfile() {
        assert_eq!(extension(Path::new(r"C:\a\Tema.MD")), "md");
        assert_eq!(extension(Path::new(r"C:\a\Dockerfile")), "dockerfile");
        assert_eq!(extension(Path::new(r"C:\a\LEEME")), "");
    }

    #[test]
    fn decodes_utf8_with_and_without_bom() {
        assert_eq!(decode("normalización".as_bytes()).unwrap(), "normalización");
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend_from_slice("ñandú".as_bytes());
        assert_eq!(decode(&bom).unwrap(), "ñandú");
    }

    #[test]
    fn decodes_utf16_with_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for u in "Tema 3".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode(&bytes).unwrap(), "Tema 3");
    }

    #[test]
    fn falls_back_to_windows_1252() {
        // "normalización" guardado por el Bloc de notas antiguo: la ó es 0xF3.
        let bytes = b"normalizaci\xF3n";
        assert_eq!(decode(bytes).unwrap(), "normalización");
    }

    #[test]
    fn rejects_binaries() {
        assert!(matches!(
            decode(b"MZ\x90\x00\x03"),
            Err(ExtractError::Binary)
        ));
    }

    #[test]
    fn extracts_files_and_respects_limits() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("tema.md");
        std::fs::write(&md, "# Normalización\n3FN").unwrap();
        let e = extract(&md, 1_000).unwrap().unwrap();
        assert!(e.text.contains("3FN"));
        assert_eq!(e.hash.len(), 64);
        assert!(matches!(extract(&md, 5), Err(ExtractError::TooLarge(_))));

        let pdf = dir.path().join("tema.pdf");
        std::fs::write(&pdf, b"%PDF-1.7").unwrap();
        assert!(extract(&pdf, 1_000).unwrap().is_none()); // solo por nombre
    }

    #[test]
    fn same_content_same_hash() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
        std::fs::write(&a, "igual").unwrap();
        std::fs::write(&b, "igual").unwrap();
        assert_eq!(
            extract(&a, 100).unwrap().unwrap().hash,
            extract(&b, 100).unwrap().unwrap().hash
        );
    }
}
