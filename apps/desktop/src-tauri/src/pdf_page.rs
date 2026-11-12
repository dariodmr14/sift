//! Abrir un PDF en una página concreta con el lector predeterminado de Windows.
//!
//! Cada lector tiene su forma de decir «en la página 37»:
//! - Navegadores (Edge, Chrome, Brave, Firefox, Vivaldi): la dirección `file:///…#page=37`.
//! - Adobe Acrobat / Reader, Foxit, PDF-XChange: `/A "page=37" archivo`.
//! - SumatraPDF: `-page 37 archivo`.
//!
//! Se pregunta a Windows qué programa abre los `.pdf` (`AssocQueryStringW`, lo mismo que
//! usa «Abrir con»). Si es uno de esos, se lanza con programa y argumentos separados (sin
//! shell). Si no se reconoce o algo falla, se abre el PDF normal: nunca es un error.

use std::path::{Path, PathBuf};

/// Programa y argumentos para abrir `pdf` en `page` con el lector `reader` (su .exe), o
/// `None` si no se sabe hacerlo con ese lector.
pub fn page_command(reader: &Path, pdf: &Path, page: u32) -> Option<(PathBuf, Vec<String>)> {
    let exe = reader.file_name()?.to_string_lossy().to_ascii_lowercase();
    let pdf_arg = pdf.display().to_string();
    let args = match exe.as_str() {
        "msedge.exe" | "chrome.exe" | "brave.exe" | "firefox.exe" | "vivaldi.exe" => {
            vec![format!("{}#page={page}", file_url(pdf))]
        }
        "acrobat.exe" | "acrord32.exe" | "foxitpdfreader.exe" | "foxitreader.exe"
        | "foxitpdfeditor.exe" | "pdfxedit.exe" => {
            vec!["/A".into(), format!("page={page}"), pdf_arg]
        }
        "sumatrapdf.exe" => vec!["-page".into(), page.to_string(), pdf_arg],
        _ => return None,
    };
    Some((reader.to_path_buf(), args))
}

/// `C:\Mis apuntes\tema #4.pdf` → `file:///C:/Mis%20apuntes/tema%20%234.pdf`.
pub fn file_url(path: &Path) -> String {
    let mut url = String::from("file:///");
    for byte in path.display().to_string().replace('\\', "/").bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                url.push(byte as char)
            }
            other => url.push_str(&format!("%{other:02X}")),
        }
    }
    url
}

/// Abre `pdf` en `page` si el lector predeterminado lo permite; si no, normal.
pub fn open_pdf_at(pdf: &Path, page: u32) -> Result<(), String> {
    if let Some((program, args)) =
        platform::default_pdf_reader().and_then(|reader| page_command(&reader, pdf, page))
    {
        match std::process::Command::new(&program).args(&args).spawn() {
            Ok(_) => return Ok(()),
            Err(e) => tracing::warn!(
                program = %program.display(),
                error = %e,
                "no se pudo abrir el PDF en su página; se abre normal"
            ),
        }
    }
    use sift_spaces::Opener;
    crate::shell::WindowsOpener.open_path(pdf)
}

#[cfg(windows)]
#[allow(unsafe_code)] // Win32: el bloque explica por qué es correcto.
mod platform {
    use std::path::PathBuf;

    use windows::Win32::UI::Shell::{ASSOCF_NONE, ASSOCSTR_EXECUTABLE, AssocQueryStringW};
    use windows::core::{PWSTR, w};

    /// El .exe que abre los PDF (según Windows), o `None` si no hay o es una app de la
    /// Tienda (no tiene un .exe al que pasarle argumentos).
    pub fn default_pdf_reader() -> Option<PathBuf> {
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        // SAFETY: cadenas UTF-16 constantes terminadas en NUL; `buf` vive durante la
        // llamada y `len` dice su tamaño en caracteres, como pide la API.
        let result = unsafe {
            AssocQueryStringW(
                ASSOCF_NONE,
                ASSOCSTR_EXECUTABLE,
                w!(".pdf"),
                w!("open"),
                Some(PWSTR(buf.as_mut_ptr())),
                &mut len,
            )
        };
        if result.is_err() {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let path = PathBuf::from(String::from_utf16_lossy(&buf[..end]));
        path.is_file().then_some(path)
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn default_pdf_reader() -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(reader: &str) -> Option<Vec<String>> {
        page_command(
            Path::new(reader),
            Path::new(r"C:\Uni\Mis apuntes\tema #4.pdf"),
            37,
        )
        .map(|(_, args)| args)
    }

    #[test]
    fn browsers_get_a_file_url_with_the_page() {
        let args = cmd(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe").unwrap();
        assert_eq!(
            args,
            vec!["file:///C:/Uni/Mis%20apuntes/tema%20%234.pdf#page=37"]
        );
        assert!(cmd(r"C:\x\FIREFOX.EXE").is_some());
    }

    #[test]
    fn adobe_foxit_and_sumatra_get_their_own_arguments() {
        let path = r"C:\Uni\Mis apuntes\tema #4.pdf";
        assert_eq!(
            cmd(r"C:\Program Files\Adobe\Acrobat DC\Acrobat\Acrobat.exe").unwrap(),
            vec!["/A", "page=37", path]
        );
        assert_eq!(cmd(r"C:\x\FoxitPDFReader.exe").unwrap()[0], "/A");
        assert_eq!(
            cmd(r"C:\x\SumatraPDF.exe").unwrap(),
            vec!["-page", "37", path]
        );
    }

    /// Qué lector hay en este PC y cómo se abriría (a mano: depende del equipo).
    /// `cargo test -p sift-desktop pdf_page -- --ignored --nocapture`
    #[test]
    #[ignore = "depende del lector instalado en el equipo"]
    fn shows_the_default_reader_of_this_pc() {
        let reader = platform::default_pdf_reader();
        eprintln!("lector: {reader:?}");
        if let Some(r) = reader {
            eprintln!("orden: {:?}", page_command(&r, Path::new("a.pdf"), 37));
        }
    }

    #[test]
    fn unknown_readers_open_the_pdf_normally() {
        assert!(cmd(r"C:\x\lector-raro.exe").is_none());
    }

    #[test]
    fn accents_are_percent_encoded_in_file_urls() {
        assert_eq!(
            file_url(Path::new(r"C:\Teoría\a.pdf")),
            "file:///C:/Teor%C3%ADa/a.pdf"
        );
    }
}
