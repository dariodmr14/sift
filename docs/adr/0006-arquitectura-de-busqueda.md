# ADR 0006 — Arquitectura de búsqueda (preparada para la búsqueda por significado)

- **Estado:** aceptada
- **Fecha:** 2026-11-10

## Contexto

La Fase 2 trae la búsqueda por palabras: dentro de los archivos, no solo por nombre, en < 50 ms. El objetivo final (Fase 4) es que «bases de datos» encuentre todo lo relacionado aunque no contenga esas palabras. Lo que se haga ahora no debe obligar a rehacer el esquema ni el flujo cuando llegue la parte semántica.

## Decisión

### 1. La unidad es el *fragmento*, no el archivo

Cada archivo se trocea en fragmentos (`chunks`) de ~1.200 caracteres, con 150 de solapamiento, cortando en párrafos o frases cuando se puede. Es la misma unidad que necesitará la Fase 4:

- `multilingual-e5-small` acepta hasta 512 tokens; ~1.200 caracteres de español son ~300–400 tokens, con margen.
- Un fragmento con su propio id estable en SQLite (`chunks.id`) es lo que tendrá un vector. El índice HNSW de la Fase 4 usará ese mismo id como clave.
- El solapamiento evita que una idea partida entre dos fragmentos no la encuentre ninguno.

### 2. SQLite es la fuente de verdad; Tantivy (y luego HNSW) se reconstruyen desde ahí

Migración v2: `roots` (carpetas), `ignore_patterns`, `files` (ruta, tamaño, fecha, hash del contenido, error) y `chunks` (texto y posición). También `opens` (qué abres y cuándo), que alimentará la frecuencia de uso de la Fase 5.

El índice de Tantivy guarda una copia del texto para generar los fragmentos resaltados sin consultar SQLite en cada búsqueda. Pero es derivado: si se borra o cambia su formato, se reconstruye desde `chunks`.

### 3. La búsqueda es un *pipeline* de recuperadores con fusión

```
consulta ─┬─► recuperador léxico (Tantivy)        ─┐
          └─► recuperador semántico (Fase 4, HNSW) ─┴─► fusión RRF ─► agrupar por archivo ─► resultados
```

- `Retriever` es un trait: recibe la consulta y devuelve fragmentos ordenados (`chunk_id`, puntuación).
- La fusión es **Reciprocal Rank Fusion**: combina por *posición* en cada lista, no por puntuación, así que no hace falta que BM25 y la similitud coseno estén en la misma escala. Ya funciona hoy con una sola lista.
- Añadir la Fase 4 = implementar otro `Retriever` y registrarlo. Ni el esquema, ni la fusión, ni la UI cambian.

### 4. Texto en español

Analizador propio de Tantivy: tokenizar → minúsculas → **quitar tildes** → **raíz en español** (`normalización` y `normalizar` → `normaliz`). La consulta pasa por el mismo analizador. Encima:

- **Prefijos** en la última palabra, para encontrar mientras escribes (`normali…`).
- **Erratas**: distancia de edición 1 (palabras de 5+ letras) o 2 (de 9+).
- El nombre del archivo pesa más que el contenido.

### 5. Una sola barra para todo

La barra busca a la vez en Espacios, comandos y archivos, en una lista con secciones. Espacios y comandos se filtran en la UI (son pocos y es instantáneo); los archivos los busca Rust. Cada respuesta lleva un número de secuencia y las respuestas atrasadas se descartan, así teclear rápido nunca deja resultados viejos.

## Alternativas descartadas

- **Indexar archivos enteros** (sin fragmentos): más simple ahora, pero en la Fase 4 un vector por archivo diluye el significado de un PDF de 80 páginas. Habría que rehacer el esquema.
- **Solo FTS5 de SQLite**: ver ADR 0001 (peor ranking y sin analizador en español).
- **Combinar puntuaciones con pesos**: BM25 y coseno no son comparables; RRF no necesita calibrarlos.

## Consecuencias

- Para la Fase 4 se añade una migración con el modelo de embeddings y la fecha de cálculo por fragmento, más el índice HNSW con clave `chunk_id`.
- En esta fase solo se extrae el contenido de texto, Markdown y código. PDF y Word se indexan por nombre hasta la Fase 3.
- Sin watcher hasta la Fase 3: el indexado se sincroniza al arrancar, al cambiar las carpetas y con «Reindexar». Es incremental (por fecha, tamaño y hash), así que repetirlo es barato.
