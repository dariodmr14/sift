# Sift

Lanzador y buscador semántico de escritorio para Windows. Pulsas un atajo, escribes lo que buscas y encuentra tus archivos por su significado, no solo por sus palabras. Todo en local: ni el índice, ni los archivos, ni el modelo de IA salen del ordenador.

> 🚧 En desarrollo: Fases 1 (el lanzador), 1.5 (Espacios básicos), 2 (búsqueda por palabras), 3 (más formatos e indexado en tiempo real), 4 (búsqueda por significado) y 5 (ranking híbrido y medición) terminadas. Plan completo en [PLAN.md](PLAN.md).

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
- **Búsqueda inteligente (por significado)**: «bases de datos» trae también lo relacionado aunque no use esas palabras. Combina la búsqueda por palabras con un modelo local (`multilingual-e5-small`, en ONNX Runtime) que se descarga una vez (~135 MB, con su hash comprobado) y prepara tus archivos en segundo plano. Los resultados que llegan solo por significado se marcan «Por significado». En las búsquedas de un nombre o un código («shell-v4.3») no interviene. Se puede desactivar en Ajustes → General.
- **Lo que usas, primero**: lo que abres a menudo y hace poco sube en los resultados (frecencia), sin colar nada que no encaje con lo que buscas.

### Métricas (Windows 10, build de release)

| Métrica | Objetivo | Fase 1 | Fase 1.5 (Espacios + animación de entrada) |
|---|---|---|---|
| Atajo → ventana pintada (`RegisterHotKey`) | < 100 ms | p50 34 ms · p95 42 ms | p50 37 ms · p95 45 ms |
| Atajo → ventana pintada (hook de teclado) | < 100 ms | p50 34 ms · p95 40 ms | p50 36 ms · p95 43 ms |

| Búsqueda (50.000 fragmentos, índice Tantivy) | < 50 ms | | Fase 2: p50 6,3 ms · p95 12,1 ms |

#### Calidad y velocidad de la búsqueda (Fases 4 y 5)

Con 34 consultas reales sobre una carpeta de la carrera (12.871 archivos, 29.477 textos con vector), definidas en [`eval/`](eval/README.md). **recall@10** (de lo esperado, cuánto sale entre los 10 primeros) y, entre paréntesis, **MRR** (lo arriba que sale lo primero que vale):

| Búsqueda | Idea sin sus palabras (26) | Tema (3) | Nombre o código (5) | Total |
|---|---|---|---|---|
| Solo palabras (Fase 3) | 0,12 (0,12) | 0,75 (0,57) | 1,00 (1,00) | 0,30 (0,29) |
| Híbrida de la Fase 4 | 0,56 (0,47) | 0,83 (0,61) | 1,00 (1,00) | 0,65 (0,56) |
| Solo palabras (Fase 5) | 0,53 (0,44) | 0,83 (0,58) | 1,00 (1,00) | 0,62 (0,53) |
| Solo significado (Fase 5) | 0,62 (0,50) | 0,58 (0,28) | 0,40 (0,40) | 0,58 (0,46) |
| **Híbrida (la de Sift, Fase 5)** | **0,69 (0,58)** | **0,83 (0,75)** | **1,00 (1,00)** | **0,75 (0,66)** |

La híbrida gana a cada búsqueda por separado. Lo que más subió en la Fase 5 fue que las palabras dejaran de exigir *todas* las palabras de una frase (ADR 0010). «cómo evitar redundancia en tablas» trae ya los apuntes de normalización (2.º).

| Latencia de la búsqueda completa (34 consultas × 5) | p50 | p95 |
|---|---|---|
| Solo palabras | 7,6 ms | 15,8 ms |
| Híbrida de la Fase 4 | 25,2 ms | 38,3 ms |
| **Híbrida (Fase 5)**, incluido calcular el vector de la consulta | **10,3 ms** | **17,5 ms** (objetivo < 50 ms) |

Medido en un i7-7700K con el PC en reposo (sin Sift abierto ni nada compilando), dos veces con el mismo resultado. Con la p95 tan por debajo de 50 ms, no hizo falta el plan B de la Fase 4 (enseñar primero los resultados de palabras y añadir los de significado después). Se repite con `cargo test -p sift-search --release --test eval -- --ignored recall_at_10 --nocapture` (necesita la base de datos y el modelo).

**Benchmark reproducible** (`cargo bench -p sift-search --bench search`): corpus sintético determinista del mismo tamaño (100.000 fragmentos, 30.000 con vector), medido con `criterion`. Con el PC en reposo:

| Pieza | Tiempo (mediana) |
|---|---|
| Palabras (una palabra · con errata · frase) | 1,9 ms · 4,0 ms · 0,2 ms |
| 100 vecinos en HNSW | 2,3 ms |
| Consulta → vector (e5-small) | 4,0 ms |
| Cargar y resaltar los resultados | 0,3 ms + 4,7 ms |
| **Búsqueda híbrida completa** | **p50 10,6 ms · p95 13,2 ms** |

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

Comprobaciones (la CI, en local: lo mismo que el workflow de GitHub, que solo se lanza a mano; para al primer error y acaba con un resumen):

```bash
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1
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
- [ADR 0009](docs/adr/0009-busqueda-por-significado.md): búsqueda por significado (modelo elegido midiendo, vectores en SQLite, HNSW derivado, fusión y evaluación).
- [ADR 0010](docs/adr/0010-ranking-hibrido.md): ranking híbrido (diagnóstico, búsqueda de palabras relajada, suma por archivo, frecencia, benchmark).

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

**Fase 4**
- Elegir el modelo midiendo: en las tablas públicas Granite R2 (mayo de 2026) superaba a e5-small por 9 puntos; con mis consultas en español, e5-small salió mejor en todo y, además, el doble de rápido.
- Una búsqueda de palabras que exige todas las palabras no encuentra nada con una frase («cómo evitar… en tablas»): por eso la semántica es lo que salva las ideas. Pero también por eso la semántica sola no basta para nombres y códigos (0,20): la fusión decide por tipo de búsqueda.
- Los pesos y umbrales «a ojo» del diseño estaban mal: el umbral de parecido no ayudaba (la protección real es el tipo de búsqueda) y un tercer recuperador que parecía obvio (BM25 con alguna de las palabras) no mejoraba nada. `eval/` lo dijo en minutos.
- Lo caro de una búsqueda no era el modelo (4 ms) sino cargar cien documentos de Tantivy para enseñar diez: leer solo ids de las columnas rápidas y cargar el texto al final bajó la p95 de 70 a 37 ms.
- Un vector por **texto** (hash) y no por fragmento: el 40 % de los fragmentos estaban repetidos, y reindexar un archivo no tira sus vectores.
- usearch (C++) abre las rutas con la codificación ANSI de Windows: con `C:\Users\Darío` no guardaba el índice. Los tests no lo vieron porque la carpeta temporal se llama `DARO~1`; ahora hay uno con tildes.

**Fase 5**
- Diagnosticar con datos y no con intuición: la sospecha era el modelo, y la causa grande resultó otra. La búsqueda de palabras devolvía *cero* resultados con frases como «cómo evitar redundancia en tablas» (ningún fragmento tiene todas las palabras), así que la fusión solo veía la semántica. Admitir que falte una palabra subió el total de 0,65 a 0,75.
- Lo que en la Fase 4 se descartó («alguna de las palabras») y lo que ahora funcionó se parecen, pero no son lo mismo: un recuperador aparte de BM25 puro mete ruido; la misma búsqueda exigiendo casi todas las palabras, detrás de las completas, no.
- Un índice aproximado (HNSW) se equivoca más de lo que parece: con el esfuerzo por defecto perdía el 12 % de los vecinos de verdad. Medirlo contra la búsqueda exacta costó minutos.
- Un benchmark por piezas encuentra lo que no esperas: lo más lento de una búsqueda no era el modelo ni el índice, sino **resaltar** el texto (se creaba un analizador por palabra). Arreglarlo bajó la híbrida a la mitad.
- Cuando no hay datos reales (4 aperturas), se puede calibrar simulando, siempre que el caso que ayuda y el que molesta se midan a la vez.
- Una métrica que baila ±0,02 entre ejecuciones no sirve para decidir: el baile venía de empates resueltos por el orden interno del índice.

## Licencia
[MIT](LICENSE)
