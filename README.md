# Sift

Lanzador y buscador semántico de escritorio para Windows. Pulsas un atajo, escribes lo que buscas y encuentra tus archivos por su significado, no solo por sus palabras. Todo en local: ni el índice, ni los archivos, ni el modelo de IA salen del ordenador.

> 🚧 En desarrollo: Fases 1 (el lanzador) y 1.5 (Espacios básicos) terminadas. Plan completo en [PLAN.md](PLAN.md).

## Qué hace ya

- `Alt+Espacio` (configurable) abre una barra flotante con fondo Acrylic, desde cualquier app.
- Se maneja solo con teclado: escribir, ↑ ↓, Enter, Esc. Se oculta también al hacer clic fuera.
- **Espacios**: escribe su nombre o un alias y con Enter se abre todo de golpe (carpetas, archivos, webs, apps y comandos). Se crean, editan, duplican, reordenan, exportan e importan desde Ajustes, y se guardan en SQLite.
- **Seguro por diseño**: los comandos se ejecutan con programa y argumentos separados, sin shell, y piden confirmación la primera vez (y otra vez si cambian).
- **Iconos**: el real de Windows para apps y archivos, una librería con buscador, emoji o imagen propia; o «solo texto».
- **Ajustes desde la barra**: `Ctrl+,`, el engranaje o escribiendo «ajustes».
- **Sistema de diseño «Vidrio»**: sigue el tema y el color de acento de Windows ([guía](docs/design-system.md)).
- Icono en la bandeja (Abrir, Ajustes, Salir) y opción de arrancar con Windows.
- La búsqueda en archivos llega en la Fase 2.

### Métricas (Windows 10, build de release)

| Métrica | Objetivo | Fase 1 | Fase 1.5 (Espacios + animación de entrada) |
|---|---|---|---|
| Atajo → ventana pintada (`RegisterHotKey`) | < 100 ms | p50 34 ms · p95 42 ms | p50 37 ms · p95 45 ms |
| Atajo → ventana pintada (hook de teclado) | < 100 ms | p50 34 ms · p95 40 ms | p50 36 ms · p95 43 ms |

20–22 medidas por fila. Se mide desde que el hilo del atajo detecta la pulsación hasta que la UI ha pintado el primer fotograma. La latencia aparece en el log y en Ajustes → Estado.

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
- [ADR 0004](docs/adr/0004-espacios-en-sqlite.md): Espacios en SQLite, migraciones y aprobación de comandos.
- [ADR 0005](docs/adr/0005-direccion-visual-vidrio.md): dirección visual «Vidrio».

## Lo aprendido

**Fase 1**
- Mantener la ventana creada y oculta es lo que hace posible el < 100 ms. Una vez hecho eso, el tiempo restante (~33 ms) es esperar a que Windows componga unos dos fotogramas, y el código propio apenas cuenta: debug y release miden casi igual.
- Un hook de teclado de bajo nivel puede no recibir **nada** sin dar ningún error. La causa era que tao (la capa de ventanas de Tauri) registra el teclado como Raw Input. Se encontró aislando variables: el mismo código en un ejecutable mínimo funcionaba; con Tauri, el contador de llamadas marcaba 0.
- Windows protege el foco: no cualquier proceso puede traer su ventana al frente. Por eso funciona `AttachThreadInput`.
- Generar los tipos de TypeScript desde Rust (tauri-specta), con un test que falla si se desincronizan, convierte los errores de IPC en errores de compilación.

**Fase 1.5**
- La seguridad de los comandos tiene que imponerla Rust, no la UI. `run_space` contesta «hace falta confirmar» sin lanzar nada, y la aprobación va ligada al comando exacto: si cambia un argumento, es otro comando.
- Separar la lógica de *cómo* se lanza en Windows (el trait `Opener`) permite probar con un lanzador falso que un argumento como `a & del C:\*` llega entero, como uno solo.
- En Windows, crear una ventana WebView2 desde un comando IPC síncrono se bloquea y la ventana se queda en blanco: tiene que ser `async`.
- Un sistema de diseño solo se mantiene si algo lo vigila: un test que falla con colores literales vale más que una guía.
- Probar sin poder hacer clic: la depuración remota de WebView2 permite manejar la app real por el protocolo de DevTools (teclas, IPC y capturas).

## Licencia
[MIT](LICENSE)
