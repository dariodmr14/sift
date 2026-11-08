# ADR 0001 — Tauri 2 para la app; SQLite + Tantivy + HNSW para el almacenamiento

- **Estado:** aceptada
- **Fecha:** 2026-11-08

## Contexto

Sift es un lanzador que vive siempre abierto en segundo plano y tiene que aparecer en < 100 ms, consumir < 100 MB en reposo y buscar en < 50 ms (p95) sobre ~50.000 fragmentos, por texto y por significado, todo en local (ver objetivos en [PLAN.md](../../PLAN.md)).

Hay que decidir dos cosas: con qué se construye la app de escritorio y dónde se guardan el estado y los índices.

## Decisión 1: Tauri 2 (Rust + WebView2)

**Elegido:** Tauri 2, con el núcleo en Rust y la UI en React + TypeScript.

| Opción | A favor | En contra |
|---|---|---|
| **Tauri 2** | Usa el WebView2 que ya trae Windows: binario de pocos MB, arranque rápido, poca RAM. Núcleo en Rust nativo, sin recolector de basura. | Ecosistema más pequeño que Electron. Hay que aprender Rust. |
| Electron | Ecosistema enorme, mucha documentación. | Empaqueta Chromium + Node: ~150 MB de RAM en reposo, que choca con el objetivo de < 100 MB. |
| Nativo (WinUI 3 / C#) | Integración perfecta con Windows (Mica/Acrylic, atajos). | UI más lenta de iterar; solo Windows; ecosistema de búsqueda/IA más pobre que el de Rust. |

Nota: existe Tauri 3 en alfa (noviembre de 2026). Nos quedamos en la rama estable 2.x hasta que la 3 sea estable.

## Decisión 2: SQLite como fuente de verdad + índices derivados

**Elegido:** separar *estado* e *índices*, como hacen los motores de búsqueda serios.

- **SQLite (WAL)** guarda la verdad: archivos, hashes, fragmentos, Espacios, historial, ajustes. Es transaccional y resiste cortes.
- **Tantivy** indexa el texto (BM25, prefijos, erratas, lematizador en español).
- **usearch (HNSW)** indexa los vectores de embeddings para búsqueda aproximada en milisegundos.

Tantivy y HNSW se pueden **borrar y reconstruir** desde SQLite. Eso simplifica la recuperación ante fallos: si un índice se corrompe o cambia su formato, se regenera.

| Opción | A favor | En contra |
|---|---|---|
| **SQLite + Tantivy + HNSW** | Cada pieza es la mejor en lo suyo. Ranking de texto bueno y búsqueda vectorial que escala. | Tres piezas que mantener sincronizadas. |
| Solo SQLite (FTS5 + sqlite-vec) | Un solo fichero, más simple. | FTS5: peor ranking y tokenización. sqlite-vec: búsqueda exacta por fuerza bruta, escala peor. |
| Base vectorial externa (Qdrant, etc.) | Muy potente. | Un servidor aparte: rompe "todo local, ligero y sin instalar nada más". |

**Plan B:** si Tantivy da problemas en la Fase 2, usar FTS5 de SQLite.

## Consecuencias

- La sincronización entre SQLite y los índices es responsabilidad del crate `indexer`: cada cambio se escribe primero en SQLite y luego se propaga.
- Hay que escribir en Rust la parte de búsqueda e IA (`ort`, `tantivy`, `usearch`), lo que exige aprender Rust pero da rendimiento predecible.
- Si un día se quiere soportar macOS/Linux, Tauri lo permite sin reescribir la UI.
