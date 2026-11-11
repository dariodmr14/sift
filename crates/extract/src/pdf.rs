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

    /// Un PDF mínimo de una página con `text` (Helvetica), con su tabla de referencias
    /// calculada a mano: así el test no necesita archivos de muestra.
    fn tiny_pdf(text: &str) -> Vec<u8> {
        let stream = format!("BT /F1 18 Tf 72 720 Td ({text}) Tj ET");
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R              /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_string(),
            format!("<< /Length {} >>
stream
{stream}
endstream", stream.len()),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let mut pdf = b"%PDF-1.4
"
        .to_vec();
        let mut offsets = Vec::new();
        for (i, body) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend(
                format!(
                    "{} 0 obj
{body}
endobj
",
                    i + 1
                )
                .bytes(),
            );
        }
        let xref = pdf.len();
        pdf.extend(
            format!(
                "xref
0 {}
0000000000 65535 f 
",
                objects.len() + 1
            )
            .bytes(),
        );
        for offset in offsets {
            pdf.extend(
                format!(
                    "{offset:010} 00000 n 
"
                )
                .bytes(),
            );
        }
        pdf.extend(
            format!(
                "trailer
<< /Size {} /Root 1 0 R >>
startxref
{xref}
%%EOF
",
                objects.len() + 1
            )
            .bytes(),
        );
        pdf
    }

    /// Dónde está `pdfium.dll` en el repositorio (la baja `scripts/fetch-pdfium.ps1`).
    fn repo_pdfium() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/pdfium")
    }

    /// Con PDFium de verdad: lee el texto de un PDF, y uno roto da «dañado». Si la DLL no
    /// está (no se ha ejecutado el script), se salta avisando; la CI sí la tiene.
    #[test]
    fn reads_a_real_pdf_with_pdfium() {
        let engine = match Engine::load(&repo_pdfium()) {
            Ok(engine) => engine,
            Err(reason) => {
                eprintln!("se salta: {reason} (ejecuta scripts/fetch-pdfium.ps1)");
                return;
            }
        };
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("tema.pdf");
        std::fs::write(&good, tiny_pdf("Normalizacion y formas normales")).unwrap();
        let text = engine.text(&good, 1024).unwrap();
        assert!(text.contains("Normalizacion y formas normales"), "{text:?}");

        let broken = dir.path().join("roto.pdf");
        std::fs::write(&broken, b"%PDF-1.4 esto se corta aqui").unwrap();
        assert!(matches!(
            engine.text(&broken, 1024),
            Err(ExtractError::Damaged(_))
        ));
    }

    #[test]
    fn without_the_dll_the_engine_says_why() {
        let dir = tempfile::tempdir().unwrap();
        let reason = Engine::load(dir.path()).err().unwrap();
        assert!(reason.contains("pdfium"), "{reason}");
    }
}
