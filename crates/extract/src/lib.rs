//! Extracción de texto de archivos, y qué archivos merece la pena indexar.
//!
//! Solo se indexan **formatos útiles para buscar**: textos, código y documentos. Lo demás
//! (programas, compilados, imágenes, vídeos, cachés, archivos sin extensión…) ni se
//! guarda ni se cuenta: en una carpeta de la universidad real, 69.000 de 86.500 archivos
//! eran eso, y solo hacían ruido en los resultados ([`Kind::Other`]).
//!
//! Se lee el contenido de texto plano, Markdown y código, y el de los documentos: PDF
//! (con PDFium, [`pdf`]) y Word, PowerPoint y Writer ([`office`]). Los demás documentos
//! (hojas de cálculo, `.doc` antiguo, epub) se encuentran por nombre.
//!
//! La codificación importa: en Windows hay muchos apuntes en Windows-1252 (Bloc de notas
//! antiguo) y en UTF-16 (algunas herramientas). Leídos como UTF-8, «normalización» saldría
//! roto y no se encontraría.

use std::io::Read;
use std::path::Path;

pub mod office;
pub mod pdf;

/// Qué se hace con un archivo según su extensión.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Markdown,
    Code,
    /// Documento: se lee su contenido si es PDF, Word, PowerPoint u odt; si no, por nombre.
    Document,
    /// Imagen o vídeo: se puede buscar por nombre (opcional), nunca por contenido.
    Media,
    /// No se indexa: binarios, compilados, cachés…
    Other,
}

const TEXT: &[&str] = &["txt", "text", "csv", "tsv", "ini", "cfg", "conf", "rtf"];
/// Documentos. De los que no se sabe leer el contenido queda el nombre.
const DOCUMENT: &[&str] = &[
    "pdf", "doc", "docx", "odt", "ppt", "pptx", "odp", "xls", "xlsx", "ods", "epub",
];
/// Imágenes y vídeos: solo por nombre, y solo si el usuario no lo desactiva.
const MEDIA: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "svg", "tif", "tiff", "heic", "heif", "mp4", "mkv",
    "mov", "avi", "webm", "wmv", "m4v", "mpg", "mpeg",
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
    } else if DOCUMENT.contains(&ext) {
        Kind::Document
    } else if MEDIA.contains(&ext) {
        Kind::Media
    } else {
        Kind::Other
    }
}

/// ¿Merece la pena indexar este archivo? Textos, código y documentos siempre; imágenes
/// y vídeos (por nombre) solo con `media`.
pub fn is_indexable(path: &Path, media: bool) -> bool {
    match kind_for(&extension(path)) {
        Kind::Other => false,
        Kind::Media => media,
        _ => true,
    }
}

/// Extensión de un archivo en minúsculas y sin punto. Archivos sin extensión con nombre
/// conocido cuentan como código (`Dockerfile`, `Makefile`) o texto (`README`, `LICENSE`).
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
                Some("readme" | "leeme" | "license" | "licencia" | "notas" | "todo") => {
                    "txt".into()
                }
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
    #[error("está protegido con contraseña")]
    Protected,
    #[error("está dañado o incompleto: {0}")]
    Damaged(String),
}

/// Por qué no se pudo leer el contenido de un archivo, en categorías que entiende
/// cualquiera. Es lo que agrupa Ajustes → Carpetas («439 sin poder leer»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnreadReason {
    /// El contenido no es lo que dice la extensión (un `.txt` que en realidad es binario).
    Unsupported,
    TooLarge,
    NoPermission,
    /// Protegido con contraseña (documentos, Fase 3).
    Protected,
    /// Dañado o incompleto (documentos, Fase 3).
    Damaged,
    Other,
}

impl UnreadReason {
    pub const ALL: [UnreadReason; 6] = [
        UnreadReason::Unsupported,
        UnreadReason::TooLarge,
        UnreadReason::NoPermission,
        UnreadReason::Protected,
        UnreadReason::Damaged,
        UnreadReason::Other,
    ];

    /// Código estable con el que se guarda en la base de datos.
    pub fn code(self) -> &'static str {
        match self {
            UnreadReason::Unsupported => "unsupported",
            UnreadReason::TooLarge => "tooLarge",
            UnreadReason::NoPermission => "noPermission",
            UnreadReason::Protected => "protected",
            UnreadReason::Damaged => "damaged",
            UnreadReason::Other => "other",
        }
    }

    pub fn from_code(code: &str) -> UnreadReason {
        Self::ALL
            .into_iter()
            .find(|r| r.code() == code)
            .unwrap_or(UnreadReason::Other)
    }
}

impl ExtractError {
    pub fn reason(&self) -> UnreadReason {
        match self {
            ExtractError::TooLarge(_) => UnreadReason::TooLarge,
            ExtractError::Binary => UnreadReason::Unsupported,
            ExtractError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                UnreadReason::NoPermission
            }
            ExtractError::Io(_) => UnreadReason::Other,
            ExtractError::Protected => UnreadReason::Protected,
            ExtractError::Damaged(_) => UnreadReason::Damaged,
        }
    }
}

/// Tamaño máximo de un documento (PDF, Word…) para abrirlo. Son mucho más grandes que
/// su texto: un PDF de 40 MB con figuras puede tener 200 KB de texto.
pub const MAX_DOCUMENT_BYTES: u64 = 200 * 1024 * 1024;

/// El que lee archivos. Guarda el motor de PDF (PDFium), que se carga una vez.
///
/// Sin PDFium (no se encontró `pdfium.dll`), los PDF se indexan solo por nombre: la app
/// funciona igual y lo dice en Ajustes → Estado.
#[derive(Default)]
pub struct Extractor {
    pdf: Option<pdf::Engine>,
}

impl Extractor {
    /// Sin motor de PDF: textos, código y documentos de Office.
    pub fn new() -> Extractor {
        Extractor::default()
    }

    /// Con PDFium, buscando `pdfium.dll` en `dir`. Si no se puede cargar, devuelve el
    /// motivo y un extractor sin PDF.
    pub fn with_pdfium(dir: &Path) -> (Extractor, Option<String>) {
        match pdf::Engine::load(dir) {
            Ok(engine) => (Extractor { pdf: Some(engine) }, None),
            Err(reason) => (Extractor::new(), Some(reason)),
        }
    }

    pub fn reads_pdf(&self) -> bool {
        self.pdf.is_some()
    }

    /// ¿Sabe este extractor leer el contenido de `path`? (No, para imágenes, hojas de
    /// cálculo, o PDF sin PDFium: esos van por nombre.)
    pub fn reads_content(&self, path: &Path) -> bool {
        let ext = extension(path);
        match kind_for(&ext) {
            Kind::Text | Kind::Markdown | Kind::Code => true,
            Kind::Document => match ext.as_str() {
                "pdf" => self.pdf.is_some(),
                "docx" | "pptx" | "odt" => true,
                _ => false,
            },
            Kind::Media | Kind::Other => false,
        }
    }

    /// Lee el texto de un archivo si su tipo lo permite. `Ok(None)` = solo nombre.
    /// `max_bytes` es el tope para archivos de texto; los documentos tienen el suyo
    /// ([`MAX_DOCUMENT_BYTES`]) y su texto se recorta a `max_bytes`.
    pub fn extract(&self, path: &Path, max_bytes: u64) -> Result<Option<Extracted>, ExtractError> {
        let ext = extension(path);
        match kind_for(&ext) {
            Kind::Media | Kind::Other => Ok(None),
            Kind::Document => self.extract_document(path, &ext, max_bytes),
            Kind::Text | Kind::Markdown | Kind::Code => {
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
        }
    }

    fn extract_document(
        &self,
        path: &Path,
        ext: &str,
        max_text: u64,
    ) -> Result<Option<Extracted>, ExtractError> {
        let readable = match ext {
            "pdf" => self.pdf.is_some(),
            "docx" | "pptx" | "odt" => true,
            _ => false,
        };
        if !readable {
            return Ok(None);
        }
        let len = std::fs::metadata(path)?.len();
        if len > MAX_DOCUMENT_BYTES {
            return Err(ExtractError::TooLarge(len));
        }
        // El hash es del archivo (no del texto): si no cambia, no se vuelve a abrir.
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut std::fs::File::open(path)?, &mut hasher)?;
        let hash = hasher.finalize().to_hex().to_string();

        let text = match (&self.pdf, ext) {
            (Some(engine), "pdf") => engine.text(path, max_text as usize)?,
            _ => {
                let file = std::io::BufReader::new(std::fs::File::open(path)?);
                office::extract_text(file, ext)?.unwrap_or_default()
            }
        };
        Ok(Some(Extracted {
            text: truncate_at_char(text, max_text as usize),
            hash,
        }))
    }
}

/// Recorta un texto a `max` bytes sin partir un carácter.
fn truncate_at_char(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut cut = max;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
    }
    text
}

/// Lee el texto de un archivo sin motor de PDF (ver [`Extractor::extract`]).
pub fn extract(path: &Path, max_bytes: u64) -> Result<Option<Extracted>, ExtractError> {
    Extractor::new().extract(path, max_bytes)
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
        assert_eq!(kind_for("pdf"), Kind::Document);
        assert_eq!(kind_for(""), Kind::Other);
    }

    #[test]
    fn only_text_code_and_documents_are_worth_indexing() {
        for yes in [
            "Tema 4.pdf",
            "apuntes.md",
            "Main.java",
            "notas.docx",
            "README",
            "Makefile",
        ] {
            assert!(is_indexable(Path::new(yes), false), "{yes}");
        }
        for no in [
            "foto.png",
            "clase.mp4",
            "app.exe",
            "lib.dll",
            "Main.class",
            "datos.bin",
            "a.o",
            "practica.zip",
            "Player.cs.meta",
            "debug.log",
            "sin_extension",
        ] {
            assert!(!is_indexable(Path::new(no), false), "{no}");
        }
        // Imágenes y vídeos, solo si se pide (y nunca se lee su contenido).
        assert!(is_indexable(Path::new("foto.png"), true));
        assert!(is_indexable(Path::new("clase.MP4"), true));
        assert!(!is_indexable(Path::new("app.exe"), true));
        assert_eq!(kind_for("jpg"), Kind::Media);
    }

    #[test]
    fn word_documents_are_read_and_hashed_by_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Tema 4.docx");
        let xml = r#"<w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Dependencias funcionales</w:t></w:r></w:p></w:body></w:document>"#;
        std::fs::write(
            &path,
            office::tests::zip_with(&[("word/document.xml", xml)]),
        )
        .unwrap();
        let got = extract(&path, 1024).unwrap().unwrap();
        assert!(got.text.contains("Dependencias funcionales"));
        assert_eq!(
            got.hash,
            blake3::hash(&std::fs::read(&path).unwrap())
                .to_hex()
                .to_string()
        );
    }

    #[test]
    fn knows_which_contents_it_can_read() {
        let e = Extractor::new();
        assert!(e.reads_content(Path::new("a.md")));
        assert!(e.reads_content(Path::new("a.docx")));
        assert!(!e.reads_content(Path::new("a.pdf"))); // sin PDFium
        assert!(!e.reads_content(Path::new("a.xlsx")));
        assert!(!e.reads_content(Path::new("a.png")));
    }

    #[test]
    fn documents_without_a_reader_keep_only_their_name() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["notas.xlsx", "viejo.doc", "sin motor.pdf"] {
            let path = dir.path().join(name);
            std::fs::write(&path, b"x").unwrap();
            assert_eq!(extract(&path, 1024).unwrap(), None, "{name}");
        }
    }

    #[test]
    fn long_document_text_is_cut_on_a_character_boundary() {
        assert_eq!(truncate_at_char("añadir".into(), 2), "a"); // la ñ ocupa 2 bytes
        assert_eq!(truncate_at_char("corto".into(), 99), "corto");
    }

    #[test]
    fn errors_map_to_plain_reasons() {
        assert_eq!(ExtractError::Protected.reason(), UnreadReason::Protected);
        assert_eq!(
            ExtractError::Damaged("x".into()).reason(),
            UnreadReason::Damaged
        );
        assert_eq!(ExtractError::Binary.reason(), UnreadReason::Unsupported);
        assert_eq!(ExtractError::TooLarge(9).reason(), UnreadReason::TooLarge);
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(
            ExtractError::Io(denied).reason(),
            UnreadReason::NoPermission
        );
        for r in UnreadReason::ALL {
            assert_eq!(UnreadReason::from_code(r.code()), r);
        }
    }

    #[test]
    fn extension_is_lowercase_and_knows_dockerfile() {
        assert_eq!(extension(Path::new(r"C:\a\Tema.MD")), "md");
        assert_eq!(extension(Path::new(r"C:\a\Dockerfile")), "dockerfile");
        assert_eq!(extension(Path::new(r"C:\a\LEEME")), "txt");
        assert_eq!(extension(Path::new(r"C:\a\datos")), "");
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
