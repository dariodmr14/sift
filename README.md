# Sift

Lanzador y buscador semántico de escritorio para Windows. Pulsas un atajo, escribes lo que buscas y encuentra tus archivos por su significado, no solo por sus palabras. Todo en local: ni el índice, ni los archivos, ni el modelo de IA salen del ordenador.

> 🚧 En desarrollo: Fase 1 (el lanzador) terminada. Plan completo en [PLAN.md](PLAN.md).

## Qué hace ya

- `Alt+Espacio` (configurable) abre una barra flotante con fondo Acrylic, desde cualquier app.
- Se maneja solo con teclado: escribir, ↑ ↓, Enter, Esc. Se oculta también al hacer clic fuera.
- Icono en la bandeja (Abrir, Ajustes, Salir) y opción de arrancar con Windows.
- Los resultados todavía son de prueba: la búsqueda real llega en la Fase 2.

### Métricas (Windows 10, build de release, 20 medidas)

| Métrica | Objetivo | Medido |
|---|---|---|
| Atajo → ventana pintada (`RegisterHotKey`) | < 100 ms | p50 34 ms · p95 42 ms |
| Atajo → ventana pintada (hook de teclado) | < 100 ms | p50 34 ms · p95 40 ms |

Se mide desde que el hilo del atajo detecta la pulsación hasta que la UI ha pintado el primer fotograma. La latencia aparece en el log y en Ajustes → Estado.

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

Variables útiles: `RUST_LOG=debug` (más logs) y `SIFT_FORCE_KEYBOARD_HOOK=1` (probar la vía alternativa del atajo). Logs: `%LOCALAPPDATA%\dev.sift.app\logs`.

Comprobaciones:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Decisiones de arquitectura

- [ADR 0001](docs/adr/0001-tauri-y-almacenamiento.md): Tauri 2; SQLite + Tantivy + HNSW.
- [ADR 0002](docs/adr/0002-atajo-global.md): atajo global con `RegisterHotKey` y hook de teclado como alternativa.
- [ADR 0003](docs/adr/0003-ipc-tipado-tauri-specta.md): IPC tipado con tauri-specta.

## Lo aprendido

**Fase 1**
- Mantener la ventana creada y oculta es lo que hace posible el < 100 ms. Una vez hecho eso, el tiempo restante (~33 ms) es esperar a que Windows componga unos dos fotogramas, y el código propio apenas cuenta: debug y release miden casi igual.
- Un hook de teclado de bajo nivel puede no recibir **nada** sin dar ningún error. La causa era que tao (la capa de ventanas de Tauri) registra el teclado como Raw Input. Se encontró aislando variables: el mismo código en un ejecutable mínimo funcionaba; con Tauri, el contador de llamadas marcaba 0.
- Windows protege el foco: no cualquier proceso puede traer su ventana al frente. Por eso funciona `AttachThreadInput`.
- Generar los tipos de TypeScript desde Rust (tauri-specta), con un test que falla si se desincronizan, convierte los errores de IPC en errores de compilación.

## Licencia
[MIT](LICENSE)
