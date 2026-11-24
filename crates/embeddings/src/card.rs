//! Fichas de los modelos de embeddings que conoce Sift.
//!
//! Todo lo que hace falta para usar un modelo está aquí: de dónde se baja cada archivo y
//! su SHA-256 (se comprueba antes de usarlo), cuántas dimensiones da, cómo se resume un
//! texto en un vector y qué prefijos espera. Los dos candidatos del ADR 0009 dan vectores
//! de 384 dimensiones, así que el resto de Sift no cambia según cuál se use.

/// Cómo se resume en un vector la salida del modelo (un vector por token).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pooling {
    /// El vector del primer token (`[CLS]`).
    Cls,
    /// La media de los vectores de todos los tokens (sin contar el relleno).
    Mean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile {
    /// Nombre del archivo en la carpeta del modelo.
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
}

#[derive(Debug, PartialEq)]
pub struct ModelCard {
    /// Identificador estable: es lo que se guarda en la base de datos junto a cada vector.
    pub id: &'static str,
    pub name: &'static str,
    pub dims: usize,
    pub pooling: Pooling,
    /// Tokens como mucho por texto (lo demás se corta).
    pub max_tokens: usize,
    pub query_prefix: &'static str,
    pub document_prefix: &'static str,
    /// Parecido mínimo para que un resultado entre por significado. Depende del modelo
    /// (e5 da similitudes en una franja estrecha y alta); se calibra con `eval/`.
    pub min_similarity: f32,
    pub model: ModelFile,
    pub tokenizer: ModelFile,
}

impl ModelCard {
    pub fn files(&self) -> [&ModelFile; 2] {
        [&self.model, &self.tokenizer]
    }

    /// Lo que ocupa la descarga, en bytes.
    pub fn download_bytes(&self) -> u64 {
        self.model.bytes + self.tokenizer.bytes
    }
}

/// IBM, mayo de 2026. Apache 2.0. ONNX cuantizado a 8 bits para CPU con AVX2.
pub const GRANITE_97M_R2: ModelCard = ModelCard {
    id: "granite-97m-r2",
    name: "Granite Embedding 97M Multilingual R2",
    dims: 384,
    pooling: Pooling::Cls,
    // Admite 32.768, pero los fragmentos de Sift caben de sobra en 512 y más es más lento.
    max_tokens: 512,
    query_prefix: "",
    document_prefix: "",
    min_similarity: 0.0,
    model: ModelFile {
        name: "model.onnx",
        url: "https://huggingface.co/ibm-granite/granite-embedding-97m-multilingual-r2/resolve/main/onnx/model_quint8_avx2.onnx",
        sha256: "a6022dd8220ea6f6595562a1328ee216f4a94faa55362f2f4747c80f1e78772e",
        bytes: 98_247_878,
    },
    tokenizer: ModelFile {
        name: "tokenizer.json",
        url: "https://huggingface.co/ibm-granite/granite-embedding-97m-multilingual-r2/resolve/main/tokenizer.json",
        sha256: "4f2842d568e2724370aec203652a42ac783c7937f8347a1a2cc7506d71f1582f",
        bytes: 25_301_672,
    },
};

/// Microsoft, 2022. MIT. ONNX cuantizado a 8 bits (versión genérica: la oficial necesita
/// AVX-512, que no tienen muchos procesadores de escritorio).
pub const E5_SMALL: ModelCard = ModelCard {
    id: "e5-small",
    name: "multilingual-e5-small",
    dims: 384,
    pooling: Pooling::Mean,
    max_tokens: 512,
    query_prefix: "query: ",
    document_prefix: "passage: ",
    // Calibrado con eval/ (2026-11-13): ningún umbral mejoraba la híbrida; el ruido en
    // búsquedas exactas lo evita el tipo de búsqueda (`Intent`), no el umbral.
    min_similarity: 0.0,
    model: ModelFile {
        name: "model.onnx",
        url: "https://huggingface.co/Xenova/multilingual-e5-small/resolve/main/onnx/model_quantized.onnx",
        sha256: "f80102d3f2a1229f387d3c81909990d8945513e347b0eab049f7de3c6f98c193",
        bytes: 118_308_185,
    },
    tokenizer: ModelFile {
        name: "tokenizer.json",
        url: "https://huggingface.co/intfloat/multilingual-e5-small/resolve/main/tokenizer.json",
        sha256: "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39",
        bytes: 17_082_730,
    },
};

/// Los modelos que conoce Sift. El primero es el que se usa por defecto: e5-small, elegido
/// por calidad en `eval/` (recall@10 híbrido 0,69 frente a 0,65 de Granite) y además más
/// rápido (ADR 0009).
pub const MODELS: [&ModelCard; 2] = [&E5_SMALL, &GRANITE_97M_R2];

/// La ficha de un modelo por su id.
pub fn model(id: &str) -> Option<&'static ModelCard> {
    MODELS.into_iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_card_is_complete_and_ids_are_unique() {
        for card in MODELS {
            assert_eq!(model(card.id), Some(card));
            for file in card.files() {
                assert!(
                    file.url.starts_with("https://huggingface.co/"),
                    "{}",
                    file.url
                );
                assert_eq!(file.sha256.len(), 64, "{}", file.name);
                assert!(file.bytes > 0);
            }
            assert!(card.dims > 0 && card.max_tokens > 0);
        }
        assert_eq!(model("no-existe"), None);
    }

    /// La cifra que se enseña antes de que Rust la diga (`FALLBACK_MB` en
    /// `apps/desktop/src/settings/useWelcome.ts`) y la de los README tienen que ser esta.
    #[test]
    fn the_default_model_is_about_135_mb() {
        assert_eq!(MODELS[0].download_bytes() / 1_000_000, 135);
    }
}
