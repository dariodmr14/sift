//! Texto de los PDF con **PDFium**, el motor de PDF de Chrome (crate `pdfium-render`).
//!
//! PDFium es una DLL (`pdfium.dll`) que se carga al arrancar; cómo llega esa DLL al
//! usuario está en el ADR 0008. Si no está, [`Engine::load`] falla y los PDF se indexan
//! solo por nombre.
//!
//! Un PDF escaneado (solo imágenes) no tiene texto: sale vacío, sin error. Leerlo
//! necesitaría OCR, que no entra en esta fase.

use std::path::Path;

use pdfium_render::prelude::{Pdfium, PdfiumError, PdfiumInternalError};

use crate::ExtractError;

pub struct Engine {
    pdfium: Pdfium,
}

impl Engine {
    /// Carga `pdfium.dll` de `dir`. Devuelve el motivo si no se puede.
    pub fn load(dir: &Path) -> Result<Engine, String> {
        let library = Pdfium::pdfium_platform_library_name_at_path(dir);
        if !library.is_file() {
            return Err(format!("no está {}", library.display()));
        }
        let bindings = Pdfium::bind_to_library(&library)
            .map_err(|e| format!("no se pudo cargar {}: {e}", library.display()))?;
        Ok(Engine {
            pdfium: Pdfium::new(bindings),
        })
    }

    /// Texto de todas las páginas, separadas por una línea en blanco. Deja de leer al
    /// llegar a `max_bytes` (un PDF de 2.000 páginas no hace falta entero).
    pub fn text(&self, path: &Path, max_bytes: usize) -> Result<String, ExtractError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(to_extract_error)?;
        let mut text = String::new();
        for page in document.pages().iter() {
            // Una página que no se deja leer no invalida las demás.
            if let Ok(page_text) = page.text() {
                text.push_str(&page_text.all());
                text.push_str("\n\n");
            }
            if text.len() >= max_bytes {
                break;
            }
        }
        Ok(text)
    }
}

fn to_extract_error(e: PdfiumError) -> ExtractError {
    match e {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
            ExtractError::Protected
        }
        PdfiumError::PdfiumLibraryInternalError(
            PdfiumInternalError::FormatError | PdfiumInternalError::PageError,
        ) => ExtractError::Damaged("no es un PDF válido".into()),
        PdfiumError::IoError(io) => ExtractError::Io(io),
        other => ExtractError::Damaged(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_and_format_errors_become_plain_reasons() {
        let protected = PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError);
        assert!(matches!(
            to_extract_error(protected),
            ExtractError::Protected
        ));
        let broken = PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError);
        assert!(matches!(to_extract_error(broken), ExtractError::Damaged(_)));
    }

    #[test]
    fn without_the_dll_the_engine_says_why() {
        let dir = tempfile::tempdir().unwrap();
        let reason = Engine::load(dir.path()).err().unwrap();
        assert!(reason.contains("pdfium"), "{reason}");
    }
}
