# Sift

Lanzador y buscador semántico de escritorio para Windows. Pulsas un atajo, escribes lo que buscas y encuentra tus archivos por su significado, no solo por sus palabras. Todo en local: ni el índice, ni los archivos, ni el modelo de IA salen del ordenador.

> 🚧 En desarrollo: Fases 1 (el lanzador), 1.5 (Espacios básicos), 2 (búsqueda por palabras), 3 (más formatos e indexado en tiempo real), 4 (búsqueda por significado), 5 (ranking híbrido y medición), 6 (Espacios completos) y 6.5 (colocar ventanas) terminadas. Plan completo en [PLAN.md](PLAN.md).

## Qué hace ya

- `Alt+Espacio` (configurable) abre una barra flotante de vidrio (Acrylic), desde cualquier app.
- Se maneja solo con teclado: escribir, ↑ ↓, Enter, Esc. Se oculta también al hacer clic fuera.
- **Espacios**: escribe su nombre o una de sus palabras clave y con Enter se abre todo de golpe (carpetas, archivos, webs, apps y comandos). Se crean, editan, duplican, reordenan, exportan e importan desde Ajustes, y se guardan en SQLite.
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
- **Barra vacía útil**: sin escribir nada, tus Espacios y lo último que abriste desde Sift.
- **Filtros rápidos**: «pdf contrato», «código árbol», «imagen playa»… buscan solo ese tipo de archivo (la palabra queda como etiqueta; Retroceso la quita).
- **Vista previa**: sin salir de la barra (Tab), por qué sale cada resultado y un fragmento más largo, con lo que encaja subrayado.
- **PDF en su página**: un PDF se abre en la página que encaja con tu búsqueda («pág. 37»), con el lector que tengas (Edge, Chrome, Adobe, Sumatra…).
- **Espacios y archivos en la misma lista**: si escribes el nombre o una palabra clave de un Espacio, sale el primero; si encaja a medias, se coloca entre los archivos según lo que más uses.
- **Colocar ventanas**: cada cosa que abre un Espacio puede ir a una zona de la pantalla (mitades, tercios, cuartos o toda la pantalla), en la pantalla que elijas; si ya estaba abierta, se mueve en vez de abrir otra.
- **Esperas en los Espacios**: «Esperar a que responda una web de tu equipo», «…un servicio acepte conexiones (como una base de datos)», «…termine el comando anterior» o unos segundos, con «Si no ocurre en X segundos: no seguir y avisarme / seguir igualmente». La barra dice por qué paso va, Esc cancela (lo ya abierto sigue abierto) y al final cuenta qué se ejecutó y qué no.
- **Rutas sugeridas** en el editor de Espacios: escribe el nombre de un archivo o carpeta y Sift te la sugiere desde su índice (o elígela con el diálogo de Windows).

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

## Instalar para usarlo a diario

Desde `main`: `powershell -ExecutionPolicy Bypass -File scripts/instalar.ps1` (compila y abre el instalador; para actualizar, lo mismo). Detalles, y por qué la versión de desarrollo no toca tus datos, en [docs/instalar.md](docs/instalar.md).

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

La versión de desarrollo usa sus propios datos (`%APPDATA%\dev.sift.app-dev`), con datos de prueba, y el atajo Ctrl+Alt+Espacio: nunca toca la instalada (ver [docs/instalar.md](docs/instalar.md)).

Variables útiles: `RUST_LOG=debug` (más logs) y `SIFT_FORCE_KEYBOARD_HOOK=1` (probar la vía alternativa del atajo). Logs: `%LOCALAPPDATA%\dev.sift.app\logs`.

Comprobaciones (la CI, en local: lo mismo que el workflow de GitHub, que solo se lanza a mano; para al primer error y acaba con un resumen):

```bash
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1
```

Prueba de extremo a extremo (arranca la versión de desarrollo con sus datos de prueba y **abre ventanas**; comprueba la barra, la búsqueda y Ajustes, y deja capturas en `target/e2e/`):

```bash
node scripts/e2e.mjs
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
- [ADR 0011](docs/adr/0011-espacios-en-los-resultados.md): Espacios mezclados con los archivos (exactos primero, parciales en la escala de los archivos) y rutas sugeridas del índice.
- [ADR 0012](docs/adr/0012-esperas-en-los-espacios.md): esperas en los Espacios (solo cosas de tu equipo, el comando esperado sin ventana, cancelar sin cerrar nada).
- [ADR 0013](docs/adr/0013-pdf-en-su-pagina-y-filtros.md): PDF en su página, filtros rápidos por tipo y barra vacía con lo reciente.
- [ADR 0014](docs/adr/0014-colocar-ventanas.md): colocar ventanas en zonas (encontrar la ventana nueva, bordes invisibles, varias pantallas).
- [ADR 0015](docs/adr/0015-bienvenida-y-permiso-de-descarga.md): bienvenida la primera vez y permiso antes de descargar el modelo.
- [ADR 0016](docs/adr/0016-campos-y-movimiento-al-escribir.md): borde inferior de los campos a 3:1 y sin animación al escribir.

## Créditos

- El pulido visual de la Fase 7 se hizo con [unslop](https://github.com/heydamir/unslop), una guía para quitarle a una interfaz el aspecto de hecha con IA. unslop was created by Damir Sokolovsky ([@heydamir](https://x.com/heydamir)), who started the Unslopping series on X. It is licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), so keep this credit in any copy or adaptation. La auditoría y la dirección que salieron de ella están en [docs/unslop-direccion.md](docs/unslop-direccion.md).

## Licencia
**Todos los derechos reservados.** El código es público para que se pueda ver y estudiar, pero no es de código abierto: sin permiso no se puede copiar, modificar ni redistribuir. Sí puedes instalar las versiones publicadas y usarlas. Los componentes de terceros (dependencias, PDFium, el modelo de embeddings) conservan sus propias licencias. Detalles en [LICENSE](LICENSE).

## Cómo se ha construido

Sift lo he ideado, diseñado y dirigido yo; la mayor parte del código la ha escrito Claude Code (el asistente de programación de Anthropic) siguiendo mi plan. El reparto ha sido este:

- **Lo que he hecho yo:** la idea y para qué sirve la app; el plan por fases con lo que tenía que cumplir cada una; las prioridades y las decisiones de producto y de diseño, eligiendo entre las opciones que salían; revisar y entender cada cambio antes de aceptarlo; y usar Sift a diario para encontrar lo que fallaba o sobraba ([FEEDBACK.md](FEEDBACK.md)).
- **En lo que me ha apoyado Claude:** escribir la mayor parte del código y de los tests a partir de ese plan, proponer alternativas con sus pros y contras, medir (benchmarks y la evaluación de `eval/`) y dejar por escrito las decisiones en los [ADR](docs/adr/).

Cada cambio ha ido en su rama y su PR, con la CI local en verde y con mi visto bueno antes de fusionarse.

Por eso Claude aparece en GitHub entre quienes han contribuido al repositorio: parte del código, los primeros pull requests y la puesta en marcha de GitHub (ramas, PR y CI) los hice con Claude Code, que firmaba los commits como coautor. GitHub conserva esa firma en los PR ya fusionados y no se puede quitar.

### Lo aprendido

Lo más útil de cada fase está en [docs/aprendido.md](docs/aprendido.md). En resumen:

- Medir antes de decidir: el modelo, los pesos de la búsqueda y lo que la hacía lenta salieron de medir, y casi nunca era lo que parecía.
- La seguridad la impone Rust, no la interfaz: lo que ejecuta un Espacio se aprueba en Rust y con el comando exacto.
- Windows tiene sus propias reglas (el foco, los bordes invisibles, los materiales, las apps que reutilizan su ventana), y conviene descubrirlas con pruebas pequeñas y aisladas.
- Un sistema de diseño solo se mantiene si un test lo vigila.
