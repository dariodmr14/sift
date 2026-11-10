//! Índice de texto con Tantivy. Es **derivado**: se puede borrar y reconstruir desde los
//! fragmentos de SQLite (ver ADR 0006). Implementa el `IndexSink` del indexador.
//!
//! Un documento por fragmento (y uno sin texto para los archivos que solo se indexan por
//! nombre). Cada documento lleva también el nombre y la ruta del archivo, para encontrar
//! por nombre y pintar el resultado sin preguntar a SQLite.

use std::path::Path;
use std::sync::Mutex;

use sift_indexer::{IndexSink, IndexedFile};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, Query, TermQuery};
use tantivy::schema::{
    FAST, Field, INDEXED, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing,
    TextOptions, Value,
};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term, doc};

use crate::analyzer::{self, spanish};
use crate::query::ParsedQuery;
use crate::snippet::max_typos;

/// Versión del formato del índice. Si cambia (campos, analizador), el índice se borra y
/// se reconstruye desde SQLite al arrancar.
pub const SCHEMA_VERSION: u32 = 2;
const VERSION_FILE: &str = "sift-schema-version";
/// Memoria del escritor de Tantivy (lo mínimo razonable; el indexado no tiene prisa).
const WRITER_MEMORY: usize = 50_000_000;
/// El nombre del archivo pesa más que el contenido.
const NAME_BOOST: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
pub struct Fields {
    pub chunk_id: Field,
    pub file_id: Field,
    pub path: Field,
    pub name: Field,
    pub ext: Field,
    pub body: Field,
    /// Nombre y contenido sin raíz (minúsculas, sin tildes): erratas y prefijos.
    pub name_fold: Field,
    pub body_fold: Field,
}

/// Un fragmento encontrado por el recuperador léxico.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkHit {
    pub chunk_id: i64,
    pub file_id: i64,
    pub path: String,
    pub name: String,
    pub ext: String,
    pub body: String,
    pub score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("Tantivy: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("no se pudo abrir el índice: {0}")]
    Open(String),
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
}

pub struct TextIndex {
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    pub fields: Fields,
    /// `true` si el índice se acaba de crear (vacío): hay que reconstruirlo desde SQLite.
    pub needs_rebuild: bool,
}

fn schema() -> (Schema, Fields) {
    let mut b = Schema::builder();
    let text = |tokenizer: &str, stored: bool| {
        let options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(tokenizer)
                .set_index_option(IndexRecordOption::WithFreqs),
        );
        // El texto se guarda una sola vez (en los campos con raíz).
        if stored {
            options.set_stored()
        } else {
            options
        }
    };
    let fields = Fields {
        chunk_id: b.add_i64_field("chunk_id", STORED),
        file_id: b.add_i64_field("file_id", INDEXED | STORED | FAST),
        path: b.add_text_field("path", STRING | STORED),
        name: b.add_text_field("name", text(analyzer::NAME, true)),
        ext: b.add_text_field("ext", STRING | STORED),
        body: b.add_text_field("body", text(analyzer::NAME, true)),
        name_fold: b.add_text_field("name_fold", text(analyzer::FOLD_NAME, false)),
        body_fold: b.add_text_field("body_fold", text(analyzer::FOLD_NAME, false)),
    };
    (b.build(), fields)
}

impl TextIndex {
    /// Abre el índice de `dir` o lo crea. Si es de otra versión o está dañado, lo borra y
    /// empieza vacío con `needs_rebuild = true`.
    pub fn open(dir: &Path) -> Result<TextIndex, IndexError> {
        std::fs::create_dir_all(dir)?;
        let version_ok = std::fs::read_to_string(dir.join(VERSION_FILE))
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            == Some(SCHEMA_VERSION);
        let (schema, fields) = schema();
        let opened = if version_ok {
            Index::open_in_dir(dir).ok()
        } else {
            None
        };
        let (index, needs_rebuild) = match opened {
            Some(index) => (index, false),
            None => {
                std::fs::remove_dir_all(dir)?;
                std::fs::create_dir_all(dir)?;
                let index = Index::create_in_dir(dir, schema)
                    .map_err(|e| IndexError::Open(e.to_string()))?;
                std::fs::write(dir.join(VERSION_FILE), SCHEMA_VERSION.to_string())?;
                (index, true)
            }
        };
        Self::finish(index, fields, needs_rebuild)
    }

    /// Índice en memoria, para los tests.
    pub fn in_memory() -> Result<TextIndex, IndexError> {
        let (schema, fields) = schema();
        Self::finish(Index::create_in_ram(schema), fields, true)
    }

    fn finish(index: Index, fields: Fields, needs_rebuild: bool) -> Result<TextIndex, IndexError> {
        index.tokenizers().register(analyzer::NAME, spanish());
        index
            .tokenizers()
            .register(analyzer::FOLD_NAME, analyzer::fold_analyzer());
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let writer = index.writer(WRITER_MEMORY)?;
        Ok(TextIndex {
            reader,
            writer: Mutex::new(writer),
            fields,
            needs_rebuild,
        })
    }

    fn writer(&self) -> std::sync::MutexGuard<'_, IndexWriter> {
        self.writer.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Borra todo (antes de reconstruir).
    pub fn clear(&self) -> Result<(), IndexError> {
        self.writer().delete_all_documents()?;
        Ok(())
    }

    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    fn add_file(&self, file: &IndexedFile) -> Result<(), IndexError> {
        let f = self.fields;
        let writer = self.writer();
        writer.delete_term(Term::from_field_i64(f.file_id, file.file_id));
        let base = |chunk_id: i64, body: &str| {
            doc!(
                f.chunk_id => chunk_id,
                f.file_id => file.file_id,
                f.path => file.path.as_str(),
                f.name => file.name.as_str(),
                f.ext => file.ext.as_str(),
                f.body => body,
                f.name_fold => file.name.as_str(),
                f.body_fold => body,
            )
        };
        if file.chunks.is_empty() {
            // Solo por nombre: un documento sin texto (chunk_id 0 = "sin fragmento").
            writer.add_document(base(0, ""))?;
        }
        for (chunk_id, text) in &file.chunks {
            writer.add_document(base(*chunk_id, text))?;
        }
        Ok(())
    }

    /// Construye la consulta: todas las palabras tienen que aparecer (en el nombre o en el
    /// contenido), cada una exacta o con erratas; la última, también por prefijo.
    fn build_query(&self, q: &ParsedQuery) -> Box<dyn Query> {
        let f = self.fields;
        let boosted = |q: Box<dyn Query>, boost: f32| -> (Occur, Box<dyn Query>) {
            (Occur::Should, Box::new(BoostQuery::new(q, boost)))
        };
        let mut musts: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for term in &q.terms {
            let mut options: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for (stemmed, folded, boost) in [
                (f.name, f.name_fold, NAME_BOOST),
                (f.body, f.body_fold, 1.0),
            ] {
                // Raíz exacta: la mejor señal (vale el doble).
                let t = Term::from_field_text(stemmed, &term.stem);
                options.push(boosted(
                    Box::new(TermQuery::new(t, IndexRecordOption::WithFreqs)),
                    boost * 2.0,
                ));
                // Errata: sobre la palabra entera, nunca sobre la raíz.
                let d = max_typos(term.word.chars().count());
                if d > 0 {
                    let t = Term::from_field_text(folded, &term.word);
                    options.push(boosted(Box::new(FuzzyTermQuery::new(t, d, true)), boost));
                }
            }
            musts.push((Occur::Must, Box::new(BooleanQuery::new(options))));
        }
        if let Some(prefix) = &q.prefix {
            // La última palabra, a medio escribir: prefijo sobre la palabra entera.
            let options = [(f.name_fold, NAME_BOOST), (f.body_fold, 1.0)]
                .into_iter()
                .map(|(field, boost)| {
                    let t = Term::from_field_text(field, prefix);
                    boosted(Box::new(FuzzyTermQuery::new_prefix(t, 0, true)), boost)
                })
                .collect();
            musts.push((Occur::Must, Box::new(BooleanQuery::new(options))));
        }
        Box::new(BooleanQuery::new(musts))
    }

    /// Los `k` fragmentos que mejor encajan con la consulta.
    pub fn search_chunks(&self, q: &ParsedQuery, k: usize) -> Result<Vec<ChunkHit>, IndexError> {
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let searcher = self.reader.searcher();
        let query = self.build_query(q);
        let top = searcher.search(&*query, &TopDocs::with_limit(k).order_by_score())?;
        let f = self.fields;
        let mut hits = Vec::with_capacity(top.len());
        for (score, address) in top {
            let d: TantivyDocument = searcher.doc(address)?;
            let i64_of = |field| {
                d.get_first(field)
                    .and_then(|v| v.as_i64())
                    .unwrap_or_default()
            };
            let str_of = |field| {
                d.get_first(field)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned()
            };
            hits.push(ChunkHit {
                chunk_id: i64_of(f.chunk_id),
                file_id: i64_of(f.file_id),
                path: str_of(f.path),
                name: str_of(f.name),
                ext: str_of(f.ext),
                body: str_of(f.body),
                score,
            });
        }
        Ok(hits)
    }
}

impl IndexSink for TextIndex {
    fn upsert(&mut self, file: &IndexedFile) -> Result<(), String> {
        self.add_file(file).map_err(|e| e.to_string())
    }

    fn remove(&mut self, file_ids: &[i64]) -> Result<(), String> {
        let writer = self.writer();
        for id in file_ids {
            writer.delete_term(Term::from_field_i64(self.fields.file_id, *id));
        }
        Ok(())
    }

    fn commit(&mut self) -> Result<(), String> {
        self.writer().commit().map_err(|e| e.to_string())?;
        // Recargar el lector para que las búsquedas vean lo recién indexado.
        self.reader.reload().map_err(|e| e.to_string())
    }
}
