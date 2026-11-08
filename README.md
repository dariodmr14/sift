# Sift

Lanzador y buscador semántico de escritorio para Windows. Pulsas un atajo, escribes lo que buscas y encuentra tus archivos por su significado, no solo por sus palabras. Todo en local: ni el índice, ni los archivos, ni el modelo de IA salen del ordenador.

> 🚧 En desarrollo (Fase 0: cimientos). Plan completo en [PLAN.md](PLAN.md).

## Stack
- **App**: Tauri 2 (Rust + WebView2), UI en React + TypeScript + Vite.
- **Almacenamiento**: SQLite (fuente de verdad) + Tantivy (texto) + usearch/HNSW (vectores).
- **IA local**: `multilingual-e5-small` en ONNX Runtime.

## Desarrollo

Requisitos: Rust (stable, MSVC), Node 22+, Visual Studio Build Tools con "Desarrollo de escritorio con C++", WebView2.

```bash
cd apps/desktop
npm install
npm run tauri dev
```

Comprobaciones:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Licencia
[MIT](LICENSE)
