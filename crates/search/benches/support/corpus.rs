//! Corpus sintético y determinista para medir la búsqueda: siempre el mismo texto, los
//! mismos vectores y las mismas consultas, en cualquier PC (lo comparten el benchmark de
//! `criterion` y el test de latencia).

#![allow(dead_code)] // cada uno de los dos usa una parte

use sift_indexer::{IndexSink, IndexedFile};
use sift_search::{TextIndex, VectorIndex};

/// Vocabulario de apuntes de la carrera, para generar texto con aspecto realista.
pub const WORDS: &[&str] = &[
    "normalización",
    "tabla",
    "clave",
    "primaria",
    "foránea",
    "dependencia",
    "funcional",
    "transacción",
    "concurrencia",
    "bloqueo",
    "índice",
    "consulta",
    "relación",
    "atributo",
    "protocolo",
    "red",
    "paquete",
    "enrutamiento",
    "capa",
    "transporte",
    "algoritmo",
    "grafo",
    "árbol",
    "complejidad",
    "recursión",
    "memoria",
    "proceso",
    "hilo",
    "sistema",
    "operativo",
    "compilador",
    "gramática",
    "autómata",
    "lenguaje",
    "función",
    "variable",
    "objeto",
    "clase",
    "herencia",
    "interfaz",
    "prueba",
    "requisito",
    "diseño",
    "arquitectura",
    "servidor",
    "cliente",
    "seguridad",
    "cifrado",
    "certificado",
    "examen",
    "práctica",
    "entrega",
];

/// Generador pseudoaleatorio simple y determinista (sin dependencias).
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next_index(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as usize
    }

    /// Un vector de longitud 1 (como los de un modelo de embeddings).
    pub fn unit_vector(&mut self, dims: usize) -> Vec<f32> {
        let v: Vec<f32> = (0..dims)
            .map(|_| (self.next_index() % 2001) as f32 / 1000.0 - 1.0)
            .collect();
        let norm = v
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt()
            .max(f32::EPSILON);
        v.into_iter().map(|x| x / norm).collect()
    }
}

/// Palabras por fragmento (≈ 1.200 caracteres, como los de verdad).
const WORDS_PER_CHUNK: usize = 180;

/// Índice de palabras con `files × chunks_per_file` fragmentos. Devuelve también los
/// hashes de los textos, para darles vectores.
pub fn text_index(files: i64, chunks_per_file: i64) -> (TextIndex, Vec<Vec<i64>>) {
    let mut rng = Lcg(42);
    let mut idx = TextIndex::in_memory().expect("índice en memoria");
    let mut hashes = Vec::with_capacity(files as usize);
    let mut chunk_id = 0;
    for file_id in 1..=files {
        let chunks: Vec<(i64, String)> = (0..chunks_per_file)
            .map(|_| {
                chunk_id += 1;
                let text: Vec<&str> = (0..WORDS_PER_CHUNK)
                    .map(|_| WORDS[rng.next_index() % WORDS.len()])
                    .collect();
                (chunk_id, text.join(" "))
            })
            .collect();
        hashes.push(
            chunks
                .iter()
                .map(|(_, t)| sift_storage::text_hash(t))
                .collect(),
        );
        idx.upsert(&IndexedFile {
            file_id,
            path: format!(r"C:\Uni\apuntes\tema{file_id}.md"),
            name: format!("tema{file_id}.md"),
            ext: "md".into(),
            chunks,
        })
        .expect("añadir al índice");
    }
    idx.commit().expect("commit");
    (idx, hashes)
}

/// Índice de vectores con los primeros `per_file` fragmentos de cada archivo (como en la
/// app, donde no todo tiene vector: ver `EmbedPolicy`).
pub fn vector_index(hashes: &[Vec<i64>], per_file: usize, dims: usize) -> VectorIndex {
    let mut rng = Lcg(7);
    let vectors = VectorIndex::new(dims).expect("índice de vectores");
    let batch: Vec<(i64, Vec<f32>)> = hashes
        .iter()
        .flat_map(|file| file.iter().take(per_file))
        .map(|h| (*h, rng.unit_vector(dims)))
        .collect();
    vectors.add(&batch).expect("añadir vectores");
    vectors
}

/// Consultas variadas: palabras completas, prefijos (escribiendo), erratas y frases.
pub const QUERIES: &[&str] = &[
    "normalización ",
    "clave primaria ",
    "transacción concurrencia ",
    "normali",
    "protoc",
    "arbol ",
    "complejidad algoritmo ",
    "dependensia funcional ",
    "enrutamento ",
    "cifr",
    "cómo evitar redundancia en tablas",
    "conectar un programa con el servidor",
];
