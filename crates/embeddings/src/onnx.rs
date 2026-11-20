//! [`Embedder`] con ONNX Runtime (`ort`) y el tokenizador de Hugging Face (`tokenizers`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::pooling::pool;
use crate::{EmbedError, Embedder, ModelCard, Purpose};

/// El entorno de ONNX Runtime: uno por proceso (lo exige ONNX Runtime), compartido por
/// todos los modelos. Sin telemetría: en Windows la envía por defecto (CLAUDE.md: nada).
fn environment() -> Result<ort::environment::Environment, EmbedError> {
    static ENV: OnceLock<Result<ort::environment::Environment, String>> = OnceLock::new();
    ENV.get_or_init(|| {
        ort::init()
            .with_name("sift")
            .with_telemetry(false)
            .build()
            .map_err(|e| e.to_string())
    })
    .clone()
    .map_err(EmbedError::Runtime)
}

fn runtime(e: impl std::fmt::Display) -> EmbedError {
    EmbedError::Runtime(e.to_string())
}

pub struct OnnxEmbedder {
    card: &'static ModelCard,
    /// El tokenizador (con el corte a `max_tokens` y el relleno del lote). Ocupa unos 250 MB
    /// en memoria (el de e5 tiene 250 000 piezas), así que **uno por modelo** para todo el
    /// proceso: las dos sesiones de Sift (indexado y consultas) comparten el mismo
    /// (auditoría 1.0: antes había cuatro copias, 1 GB).
    tokenizer: Arc<Tokenizer>,
    /// El mismo sin cortar, para contar tokens. Solo se crea si alguien cuenta (las pruebas
    /// y `eval/`; la app nunca).
    counter: OnceLock<Tokenizer>,
    /// ONNX Runtime pide `&mut` para ejecutar: un hilo cada vez.
    session: Mutex<Session>,
    /// El entorno de ONNX Runtime tiene que vivir lo que viva la sesión.
    _env: ort::environment::Environment,
    /// ¿El modelo pide `token_type_ids`? (los BERT sí; ModernBERT no)
    wants_token_types: bool,
}

/// Tokenizadores ya cargados, por archivo. `Weak`: cuando no queda ningún modelo que lo
/// use (p. ej. al desactivar la búsqueda inteligente), se libera.
type TokenizerCache = Mutex<HashMap<(PathBuf, usize), Weak<Tokenizer>>>;

fn shared_tokenizer(path: &Path, max_tokens: usize) -> Result<Arc<Tokenizer>, EmbedError> {
    static CACHE: OnceLock<TokenizerCache> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let key = (path.to_path_buf(), max_tokens);
    if let Some(t) = cache.get(&key).and_then(Weak::upgrade) {
        return Ok(t);
    }
    let mut tokenizer =
        Tokenizer::from_file(path).map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::BatchLongest,
        ..PaddingParams::default()
    }));
    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: max_tokens,
            ..TruncationParams::default()
        }))
        .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
    let tokenizer = Arc::new(tokenizer);
    cache.retain(|_, w| w.strong_count() > 0);
    cache.insert(key, Arc::downgrade(&tokenizer));
    Ok(tokenizer)
}

impl OnnxEmbedder {
    /// Carga el modelo de `dir` (los archivos de la ficha). `threads`: hilos de cálculo,
    /// que es como se limita la CPU que usa.
    pub fn load(card: &'static ModelCard, dir: &Path, threads: usize) -> Result<Self, EmbedError> {
        for file in card.files() {
            if !dir.join(file.name).is_file() {
                return Err(EmbedError::MissingFile(
                    dir.join(file.name).display().to_string(),
                ));
            }
        }
        let tokenizer = shared_tokenizer(&dir.join(card.tokenizer.name), card.max_tokens)?;

        let env = environment()?;
        let session = Session::builder(&env)
            .map_err(runtime)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(runtime)?
            .with_intra_threads(threads.max(1))
            .map_err(runtime)?
            // Sin el «arena» de ONNX Runtime: guarda para siempre la memoria del lote más
            // grande que haya visto (+570 MB con 8 fragmentos de 512 tokens) y Sift pasa casi
            // todo el tiempo en reposo (auditoría 1.0, medido con `examples/memoria.rs`). Cuesta
            // un ~15 % de velocidad al vectorizar (en segundo plano); las consultas, igual.
            // Desactivar también `memory_pattern` no ahorra nada más y es un 35 % más lento.
            .with_execution_providers([ort::ep::CPU::default().with_arena_allocator(false).build()])
            .map_err(runtime)?
            .commit_from_file(dir.join(card.model.name))
            .map_err(runtime)?;
        let wants_token_types = session
            .inputs()
            .iter()
            .any(|input| input.name() == "token_type_ids");
        Ok(OnnxEmbedder {
            card,
            tokenizer,
            counter: OnceLock::new(),
            session: Mutex::new(session),
            _env: env,
            wants_token_types,
        })
    }

    /// Cuántos tokens tiene un texto (sin cortar). Para medir cuántos pasan del tope.
    pub fn count_tokens(&self, text: &str) -> usize {
        let counter = self.counter.get_or_init(|| {
            let mut t = (*self.tokenizer).clone();
            t.with_padding(None);
            let _ = t.with_truncation(None);
            t
        });
        counter.encode(text, true).map_or(0, |e| e.get_ids().len())
    }
}
impl Embedder for OnnxEmbedder {
    fn card(&self) -> &'static ModelCard {
        self.card
    }

    fn embed(&self, texts: &[&str], purpose: Purpose) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let prefix = match purpose {
            Purpose::Query => self.card.query_prefix,
            Purpose::Document => self.card.document_prefix,
        };
        let inputs: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
        let encodings = self
            .tokenizer
            .encode_batch(inputs, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        let batch = encodings.len();
        let tokens = encodings.first().map_or(0, |e| e.get_ids().len());
        let flat = |f: &dyn Fn(&tokenizers::Encoding) -> &[u32]| -> Vec<i64> {
            encodings
                .iter()
                .flat_map(|e| f(e).iter().map(|&x| i64::from(x)))
                .collect()
        };
        let ids = flat(&|e| e.get_ids());
        let mask = flat(&|e| e.get_attention_mask());
        let shape = [batch as i64, tokens as i64];

        let mut session = self.session.lock().unwrap_or_else(|p| p.into_inner());
        let ids_t = Tensor::from_array((shape, ids)).map_err(runtime)?;
        let mask_t = Tensor::from_array((shape, mask.clone())).map_err(runtime)?;
        let outputs = if self.wants_token_types {
            let types = Tensor::from_array((shape, vec![0i64; batch * tokens])).map_err(runtime)?;
            session.run(ort::inputs![
                "input_ids" => ids_t,
                "attention_mask" => mask_t,
                "token_type_ids" => types
            ])
        } else {
            session.run(ort::inputs![
                "input_ids" => ids_t,
                "attention_mask" => mask_t
            ])
        }
        .map_err(runtime)?;

        let (out_shape, hidden) = outputs[0].try_extract_tensor::<f32>().map_err(runtime)?;
        let dims = self.card.dims;
        if out_shape.len() != 3 || out_shape[2] as usize != dims {
            return Err(EmbedError::Shape(format!(
                "se esperaba [{batch}, {tokens}, {dims}] y llegó {out_shape:?}"
            )));
        }
        Ok(pool(hidden, &mask, batch, tokens, dims, self.card.pooling))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MODELS, dot};

    /// Carpeta donde la app guarda los modelos. Si un modelo no está descargado, su
    /// prueba se salta (la CI no los descarga: pesan 100 MB cada uno).
    fn model_dir(card: &ModelCard) -> Option<std::path::PathBuf> {
        let dir = std::path::PathBuf::from(std::env::var_os("APPDATA")?)
            .join("dev.sift.app")
            .join("models")
            .join(card.id);
        dir.join(card.model.name).is_file().then_some(dir)
    }

    #[test]
    fn missing_files_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let e = OnnxEmbedder::load(MODELS[0], dir.path(), 1).err().unwrap();
        assert!(matches!(e, EmbedError::MissingFile(_)), "{e}");
    }

    /// Con los modelos de verdad: una frase está más cerca de otra que dice lo mismo con
    /// otras palabras que de una que habla de otra cosa.
    #[test]
    fn related_texts_are_closer_than_unrelated_ones() {
        for card in MODELS {
            let Some(dir) = model_dir(card) else {
                eprintln!("se salta {}: no está descargado", card.id);
                continue;
            };
            let embedder = OnnxEmbedder::load(card, &dir, 2).unwrap();
            let docs = embedder
                .embed(
                    &[
                        "La normalización elimina la redundancia dividiendo las tablas según sus dependencias funcionales.",
                        "El campo eléctrico creado por una carga puntual disminuye con el cuadrado de la distancia.",
                    ],
                    Purpose::Document,
                )
                .unwrap();
            let q = embedder
                .embed(
                    &["cómo evitar datos repetidos en una base de datos"],
                    Purpose::Query,
                )
                .unwrap();
            assert_eq!(docs[0].len(), card.dims);
            let (related, unrelated) = (dot(&q[0], &docs[0]), dot(&q[0], &docs[1]));
            eprintln!(
                "{}: relacionado {related:.3}, no relacionado {unrelated:.3}",
                card.id
            );
            // e5 da similitudes en una franja estrecha (0,7–0,9): basta con que ordene bien.
            assert!(
                related > unrelated + 0.02,
                "{}: {related} vs {unrelated}",
                card.id
            );
        }
    }
}
