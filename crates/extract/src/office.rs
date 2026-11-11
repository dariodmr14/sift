//! Texto de documentos de Office y LibreOffice: Word (`.docx`), PowerPoint (`.pptx`) y
//! Writer (`.odt`).
//!
//! Los tres son un **zip con XML dentro**. No hace falta entender el formato entero:
//! el texto va en unas pocas etiquetas conocidas y el resto (estilos, imágenes) se ignora.
//!
//! | Formato | Dónde está el texto | Etiqueta de texto | Fin de párrafo |
//! |---|---|---|---|
//! | docx | `word/document.xml` | `w:t` | `w:p` |
//! | pptx | `ppt/slides/slideN.xml` (en orden) | `a:t` | `a:p` |
//! | odt | `content.xml` | todo lo que hay dentro de `text:p` / `text:h` | `text:p`, `text:h` |

use std::io::{BufReader, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::ExtractError;

/// Tope de lo que se descomprime de cada XML: un zip pequeño puede esconder gigas
/// («bomba zip»), y un documento real no tiene 64 MB de XML de texto.
const MAX_XML_BYTES: u64 = 64 * 1024 * 1024;

/// Qué etiquetas llevan el texto en cada formato.
struct Markup {
    /// Etiquetas cuyo contenido es texto visible. Vacío = cualquier texto dentro de un
    /// párrafo.
    text: &'static [&'static str],
    /// Etiquetas que acaban un párrafo (se traduce en un salto de línea).
    paragraph: &'static [&'static str],
    /// Etiquetas vacías que equivalen a un espacio en blanco (tabulador, salto).
    blank: &'static [&'static str],
}

const DOCX: Markup = Markup {
    text: &["w:t"],
    paragraph: &["w:p"],
    blank: &["w:tab", "w:br", "w:cr"],
};
const PPTX: Markup = Markup {
    text: &["a:t"],
    paragraph: &["a:p"],
    blank: &["a:br"],
};
const ODT: Markup = Markup {
    text: &[],
    paragraph: &["text:p", "text:h"],
    blank: &["text:tab", "text:line-break", "text:s"],
};

fn damaged(e: impl std::fmt::Display) -> ExtractError {
    ExtractError::Damaged(e.to_string())
}

/// Texto de un `.docx`, `.pptx` u `.odt` (según `ext`, en minúsculas). `Ok(None)` si la
/// extensión no es de las que se leen aquí.
pub fn extract_text<R: Read + Seek>(reader: R, ext: &str) -> Result<Option<String>, ExtractError> {
    let markup = match ext {
        "docx" => &DOCX,
        "pptx" => &PPTX,
        "odt" => &ODT,
        _ => return Ok(None),
    };
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| match e {
        // Los `.doc` antiguos renombrados y los archivos cortados no son un zip válido.
        zip::result::ZipError::Io(io) => ExtractError::Io(io),
        other => damaged(other),
    })?;
    let parts: Vec<String> = match ext {
        "docx" => vec!["word/document.xml".into()],
        "odt" => vec!["content.xml".into()],
        _ => slide_names(zip.file_names().filter_map(Result::ok)),
    };
    let mut text = String::new();
    for part in parts {
        let entry = match zip.by_name(&part) {
            Ok(entry) => entry,
            Err(zip::result::ZipError::UnsupportedArchive(reason))
                if reason.to_ascii_lowercase().contains("password") =>
            {
                return Err(ExtractError::Protected);
            }
            Err(e) => return Err(damaged(e)),
        };
        if entry.encrypted() {
            return Err(ExtractError::Protected);
        }
        read_xml(BufReader::new(entry.take(MAX_XML_BYTES)), markup, &mut text)?;
        text.push('\n');
    }
    Ok(Some(text))
}

/// Las diapositivas de un pptx, en su orden (`slide2` antes que `slide10`).
fn slide_names(names: impl Iterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut slides: Vec<(u32, String)> = names
        .filter_map(|name| {
            let name = name.as_ref();
            let number = name
                .strip_prefix("ppt/slides/slide")?
                .strip_suffix(".xml")?
                .parse()
                .ok()?;
            Some((number, name.to_owned()))
        })
        .collect();
    slides.sort_unstable();
    slides.into_iter().map(|(_, name)| name).collect()
}

/// Recorre un XML y va añadiendo a `out` el texto visible.
fn read_xml(
    source: impl std::io::BufRead,
    markup: &Markup,
    out: &mut String,
) -> Result<(), ExtractError> {
    let mut reader = Reader::from_reader(source);
    let mut buf = Vec::new();
    // Profundidad dentro de una etiqueta de texto (o de párrafo, si `text` está vacío).
    let mut in_text = 0usize;
    let opens_text = |name: &str| {
        if markup.text.is_empty() {
            markup.paragraph.contains(&name)
        } else {
            markup.text.contains(&name)
        }
    };
    loop {
        match reader.read_event_into(&mut buf).map_err(damaged)? {
            Event::Start(e) => {
                let name = e.name().as_ref().to_owned();
                if opens_text(&name) {
                    in_text += 1;
                }
            }
            Event::End(e) => {
                let name = e.name().as_ref().to_owned();
                if opens_text(&name) {
                    in_text = in_text.saturating_sub(1);
                }
                if markup.paragraph.contains(&name.as_str()) {
                    out.push('\n');
                }
            }
            Event::Empty(e) => {
                let name = e.name().as_ref().to_owned();
                if markup.blank.contains(&name.as_str()) {
                    out.push(' ');
                } else if markup.paragraph.contains(&name.as_str()) {
                    out.push('\n');
                }
            }
            Event::Text(t) if in_text > 0 => out.push_str(&t.xml10_content()),
            // `&amp;`, `&#233;`… llegan como eventos aparte.
            Event::GeneralRef(r) if in_text > 0 => {
                if let Some(ch) = r.resolve_char_ref().map_err(damaged)? {
                    out.push(ch);
                } else if let Some(s) =
                    quick_xml::escape::resolve_predefined_entity(&r.xml10_content())
                {
                    out.push_str(s);
                }
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
        buf.clear();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    /// Un zip en memoria con las entradas dadas (nombre, contenido).
    pub fn zip_with(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn text_of(bytes: Vec<u8>, ext: &str) -> String {
        extract_text(Cursor::new(bytes), ext).unwrap().unwrap()
    }

    #[test]
    fn reads_paragraphs_of_a_word_document() {
        let xml = r#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:t>Normalización</w:t></w:r><w:r><w:tab/><w:t xml:space="preserve"> de bases</w:t></w:r></w:p>
            <w:p><w:pPr><w:pStyle w:val="Titulo"/></w:pPr><w:r><w:t>3FN &amp; BCNF</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let text = text_of(zip_with(&[("word/document.xml", xml)]), "docx");
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(lines, ["Normalización  de bases", "3FN & BCNF"]);
    }

    #[test]
    fn reads_slides_in_order() {
        let slide =
            |t: &str| format!(r#"<p:sld xmlns:a="x"><a:p><a:r><a:t>{t}</a:t></a:r></a:p></p:sld>"#);
        let (one, two, ten) = (slide("uno"), slide("dos"), slide("diez"));
        let bytes = zip_with(&[
            ("ppt/slides/slide10.xml", &ten),
            ("ppt/slides/slide2.xml", &two),
            ("ppt/slides/slide1.xml", &one),
            ("ppt/notesSlides/notesSlide1.xml", "<a:t>no</a:t>"),
        ]);
        let text = text_of(bytes, "pptx");
        let words: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(words, ["uno", "dos", "diez"]);
    }

    #[test]
    fn reads_an_odt() {
        let xml = r#"<office:document-content xmlns:text="t" xmlns:office="o"><office:body><office:text>
            <text:h>Tema 4</text:h><text:p>Las <text:span>dependencias</text:span> funcionales</text:p>
        </office:text></office:body></office:document-content>"#;
        let text = text_of(zip_with(&[("content.xml", xml)]), "odt");
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(lines, ["Tema 4", "Las dependencias funcionales"]);
    }

    #[test]
    fn something_that_is_not_a_zip_is_damaged() {
        let e = extract_text(Cursor::new(b"esto no es un docx".to_vec()), "docx").unwrap_err();
        assert!(matches!(e, ExtractError::Damaged(_)), "{e:?}");
    }

    #[test]
    fn a_zip_without_the_document_is_damaged() {
        let e = extract_text(Cursor::new(zip_with(&[("otra.xml", "<a/>")])), "docx").unwrap_err();
        assert!(matches!(e, ExtractError::Damaged(_)), "{e:?}");
    }

    #[test]
    fn other_extensions_are_not_read_here() {
        assert!(
            extract_text(Cursor::new(Vec::new()), "xlsx")
                .unwrap()
                .is_none()
        );
    }
}
