# Desarrollo

Cómo compilar Sift, comprobarlo y las mediciones de cada fase. Para instalar la versión de uso diario, ver [instalar.md](instalar.md).

Requisitos: Rust (stable, MSVC), Node 22+, Visual Studio Build Tools con "Desarrollo de escritorio con C++", WebView2.

Una vez, para que Sift lea los PDF (baja `pdfium.dll` en una versión fijada y comprueba su hash; ver ADR 0008):

```bash
pwsh scripts/fetch-pdfium.ps1
```

Y, también una vez, el runtime de Visual C++ que va dentro del instalador (lo copia de tu Visual Studio, sin descargar nada; ver ADR 0020):

```bash
pwsh scripts/vc-runtime.ps1
```

```bash
cd apps/desktop
npm install
npm run tauri dev
```

La versión de desarrollo usa sus propios datos (`%APPDATA%\dev.sift.app-dev`), con datos de prueba, y el atajo Ctrl+Alt+Espacio: nunca toca la instalada (ver [instalar.md](instalar.md)).

Variables útiles: `RUST_LOG=debug` (más logs) y `SIFT_FORCE_KEYBOARD_HOOK=1` (probar la vía alternativa del atajo). Logs: `%LOCALAPPDATA%\dev.sift.app\logs`.

Comprobaciones (la CI, en local: lo mismo que el workflow de GitHub, que solo se lanza a mano; para al primer error y acaba con un resumen):

```bash
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1
```

Prueba de extremo a extremo (arranca la versión de desarrollo con sus datos de prueba y **abre ventanas**; comprueba la barra, la búsqueda y Ajustes, y deja capturas en `target/e2e/`):

```bash
node scripts/e2e.mjs
```

Otras herramientas:

- **Datos de demostración** (inventados, para capturas y vídeos): `SIFT_DEV_DEMO=1` antes de `npm run tauri dev`. Están en `demo/`.
- **Pruebas de aguante** (carga con 100 000 archivos, robustez y larga duración): `scripts/aguante/`. Los resultados, en la [auditoría de la 1.0](auditoria-1.0.md).
- **Evaluación de la búsqueda:** [`eval/`](../eval/README.md).

## Decisiones de arquitectura

- [ADR 0001](adr/0001-tauri-y-almacenamiento.md): Tauri 2; SQLite + Tantivy + HNSW.
- [ADR 0002](adr/0002-atajo-global.md): atajo global con `RegisterHotKey` y hook de teclado como alternativa.
- [ADR 0003](adr/0003-ipc-tipado-tauri-specta.md): IPC tipado con tauri-specta.
- [ADR 0004](adr/0004-espacios-en-sqlite.md): Espacios en SQLite, migraciones y aprobación de comandos.
- [ADR 0005](adr/0005-direccion-visual-vidrio.md): dirección visual «Vidrio».
- [ADR 0006](adr/0006-arquitectura-de-busqueda.md): arquitectura de búsqueda pensada para la Fase 4 (fragmentos, recuperadores y RRF).
- [ADR 0007](adr/0007-que-se-indexa.md): qué se indexa (solo formatos útiles) y qué carpetas se saltan.
- [ADR 0008](adr/0008-distribucion-de-pdfium.md): PDFium para los PDF y cómo llega `pdfium.dll` (dentro del instalador, versión y hash fijados).
- [ADR 0009](adr/0009-busqueda-por-significado.md): búsqueda por significado (modelo elegido midiendo, vectores en SQLite, HNSW derivado, fusión y evaluación).
- [ADR 0010](adr/0010-ranking-hibrido.md): ranking híbrido (diagnóstico, búsqueda de palabras relajada, suma por archivo, frecencia, benchmark).
- [ADR 0011](adr/0011-espacios-en-los-resultados.md): Espacios mezclados con los archivos (exactos primero, parciales en la escala de los archivos) y rutas sugeridas del índice.
- [ADR 0012](adr/0012-esperas-en-los-espacios.md): esperas en los Espacios (solo cosas de tu equipo, el comando esperado sin ventana, cancelar sin cerrar nada).
- [ADR 0013](adr/0013-pdf-en-su-pagina-y-filtros.md): PDF en su página, filtros rápidos por tipo y barra vacía con lo reciente.
- [ADR 0014](adr/0014-colocar-ventanas.md): colocar ventanas en zonas (encontrar la ventana nueva, bordes invisibles, varias pantallas).
- [ADR 0015](adr/0015-bienvenida-y-permiso-de-descarga.md): bienvenida la primera vez y permiso antes de descargar el modelo.
- [ADR 0016](adr/0016-campos-y-movimiento-al-escribir.md): borde inferior de los campos a 3:1 y sin animación al escribir.
- [ADR 0017](adr/0017-icono.md): el icono, «el tamiz».
- [ADR 0018](adr/0018-actualizaciones.md): actualizaciones automáticas, opcionales y firmadas.
- [ADR 0019](adr/0019-instalador.md): el instalador (imágenes, textos en español y «Abrir Sift»).

## Mediciones por fase

| Métrica | Objetivo | Fase 1 | Fase 1.5 (Espacios + animación de entrada) |
|---|---|---|---|
| Atajo → ventana pintada (`RegisterHotKey`) | < 100 ms | p50 34 ms · p95 42 ms | p50 37 ms · p95 45 ms |
| Atajo → ventana pintada (hook de teclado) | < 100 ms | p50 34 ms · p95 40 ms | p50 36 ms · p95 43 ms |

| Búsqueda (50.000 fragmentos, índice Tantivy) | < 50 ms | | Fase 2: p50 6,3 ms · p95 12,1 ms |

### Calidad y velocidad de la búsqueda (Fases 4 y 5)

Con 34 consultas reales sobre una carpeta de la carrera (12.871 archivos, 29.477 textos con vector), definidas en [`eval/`](../eval/README.md). **recall@10** (de lo esperado, cuánto sale entre los 10 primeros) y, entre paréntesis, **MRR** (lo arriba que sale lo primero que vale):

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

