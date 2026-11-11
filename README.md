# Sift

Lanzador y buscador semántico de escritorio para Windows. Pulsas un atajo, escribes lo que buscas y encuentra tus archivos por su significado, no solo por sus palabras. Todo en local: ni el índice, ni los archivos, ni el modelo de IA salen del ordenador.

> 🚧 En desarrollo: Fases 1 (el lanzador), 1.5 (Espacios básicos), 2 (búsqueda por palabras) y 3 (más formatos e indexado en tiempo real) terminadas. Plan completo en [PLAN.md](PLAN.md).

## Qué hace ya

- `Alt+Espacio` (configurable) abre una barra flotante de vidrio (Acrylic), desde cualquier app.
- Se maneja solo con teclado: escribir, ↑ ↓, Enter, Esc. Se oculta también al hacer clic fuera.
- **Espacios**: escribe su nombre o un alias y con Enter se abre todo de golpe (carpetas, archivos, webs, apps y comandos). Se crean, editan, duplican, reordenan, exportan e importan desde Ajustes, y se guardan en SQLite.
- **Seguro por diseño**: los comandos se ejecutan con programa y argumentos separados, sin shell, y piden confirmación la primera vez (y otra vez si cambian).
- **Iconos**: el real de Windows para apps y archivos, una librería con buscador, emoji o imagen propia; o «solo texto».
- **Ajustes desde la barra**: `Ctrl+,`, el engranaje o escribiendo «ajustes».
- **Sistema de diseño «Vidrio»**: sigue el tema y el color de acento de Windows ([guía](docs/design-system.md)).
- Icono en la bandeja (Abrir, Ajustes, Salir) y opción de arrancar con Windows.
- **Búsqueda en tus archivos**: por nombre y por contenido (texto, Markdown, código, PDF, Word y PowerPoint; imágenes y vídeos por nombre), sin importar tildes, con prefijos y erratas. La barra busca a la vez en Espacios, comandos y archivos, con el término resaltado. Enter abre, `Ctrl+Enter` abre la carpeta y `Ctrl+Shift+C` copia la ruta.
- **En tiempo real y sin notarse**: guardar, renombrar, mover o borrar un archivo se ve en la búsqueda en segundos. El indexado va en segundo plano con prioridad baja, limita su CPU y se pausa si estás usando mucho el PC. Si Sift se cierra a mitad, sigue donde iba.
- **Ajustes → Carpetas**: eliges dónde busca Sift y ves su estado en palabras («Al día · 1.240 archivos», «Leyendo 32 de 120 archivos», «En pausa mientras usas el PC»). Solo indexa lo que sirve para buscar: se salta programas, compilados, cachés y dependencias.
- **Un único vidrio** en toda la app (barra, Ajustes y diálogos), con la opción «Desactivar vidrio».

### Métricas (Windows 10, build de release)

| Métrica | Objetivo | Fase 1 | Fase 1.5 (Espacios + animación de entrada) |
|---|---|---|---|
| Atajo → ventana pintada (`RegisterHotKey`) | < 100 ms | p50 34 ms · p95 42 ms | p50 37 ms · p95 45 ms |
| Atajo → ventana pintada (hook de teclado) | < 100 ms | p50 34 ms · p95 40 ms | p50 36 ms · p95 43 ms |

| Búsqueda (50.000 fragmentos, índice Tantivy) | < 50 ms | | Fase 2: p50 6,3 ms · p95 12,1 ms |

20–22 medidas por fila en el atajo. La búsqueda se mide con `cargo test -p sift-search --release -- --ignored`. Se mide desde que el hilo del atajo detecta la pulsación hasta que la UI ha pintado el primer fotograma. La latencia aparece en el log y en Ajustes → Estado.

## Stack
- **App**: Tauri 2 (Rust + WebView2), UI en React + TypeScript + Vite.
- **Almacenamiento**: SQLite (fuente de verdad) + Tantivy (texto) + usearch/HNSW (vectores).
- **IA local**: `multilingual-e5-small` en ONNX Runtime.

## Desarrollo

Requisitos: Rust (stable, MSVC), Node 22+, Visual Studio Build Tools con "Desarrollo de escritorio con C++", WebView2.

Una vez, para que Sift lea los PDF (baja `pdfium.dll` en una versión fijada y comprueba su hash; ver ADR 0008):

```bash
pwsh scripts/fetch-pdfium.ps1
```

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
- [ADR 0006](docs/adr/0006-arquitectura-de-busqueda.md): arquitectura de búsqueda pensada para la Fase 4 (fragmentos, recuperadores y RRF).
- [ADR 0007](docs/adr/0007-que-se-indexa.md): qué se indexa (solo formatos útiles) y qué carpetas se saltan.
- [ADR 0008](docs/adr/0008-distribucion-de-pdfium.md): PDFium para los PDF y cómo llega `pdfium.dll` (dentro del instalador, versión y hash fijados).

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

**Fase 2**
- Un stemmer arregla los plurales pero rompe las erratas: «redundansia» y «redundancia» dan raíces distintas. La solución fue indexar cada campo dos veces, uno con raíces y otro solo sin tildes, y buscar en los dos.
- Diseñar desde el principio para lo que viene (fragmentos como unidad, un trait `Retriever` y fusión RRF) hace que la Fase 4 sea añadir un recuperador, no rehacer la búsqueda.
- SQLite es la fuente de verdad y Tantivy se puede reconstruir desde ella: un índice roto o de otro formato no pierde nada.
- Para decir «Leyendo 32 de 120» hay que separar *buscar cambios* (solo tamaño y fecha, barato) de *leer*. Así, además, se lee primero lo último que has tocado.
- Acrylic va a tirones al mover una ventana en Windows 10, pero solo mientras se mueve: cambiar a blur durante el arrastre da el mismo vidrio en reposo sin pagar el tirón.
- Para el arrastre con animación, una librería probada (dnd-kit) gana a un hook propio: medir, animar a los vecinos y la accesibilidad ya vienen resueltos.

**Fase 3**
- Indexar *todo* era el error: de 86.509 archivos de una carpeta real de la universidad solo 11.888 servían para buscar. Una lista de formatos permitidos y reconocer las carpetas generadas por lo que las rodea (la `Library` de Unity junto a `Assets`) dejó el índice en un 14 % y la base de datos de 620 MB en una fracción.
- Un renombrado llega del sistema como «desaparece A, aparece B». Casarlos por el hash del contenido conserva el id del archivo (sus fragmentos y su historial) y evita leerlo otra vez.
- Para recuperarse de un cierre a mitad basta con una marca por archivo: SQLite se guarda archivo a archivo y el índice de búsqueda por lotes, así que lo que quede sin confirmar se reenvía desde SQLite al arrancar, sin tocar el disco.
- Que el indexado «no se note» tiene tres capas: el modo de segundo plano de Windows (que también baja la prioridad del disco), un descanso proporcional al trabajo y una pausa con histéresis cuando el sistema está ocupado.
- Los materiales de Windows no se pueden fundir entre sí, pero sí taparse: un velo del color del fondo que entra antes de cambiar y sale después esconde el salto.

## Licencia
[MIT](LICENSE)
