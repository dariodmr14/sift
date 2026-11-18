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
/// v3: cada fragmento lleva el hash de su texto (la clave de su vector, Fase 4).
/// v4: ids en columnas rápidas, para no cargar documentos hasta mostrarlos.
pub const SCHEMA_VERSION: u32 = 4;
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
    /// Hash del texto del fragmento (`sift_storage::text_hash`): une el fragmento con su
    /// vector. 0 en los documentos sin texto.
    pub text_hash: Field,
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
        chunk_id: b.add_i64_field("chunk_id", INDEXED | STORED | FAST),
        file_id: b.add_i64_field("file_id", INDEXED | STORED | FAST),
        path: b.add_text_field("path", STRING | STORED),
        name: b.add_text_field("name", text(analyzer::NAME, true)),
        ext: b.add_text_field("ext", STRING | STORED),
        body: b.add_text_field("body", text(analyzer::NAME, true)),
        name_fold: b.add_text_field("name_fold", text(analyzer::FOLD_NAME, false)),
        body_fold: b.add_text_field("body_fold", text(analyzer::FOLD_NAME, false)),
        text_hash: b.add_i64_field("text_hash", INDEXED | STORED | FAST),
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
            let hash = if body.is_empty() {
                0
            } else {
                sift_storage::text_hash(body)
            };
            doc!(
                f.text_hash => hash,
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

    /// Construye la consulta: cada palabra tiene que aparecer (en el nombre o en el
    /// contenido), exacta o con erratas; la última, también por prefijo. Con `missing`, se
    /// admite que falten hasta tantas palabras (ver [`relaxed_missing`]).
    fn build_query(&self, q: &ParsedQuery, missing: usize) -> Box<dyn Query> {
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
                // Errata: sobre la palabra entera, nunca sobre la raíz. En la consulta
                // relajada no: con erratas y sin exigir todas las palabras, cada una abre
                // miles de documentos y la búsqueda pasa de 50 ms (medido con `eval/`).
                let d = max_typos(term.word.chars().count());
                if d > 0 && missing == 0 {
                    let t = Term::from_field_text(folded, &term.word);
                    options.push(boosted(Box::new(FuzzyTermQuery::new(t, d, true)), boost));
                }
            }
            musts.push((Occur::Should, Box::new(BooleanQuery::new(options))));
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
            musts.push((Occur::Should, Box::new(BooleanQuery::new(options))));
        }
        let required = musts.len().saturating_sub(missing).max(1);
        let words: Box<dyn Query> =
            Box::new(BooleanQuery::with_minimum_required_clauses(musts, required));
        self.only_types(words, &q.exts)
    }

    /// `query`, pero solo en archivos con alguna de estas extensiones (vacío = todos).
    fn only_types(&self, query: Box<dyn Query>, exts: &[String]) -> Box<dyn Query> {
        if exts.is_empty() {
            return query;
        }
        let types: Vec<(Occur, Box<dyn Query>)> = exts
            .iter()
            .map(|e| {
                let t = Term::from_field_text(self.fields.ext, e);
                let q: Box<dyn Query> = Box::new(TermQuery::new(t, IndexRecordOption::Basic));
                (Occur::Should, q)
            })
            .collect();
        Box::new(BooleanQuery::new(vec![
            (Occur::Must, query),
            (Occur::Must, Box::new(BooleanQuery::new(types))),
        ]))
    }

    /// Los `k` fragmentos que mejor encajan con la consulta. Solo sus ids (columnas
    /// rápidas): el texto se carga después, y solo de lo que se va a mostrar.
    pub fn search_chunks(&self, q: &ParsedQuery, k: usize) -> Result<Vec<Candidate>, IndexError> {
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let searcher = self.reader.searcher();
        let mut ids = Ids::new(&searcher);
        let strict = searcher.search(
            &*self.build_query(q, 0),
            &TopDocs::with_limit(k).order_by_score(),
        )?;
        let mut found = candidates(&mut ids, &strict)?;
        // Una frase («cómo evitar redundancia en tablas») rara vez tiene todas sus palabras
        // en el mismo fragmento: sin esto, las palabras no aportan nada. Si faltan
        // resultados, detrás de los que lo tienen todo van los que tienen casi todo.
        let words = q.terms.len() + usize::from(q.prefix.is_some());
        let missing = relaxed_missing(words);
        if found.len() < k && missing > 0 {
            let seen: std::collections::HashSet<_> = strict.iter().map(|(_, a)| *a).collect();
            let relaxed: Vec<_> = searcher
                .search(
                    &*self.build_query(q, missing),
                    &TopDocs::with_limit(k).order_by_score(),
                )?
                .into_iter()
                .filter(|(_, a)| !seen.contains(a))
                .take(k - found.len())
                .collect();
            found.extend(candidates(&mut ids, &relaxed)?);
        }
        Ok(found)
    }

    /// Los fragmentos cuyos textos son estos (hash, parecido), en ese orden. Un texto
    /// repetido en varios archivos da un fragmento por archivo. Como mucho `k`.
    pub fn chunks_by_hashes(
        &self,
        hashes: &[(i64, f32)],
        k: usize,
        exts: &[String],
    ) -> Result<Vec<Candidate>, IndexError> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }
        let f = self.fields;
        let searcher = self.reader.searcher();
        let query = self.only_types(
            Box::new(BooleanQuery::new(
                hashes
                    .iter()
                    .map(|(hash, _)| {
                        let t = Term::from_field_i64(f.text_hash, *hash);
                        let q: Box<dyn Query> =
                            Box::new(TermQuery::new(t, IndexRecordOption::Basic));
                        (Occur::Should, q)
                    })
                    .collect(),
            )),
            exts,
        );
        // Sin orden propio: las copias se ordenan después por el parecido de su texto.
        let found = searcher.search(&*query, &TopDocs::with_limit(k * 4).order_by_score())?;
        let rank: std::collections::HashMap<i64, (usize, f32)> = hashes
            .iter()
            .enumerate()
            .map(|(i, (h, s))| (*h, (i, *s)))
            .collect();
        let mut ids = Ids::new(&searcher);
        let mut hits = Vec::with_capacity(found.len());
        for (_, address) in found {
            let (file_id, chunk_id, hash) = ids.of(address)?;
            if let Some((position, similarity)) = rank.get(&hash) {
                hits.push((
                    *position,
                    Candidate {
                        file_id,
                        chunk_id,
                        score: *similarity,
                    },
                ));
            }
        }
        // Las copias de un mismo texto, por id: el mismo orden aunque se rehaga el índice.
        hits.sort_by_key(|(position, c)| (*position, c.file_id, c.chunk_id));
        Ok(hits.into_iter().map(|(_, h)| h).take(k).collect())
    }

    /// Carga nombre, ruta y texto de unos fragmentos (los que se van a mostrar). Es lo
    /// caro de una búsqueda, por eso se hace al final y con pocos.
    pub fn fetch(&self, keys: &[(i64, i64)]) -> Result<Vec<ChunkHit>, IndexError> {
        let f = self.fields;
        let searcher = self.reader.searcher();
        let mut out = Vec::with_capacity(keys.len());
        for &(file_id, chunk_id) in keys {
            let query = BooleanQuery::new(vec![
                (
                    Occur::Must,
                    Box::new(TermQuery::new(
                        Term::from_field_i64(f.file_id, file_id),
                        IndexRecordOption::Basic,
                    )) as Box<dyn Query>,
                ),
                (
                    Occur::Must,
                    Box::new(TermQuery::new(
                        Term::from_field_i64(f.chunk_id, chunk_id),
                        IndexRecordOption::Basic,
                    )),
                ),
            ]);
            let found = searcher.search(&query, &TopDocs::with_limit(1).order_by_score())?;
            if let Some((_, address)) = found.into_iter().next() {
                let d: TantivyDocument = searcher.doc(address)?;
                let str_of = |field| {
                    d.get_first(field)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_owned()
                };
                out.push(ChunkHit {
                    chunk_id,
                    file_id,
                    path: str_of(f.path),
                    name: str_of(f.name),
                    ext: str_of(f.ext),
                    body: str_of(f.body),
                    score: 0.0,
                });
            }
        }
        Ok(out)
    }
}

/// Un fragmento candidato: solo sus ids y su puntuación (sin cargar el documento).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub file_id: i64,
    pub chunk_id: i64,
    pub score: f32,
}

/// Lee los ids de un documento de las columnas rápidas de Tantivy, que están en memoria:
/// mucho más barato que cargar el documento entero (su texto incluido).
struct Ids<'a> {
    searcher: &'a tantivy::Searcher,
    columns: std::collections::HashMap<u32, [tantivy::columnar::Column<i64>; 3]>,
}

impl<'a> Ids<'a> {
    fn new(searcher: &'a tantivy::Searcher) -> Self {
        Ids {
            searcher,
            columns: std::collections::HashMap::new(),
        }
    }

    /// (file_id, chunk_id, text_hash)
    fn of(&mut self, address: tantivy::DocAddress) -> Result<(i64, i64, i64), IndexError> {
        if !self.columns.contains_key(&address.segment_ord) {
            let fast = self
                .searcher
                .segment_reader(address.segment_ord)
                .fast_fields();
            self.columns.insert(
                address.segment_ord,
                [
                    fast.i64("file_id")?,
                    fast.i64("chunk_id")?,
                    fast.i64("text_hash")?,
                ],
            );
        }
        let [file_id, chunk_id, hash] = &self.columns[&address.segment_ord];
        let get = |c: &tantivy::columnar::Column<i64>| c.first(address.doc_id).unwrap_or_default();
        Ok((get(file_id), get(chunk_id), get(hash)))
    }
}

impl TextIndex {
    pub fn remove_files(&self, file_ids: &[i64]) {
        let writer = self.writer();
        for id in file_ids {
            writer.delete_term(Term::from_field_i64(self.fields.file_id, *id));
        }
    }

    pub fn commit_and_reload(&self) -> Result<(), IndexError> {
        self.writer().commit()?;
        // Recargar el lector para que las búsquedas vean lo recién indexado.
        self.reader.reload()?;
        Ok(())
    }
}

/// El índice compartido (`&TextIndex`) también sirve de destino del indexado: el escritor
/// va dentro de un `Mutex`, así que el hilo del indexado escribe mientras las búsquedas
/// leen, sin bloquearse entre ellos.
impl IndexSink for &TextIndex {
    fn upsert(&mut self, file: &IndexedFile) -> Result<(), String> {
        self.add_file(file).map_err(|e| e.to_string())
    }

    fn remove(&mut self, file_ids: &[i64]) -> Result<(), String> {
        self.remove_files(file_ids);
        Ok(())
    }

    fn commit(&mut self) -> Result<(), String> {
        self.commit_and_reload().map_err(|e| e.to_string())
    }
}

impl IndexSink for TextIndex {
    fn upsert(&mut self, file: &IndexedFile) -> Result<(), String> {
        (&*self).upsert(file)
    }

    fn remove(&mut self, file_ids: &[i64]) -> Result<(), String> {
        (&*self).remove(file_ids)
    }

    fn commit(&mut self) -> Result<(), String> {
        (&*self).commit()
    }
}

/// Cuántas palabras pueden faltar en la consulta relajada: una, y solo con tres o más
/// (con dos, faltar una es buscar cualquiera de las dos). Calibrado con `eval/` (ADR 0010).
fn relaxed_missing(words: usize) -> usize {
    if words >= 3 { 1 } else { 0 }
}

/// Los ids de unos documentos encontrados, el mejor primero. A igual puntuación (copias
/// del mismo texto en varias carpetas), por id: si no, el orden dependería de cómo quedó
/// el índice por dentro, que cambia al rehacerlo, y los resultados también.
fn candidates(
    ids: &mut Ids,
    docs: &[(f32, tantivy::DocAddress)],
) -> Result<Vec<Candidate>, IndexError> {
    let mut out = docs
        .iter()
        .map(|(score, address)| {
            let (file_id, chunk_id, _) = ids.of(*address)?;
            Ok(Candidate {
                file_id,
                chunk_id,
                score: *score,
            })
        })
        .collect::<Result<Vec<_>, IndexError>>()?;
    out.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.file_id.cmp(&b.file_id))
            .then(a.chunk_id.cmp(&b.chunk_id))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(id: i64, path: &str, name: &str, chunks: &[(i64, &str)]) -> IndexedFile {
        IndexedFile {
            file_id: id,
            path: path.to_string(),
            name: name.to_string(),
            ext: name.rsplit('.').next().unwrap_or("").to_ascii_lowercase(),
            chunks: chunks.iter().map(|(c, t)| (*c, t.to_string())).collect(),
        }
    }

    fn found(idx: &TextIndex, q: &str) -> Vec<(i64, i64)> {
        idx.search_chunks(&ParsedQuery::parse(q), 10)
            .unwrap()
            .into_iter()
            .map(|c| (c.file_id, c.chunk_id))
            .collect()
    }

    fn ballena() -> IndexedFile {
        file(
            1,
            r"C:\Docs\informe.txt",
            "informe.txt",
            &[(10, "la ballena azul nada despacio")],
        )
    }

    #[test]
    fn indexed_file_is_found_by_content_and_by_name() {
        // Un archivo se encuentra por una palabra de su texto y por su nombre.
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&ballena()).unwrap();
        idx.commit().unwrap();
        assert_eq!(found(&idx, "ballena "), vec![(1, 10)]);
        assert_eq!(found(&idx, "informe "), vec![(1, 10)]);
    }

    #[test]
    fn removed_file_is_no_longer_found() {
        // Tras borrar y confirmar, el archivo no sale y el recuento baja.
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&ballena()).unwrap();
        idx.commit().unwrap();
        assert_eq!(idx.num_docs(), 1);
        idx.remove_files(&[1]);
        idx.commit_and_reload().unwrap();
        assert!(found(&idx, "ballena ").is_empty());
        assert_eq!(idx.num_docs(), 0);
    }

    #[test]
    fn upsert_replaces_previous_chunks() {
        // Rehacer el mismo file_id con otro texto no deja fragmentos viejos.
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&ballena()).unwrap();
        idx.commit().unwrap();
        idx.upsert(&file(
            1,
            r"C:\Docs\informe.txt",
            "informe.txt",
            &[(11, "el delfin salta alto")],
        ))
        .unwrap();
        idx.commit().unwrap();
        assert!(found(&idx, "ballena ").is_empty());
        assert_eq!(found(&idx, "delfin "), vec![(1, 11)]);
        assert_eq!(idx.num_docs(), 1);
    }

    #[test]
    fn moved_file_is_found_by_new_name_only() {
        // Mover/renombrar: mismo id, otra ruta y nombre.
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&file(1, r"C:\A\viejo.txt", "viejo.txt", &[]))
            .unwrap();
        idx.commit().unwrap();
        assert_eq!(found(&idx, "viejo "), vec![(1, 0)]);
        idx.upsert(&file(1, r"C:\B\nuevo.txt", "nuevo.txt", &[]))
            .unwrap();
        idx.commit().unwrap();
        assert!(found(&idx, "viejo ").is_empty());
        assert_eq!(found(&idx, "nuevo "), vec![(1, 0)]);
        let hit = idx.fetch(&[(1, 0)]).unwrap();
        assert_eq!(hit[0].path, r"C:\B\nuevo.txt");
    }

    #[test]
    fn fetch_returns_chunk_data_and_nothing_for_unknown_keys() {
        // `fetch` devuelve los datos del fragmento pedido; una clave inexistente, nada.
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&ballena()).unwrap();
        idx.commit().unwrap();
        let hits = idx.fetch(&[(1, 10)]).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, r"C:\Docs\informe.txt");
        assert_eq!(hits[0].name, "informe.txt");
        assert_eq!(hits[0].ext, "txt");
        assert_eq!(hits[0].body, "la ballena azul nada despacio");
        assert!(idx.fetch(&[(1, 99), (2, 10)]).unwrap().is_empty());
    }

    #[test]
    fn name_only_file_is_found_and_fetched_with_empty_text() {
        // Sin fragmentos: un documento con chunk_id 0 y texto vacío.
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&file(5, r"C:\Fotos\paisaje.jpg", "paisaje.jpg", &[]))
            .unwrap();
        idx.commit().unwrap();
        assert_eq!(found(&idx, "paisaje "), vec![(5, 0)]);
        let hits = idx.fetch(&[(5, 0)]).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].body, "");
    }

    #[test]
    fn open_keeps_committed_documents_after_reopening() {
        // Lo confirmado sigue ahí al volver a abrir la carpeta.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idx");
        {
            let mut idx = TextIndex::open(&path).unwrap();
            assert!(idx.needs_rebuild);
            idx.upsert(&ballena()).unwrap();
            idx.commit().unwrap();
        }
        let idx = TextIndex::open(&path).unwrap();
        assert!(!idx.needs_rebuild);
        assert_eq!(idx.num_docs(), 1);
        assert_eq!(found(&idx, "ballena "), vec![(1, 10)]);
    }

    #[test]
    fn clear_empties_the_index() {
        // `clear` borra todo (tras confirmar).
        let mut idx = TextIndex::in_memory().unwrap();
        idx.upsert(&ballena()).unwrap();
        idx.commit().unwrap();
        idx.clear().unwrap();
        idx.commit().unwrap();
        assert_eq!(idx.num_docs(), 0);
        assert!(found(&idx, "ballena ").is_empty());
    }
}
