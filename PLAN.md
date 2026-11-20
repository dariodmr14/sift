# Sift — Plan del proyecto

> Nombre provisional. Lanzador y buscador semántico de escritorio para Windows.
> Plan por **fases**, no por semanas: se avanza cuando hay tiempo. Una fase está acabada cuando cumple su **criterio de hecho**. El plan está vivo: si surge una funcionalidad nueva, se añade al backlog o a una fase.

---

## 1. Qué es

Pulsas `Alt+Espacio` y aparece una barra flotante en el centro de la pantalla.

- **Buscar**: "normalización 3FN" → PDF del tema, apuntes, práctica… aunque no contengan esas palabras exactas.
- **Espacios**: "bases de datos" + Enter → se abre todo lo de ese contexto: carpeta de la asignatura, apuntes, gestor de BD, contenedor Docker de prácticas, página de Notion, Campus Virtual.
- **Todo local**: nada sale del ordenador. Ni el índice, ni los archivos, ni el modelo de IA.

### Objetivos de calidad (lo que lo hace "profesional")
| Métrica | Objetivo |
|---|---|
| Tiempo desde el atajo hasta ver la ventana | < 100 ms |
| Latencia de búsqueda (p95) | < 50 ms con ~50.000 fragmentos |
| Consumo en reposo | < 100 MB RAM, ~0 % CPU |
| Indexado | En segundo plano, sin bloquear la UI, se pausa si estás usando mucho el PC |
| Arranque en frío de la app | < 1 s |

---

## 2. Arquitectura

### 2.1 Vista general

```
┌──────────────────────────── App Tauri ────────────────────────────┐
│  UI (React + TS)  ⇄  IPC tipado (comandos + eventos)              │
│                                                                    │
│  Núcleo en Rust (workspace de crates)                              │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ watcher  │→ │ indexer   │→ │ storage  │  │ search           │  │
│  │ (notify) │  │ (cola de  │  │ SQLite   │← │ texto (Tantivy)  │  │
│  └──────────┘  │ trabajos) │  │ Tantivy  │  │ vectores (HNSW)  │  │
│                │ extract → │  │ HNSW     │  │ fusión + ranking │  │
│                │ chunk →   │  └──────────┘  └──────────────────┘  │
│                │ embed     │                                       │
│                └───────────┘   ┌──────────┐  ┌──────────────────┐  │
│                                │ spaces   │  │ embeddings (ONNX)│  │
│                                └──────────┘  └──────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

### 2.2 Decisiones técnicas (y por qué)

**App: Tauri 2 (Rust + WebView2) en lugar de Electron**
Ventana ligera, arranque rápido y bajo consumo en reposo, que es lo que importa en un lanzador que vive siempre abierto. El trabajo pesado (indexar, buscar, IA) va en Rust, nativo y sin pausas de recolector de basura. WebView2 ya viene con Windows 10/11.

**Almacenamiento: una fuente de verdad + índices derivados**
Es como lo hacen los motores de búsqueda serios: un almacén fiable con el estado y varios índices especializados que se pueden reconstruir desde él.

| Pieza | Tecnología | Para qué | Por qué |
|---|---|---|---|
| Fuente de verdad | **SQLite** (modo WAL) | Archivos, hashes, fragmentos, Espacios, historial de uso, ajustes | Transaccional, embebido, a prueba de cortes. Ideal para estado y metadatos. |
| Búsqueda de texto | **Tantivy** | Búsqueda por palabras (BM25), prefijos, tolerancia a erratas | Motor tipo Lucene en Rust. Más rápido y flexible que FTS5 de SQLite, con lematizador en español ("normalizar" encuentra "normalización"). |
| Búsqueda semántica | **usearch** (índice HNSW) | Vecinos más cercanos sobre los embeddings | Búsqueda aproximada en milisegundos aunque crezca el índice. |

> ¿Por qué no solo SQLite (FTS5 + sqlite-vec)? Funciona y es más simple, pero FTS5 tiene peor ranking y tokenización, y sqlite-vec hace búsqueda exacta (fuerza bruta), que escala peor. Si en la Fase 2 Tantivy diera problemas, FTS5 es el plan B.
> Referencia curiosa: el indexador de Windows (Windows Search) también separa estado e índices; lo que proponemos es la versión moderna de esa idea.

**Embeddings locales: `multilingual-e5-small` en ONNX Runtime (crate `ort`)**
Unos 118M de parámetros, vectores de 384 dimensiones, buen nivel en español y va bien en CPU. ONNX Runtime es el motor de inferencia de Microsoft; en Windows se puede acelerar con DirectML más adelante.
Detalle importante: e5 espera los prefijos `query: ` (consultas) y `passage: ` (documentos).

**Extracción de texto**
- PDF: **PDFium** (`pdfium-render`), el motor de PDF de Chrome. Mucho más fiable que los extractores puros en Rust.
- Word (.docx): descomprimir el zip y leer el XML (`zip` + `quick-xml`).
- Texto, Markdown y código: lectura directa, detectando la codificación.

**Detección de cambios**
- Fase 3: crate `notify` (usa `ReadDirectoryChangesW` de Windows) + hash de contenido para no reprocesar.
- Backlog: leer el **USN Journal de NTFS**, como hace Everything, para detectar cambios incluso con la app cerrada.

**Ranking híbrido**
Búsqueda de texto y búsqueda semántica en paralelo → fusión con **Reciprocal Rank Fusion (RRF)** → ajuste por **frecencia** (lo que abres a menudo y hace poco sube).

**UI**
React + TypeScript + Vite, Tailwind, Motion para animaciones, Zustand para estado. Tipos compartidos entre Rust y TS generados automáticamente (`tauri-specta`), para que un cambio en el backend rompa la compilación del frontend en lugar de fallar en ejecución.

**Atajo global en Windows**
`Alt+Espacio` está reservado por Windows para el menú de la ventana. Primero se intenta `RegisterHotKey`; si no se puede, un hook de teclado de bajo nivel (como PowerToys Run). El atajo es configurable.

**Seguridad de los Espacios**
Los comandos solo los define el usuario en la configuración. Se ejecutan con programa y argumentos separados, nunca concatenando texto en una shell (evita inyección de comandos). La primera vez que un Espacio ejecuta un comando, se pide confirmación.

**Calidad**
- Errores: `thiserror` en las librerías, `anyhow` en la app. Logs con `tracing` a fichero rotativo.
- Tests: `cargo test` (unitarios + integración con carpetas de prueba), Vitest en la UI.
- CI: GitHub Actions en `windows-latest`: formato, lint (`clippy`, ESLint), tests y build.
- Sin telemetría.

### 2.3 Estructura del repositorio

```
sift/
├─ apps/desktop/            # App Tauri
│  ├─ src/                  # UI React + TS
│  └─ src-tauri/            # Arranque, ventana, atajo, bandeja, comandos IPC
├─ crates/
│  ├─ core/                 # Tipos de dominio, configuración, errores
│  ├─ storage/              # SQLite: esquema, migraciones, repositorios
│  ├─ extract/              # PDF, docx, texto
│  ├─ indexer/              # Recorrido, watcher, cola de trabajos, troceado
│  ├─ embeddings/           # Modelo ONNX
│  ├─ search/               # Tantivy, HNSW, fusión y ranking
│  └─ spaces/               # Modelo y ejecución de Espacios
├─ eval/                    # Consultas de prueba y resultados esperados
├─ docs/adr/                # Registro de decisiones de arquitectura
├─ PLAN.md
└─ CLAUDE.md                # Reglas para Claude Code
```

---

## 3. Fases

### Fase 0 — Cimientos
- [x] Repo en GitHub, licencia (MIT; desde la 1.0.0-rc.1, todos los derechos reservados) y README mínimo.
- [x] Workspace de Rust con los crates vacíos + app Tauri 2 con React/TS/Vite.
- [x] Formato y lint: `rustfmt`, `clippy`, ESLint, Prettier.
- [x] CI en GitHub Actions (Windows): lint + tests + build.
- [x] `CLAUDE.md` con las reglas del proyecto (ver sección 5).
- [x] Primer ADR: por qué Tauri y por qué SQLite + Tantivy + HNSW.

**Hecho cuando:** `cargo test` y la CI pasan en verde y la app abre una ventana vacía.

### Fase 1 — El lanzador
- [x] Ventana sin bordes, centrada, siempre encima, con fondo translúcido (Mica/Acrylic).
- [x] Atajo global `Alt+Espacio` (con alternativa configurable).
- [x] Mostrar/ocultar: se oculta con `Esc` o al perder el foco.
- [x] Caja de búsqueda y lista de resultados de prueba, todo con teclado (↑ ↓ Enter).
- [x] Icono en la bandeja del sistema con Ajustes y Salir.
- [x] Opción de arrancar con Windows.

**Hecho cuando:** el atajo abre la barra en < 100 ms desde cualquier app y se maneja sin ratón.

### Fase 1.5 — Espacios básicos y ajustes desde la barra (adelantada)
> Adelantada de la Fase 6 tras usar la Fase 1 (ver [FEEDBACK.md](FEEDBACK.md)). Incluye lo que **no depende del indexado**.
- [x] Ajustes accesibles desde la barra: `Ctrl+,`, icono discreto y escribiendo "ajustes".
- [x] Persistencia en SQLite (`sift-storage`) con migraciones.
- [x] Modelo de Espacio: nombre, alias, icono y lista ordenada de acciones.
- [x] Acciones: abrir carpeta, abrir archivo, abrir URL, abrir aplicación (con selector) y ejecutar comando (programa + argumentos + carpeta de trabajo).
- [x] Seguridad: programa y argumentos por separado, sin shell; confirmación la primera vez que se ejecuta cada comando.
- [x] Editor en Ajustes: crear, editar, duplicar, borrar y reordenar; botón "probar" por acción.
- [x] Iconos: librería con buscador, emoji, imagen propia (PNG/SVG/ICO) o ninguno; icono real de Windows por defecto para apps y archivos; opción global "solo texto".
- [x] Exportar e importar los Espacios a JSON.
- [x] Sin datos de ejemplo: estado vacío con atajo para crear el primer Espacio.
- [x] Dirección visual elegida («Vidrio», [ADR 0005](docs/adr/0005-direccion-visual-vidrio.md)) y aplicada como sistema de diseño ([tokens](docs/design-system.md)), respetando "reducir movimiento".

**Hecho cuando:** creas un Espacio "bases de datos" desde Ajustes y, desde la barra, "bases" + Enter abre todas sus acciones (con confirmación la primera vez que hay un comando).

### Fase 2 — Indexado y búsqueda por palabras
> Arquitectura pensada para la Fase 4: ver [ADR 0006](docs/adr/0006-arquitectura-de-busqueda.md).
- [x] Esquema SQLite con migraciones: `roots`, `ignore_patterns`, `files`, `chunks` (fragmentos, la unidad de la Fase 4) y `opens`.
- [x] Ajustes → «Carpetas»: añadir y quitar carpetas (selector de Windows), patrones a ignorar y estado del indexado.
- [x] Recorrido de carpetas y extracción de texto/Markdown/código (con detección de codificación); el resto, por nombre.
- [x] Troceado en fragmentos con solapamiento.
- [x] Índice Tantivy con analizador en español; búsqueda con prefijos y erratas, en nombre y contenido.
- [x] Pipeline de recuperadores + fusión RRF (hoy solo el léxico; el semántico se añade en la Fase 4).
- [x] La barra busca en todo a la vez: Espacios, comandos y archivos, con secciones.
- [x] Resultados con nombre, ruta, fragmento con el término resaltado e icono por tipo.
- [x] Acciones: abrir, abrir carpeta contenedora, copiar ruta.
- [x] Latencia de búsqueda medida (objetivo < 50 ms).

**Hecho cuando:** buscas una palabra que está dentro de un apunte y aparece en < 50 ms. ✅ p50 6,3 ms · p95 12,1 ms con 50.000 fragmentos.

### Fase 3 — Más formatos e indexado incremental
> Qué se indexa: [ADR 0007](docs/adr/0007-que-se-indexa.md). Cómo llega `pdfium.dll`: [ADR 0008](docs/adr/0008-distribucion-de-pdfium.md).
- [x] Extracción de PDF (PDFium) y Word (también PowerPoint y odt). Protegidos y dañados, con su motivo.
- [x] Watcher de carpetas + hash de contenido: solo se reprocesa lo que cambia. Renombrar o mover conserva el archivo (se reconoce por su contenido) y no deja resultados rotos.
- [x] Cola de trabajos en segundo plano con prioridad (lo cambiado hace poco va primero) y límite de CPU; prioridad baja de Windows y pausa si el PC está ocupado.
- [x] Progreso del indexado visible en la UI, en lenguaje humano.
- [x] Recuperación ante fallos: si la app se cierra a mitad, retoma donde iba.

**Hecho cuando:** guardas un PDF nuevo en una carpeta y en segundos aparece en la búsqueda, sin reindexar todo. ⏳ Comprobado en la app con datos reales: 542 de 543 PDF y 750 Word leídos (el que falta, dañado), y el vigilante probado sobre el disco real. Falta la prueba a mano de guardar un PDF nuevo y verlo en la barra.

### Fase 4 — Búsqueda semántica
> Diseño, elección del modelo y resultados: [ADR 0009](docs/adr/0009-busqueda-por-significado.md).
- [x] Troceado de documentos (por párrafos/secciones, con solapamiento): el de la Fase 2, con una política de qué lleva vector (documentos, apuntes, código; `.txt` hasta 20 fragmentos; datos, no) y un vector por texto distinto.
- [x] Modelo en ONNX Runtime, descargado la primera vez: `multilingual-e5-small`, **elegido midiendo** frente a Granite 97M R2.
- [x] Cálculo de embeddings en segundo plano dentro de la cola de trabajos (prioridad más baja, mismo límite de CPU y pausa).
- [x] Índice HNSW (usearch), persistido en disco y reconstruible desde SQLite.
- [x] Búsqueda por significado conectada a la UI (marca «Por significado», estado en Carpetas, interruptor en Ajustes).
- [x] Adelantado de la Fase 5: fusión RRF ponderada por tipo de búsqueda y `eval/` con 34 consultas reales.

**Hecho cuando:** "cómo evitar redundancia en tablas" encuentra el tema de normalización sin compartir palabras. ✅ **Cumplido en la Fase 5**: trae `apuntes_bd_completos.pdf` el 2.º (ver ADR 0010). Al cerrar la Fase 4 no se cumplía: la búsqueda por palabras exigía todas las palabras y no aportaba nada, y HNSW perdía el fragmento bueno.

### Fase 5 — Ranking híbrido y medición
- [x] Búsqueda de texto + semántica y fusión con RRF (adelantado a la Fase 4).
- [x] Frecencia: lo abierto a menudo y hace poco sube (vida media de 14 días, empujón de hasta +25 %, solo reordena; calibrada simulando sobre `eval/`).
- [x] Carpeta `eval/` con ~30 consultas reales y sus resultados esperados; métrica *recall@10* (adelantado a la Fase 4).
- [x] Benchmark de latencia (p50/p95) con `criterion`, reproducible (corpus sintético determinista). Encontró que resaltar era lo más caro: la híbrida pasó de p95 32,6 a 17,0 ms.
- [x] Ajustar pesos con los datos, no a ojo: `ef` de HNSW, fragmentos que suman por archivo, candidatos semánticos, búsqueda de palabras relajada y peso de la frecencia, cada uno con su rejilla en `eval/` (ADR 0010).
- [x] Mejorar las búsquedas de ideas que aún fallan: diagnóstico con datos y, sobre todo, búsqueda de palabras relajada para frases. Ideas 0,56 → 0,69; total 0,65 → 0,75. El *reranker* y el contexto del documento, al backlog (ADR 0010).
- [ ] Ampliar `eval/` con las consultas que fallen en el uso real (continuo: según lo que vaya saliendo).

**Hecho cuando:** hay números en el README (recall y latencia) y mejoran frente a usar cada búsqueda por separado. ✅ Híbrida 0,75 frente a 0,62 (solo palabras) y 0,58 (solo significado), con p95 por debajo de 50 ms (ver README).

### Fase 6 — Espacios completos
> La versión básica (modelo, acciones, editor, confirmación) se adelantó a la Fase 1.5. Aquí queda lo que se apoya en el indexado.
- [x] Los Espacios se mezclan con los resultados de búsqueda y suben con la frecencia (nombre o alias exacto → primero; parcial → intercalado con los archivos en su misma escala; ADR 0011).
- [x] Sugerir archivos y carpetas del índice al añadir acciones (por nombre, con frecencia; se mantiene el selector de Windows).
- [x] Acciones condicionales o con espera (p. ej. abrir la web cuando el contenedor esté listo): paso «Esperar a que…» (web o servicio de tu equipo, fin del comando anterior, unos segundos), «Si no ocurre en X segundos…», Probar, progreso en la barra y Esc para cancelar (ADR 0012).

**Hecho cuando:** "bases de datos" + Enter abre todo tu entorno de la asignatura. ✅ El nombre o alias exacto sale el primero, y con las esperas el entorno se abre en orden (contenedores → web cuando responde), diciendo qué falló si algo falla.

### Mejoras antes de la Fase 6.5 (adelantadas)
- [x] Textos para cualquiera: sin jerga ni ejemplos de los apuntes; lo técnico, en «Avanzado» y explicado.
- [x] Barra vacía con «Espacios» y «Abierto recientemente».
- [x] Abrir el PDF en la página del fragmento que encaja («pág. 37»), con el lector predeterminado (ADR 0013).
- [x] Filtros rápidos por tipo («pdf …», «código …», «imagen …») con etiqueta que se quita con Retroceso.

### Fase 6.5 — Colocar ventanas (zonas)
Acciones que, además de abrir una app, colocan su ventana en una posición o zona de la pantalla (p. ej. "abrir VS Code en la mitad derecha"), como hacen los FancyZones de PowerToys.
- [x] Zonas predefinidas (mitades, tercios, dos tercios, cuartos, toda la pantalla). Las personalizadas, al backlog.
- [x] Esperar a que la app abra: hasta 15 s a que aparezca su ventana; si no, sigue y avisa.
- [x] Identificar la ventana correcta: una ventana **nueva** del programa esperado (sirve con apps que reutilizan su proceso), con el título para desempatar.
- [x] Varios monitores: principal, donde esté el ratón o una concreta (nombre + posición); área útil y escala de cada pantalla.
- [x] Ventanas ya abiertas: moverlas en lugar de abrir otra (opcional por acción).
- [x] Colocar con `SetWindowPos`, compensando los bordes invisibles de Windows 10 (`DwmGetWindowAttribute`). Comprobado en pantalla, exacto al píxel (ADR 0014).

**Hecho cuando:** un Espacio abre VS Code en la mitad derecha y el navegador en la izquierda del monitor elegido, aunque alguna de las dos ya estuviera abierta. ✅ Comprobado con el Bloc de notas y el Explorador (que, como VS Code, reutiliza su proceso) en las dos pantallas; con VS Code y el navegador de Darío, pendiente de su uso real.

### Fase 7 — Pulido
- [x] Instalar y usar la skill unslop (https://github.com/heydamir/unslop, licencia CC BY 4.0: citar a Damir Sokolovsky en el README) para auditar y rediseñar la interfaz, respetando el sistema de diseño Vidrio y justificando cualquier cambio en un ADR. Usarla también para el vídeo de LinkedIn.
- [x] Diseño final: tipografía, espaciado, tema claro/oscuro según Windows.
- [x] Animaciones de entrada/salida y de lista (rápidas, sin estorbar).
- [x] Vista previa del resultado seleccionado.
- [x] Pantalla de bienvenida: elegir carpetas y crear el primer Espacio.
- [x] Accesibilidad básica: contraste, foco visible, lector de pantalla.

**Hecho cuando:** una persona que no lo conoce lo instala y lo usa sin explicaciones.

**Estado (2026-11-18):** todas las tareas hechas y comprobadas en pantalla (`docs/capturas/final/`). Falta lo único que no se puede comprobar solo: que alguien que no conoce Sift lo instale y lo use sin explicaciones. Cuando Darío lo pruebe con una persona, se cierra la fase. Guion de esa prueba y de la instalación limpia en Windows Sandbox: [docs/prueba-instalacion.md](docs/prueba-instalacion.md).

### Fase 8 — Lanzamiento
- [ ] Instalador (NSIS/MSI) y release en GitHub con GitHub Actions.
- [ ] Actualizaciones automáticas (plugin updater de Tauri).
- [ ] README completo: GIF, instalación, arquitectura, decisiones y métricas.
- [ ] Vídeo demo de 30–60 s y post en LinkedIn. **Se graba con la versión final**, para no enseñar pantallas antiguas (decidido el 2026-11-19). Hay una primera versión fuera del repo, en el equipo de Darío (`Documents\Sift - vídeo de presentación`), con su proyecto de HyperFrames y su guion de grabación.

- [ ] Preparación de la publicación (apuntado el 2026-11-20):
  - README en dos idiomas: `README.md` en español (principal) y `README.en.md` en inglés, con un enlace arriba para cambiar de idioma y el vídeo de cada idioma.
  - Vídeo en inglés: las ventanas y los datos de ejemplo salen en español (p. ej. «Calculadora»); para la versión en inglés, datos de ejemplo y nombres en inglés.
  - Valorar la traducción de la app al inglés (ver backlog).

**Hecho cuando:** cualquiera puede descargarlo desde GitHub e instalarlo.

---

## 4. Backlog (ideas para después)
- Conectores: Notion, Google Calendar, marcadores del navegador.
- Sugerencia automática de Espacios agrupando archivos por tema con IA.
- USN Journal de NTFS para detectar cambios con la app cerrada.
- Aceleración del modelo con DirectML (GPU).
- Búsqueda de apps instaladas y comandos del sistema.
- Calculadora y conversiones en la barra.
- Sistema de extensiones.
- macOS y Linux.
- Traducción de la app al inglés (textos de la interfaz, datos de demostración y vídeo; ver Fase 8).
- Firma de código del instalador (evita el aviso de SmartScreen; tiene coste).
- ~~Icono propio de la app y de la bandeja~~ ✅ Hecho (2026-11-18): «el tamiz», ADR 0017.
- Grabar el atajo pulsándolo en Ajustes, en vez de escribirlo como texto.
- Zonas propias para colocar ventanas (dibujarlas por pantalla, como FancyZones). El modelo ya lo admite (ADR 0014).
- Colocar también la ventana de un comando (`code .`): habría que saber qué programa abre.
- Búsqueda: reordenar los primeros resultados con un *reranker* local, si las ideas siguen fallando y cabe en los 50 ms (ADR 0010).
- Búsqueda: calcular el vector de cada fragmento con el contexto de su documento (título, asignatura). Obliga a recalcular todos los vectores (ADR 0010).
- ~~Búsqueda: agrupar copias del mismo archivo en varias carpetas~~ ✅ Hecho (2026-11-18): una copia con el mismo nombre y el mismo fragmento ya no se repite (39 → 12 repeticiones en `eval/`, sin cambiar recall ni MRR). Falta decidir si decir «y N copias más» (ver `docs/proceso/pendiente-decidir.md`).
- Barra: volver a buscar sola cuando el indexado termina, si está abierta (hoy, durante el primer indexado, los resultados no se actualizan hasta que cambias el texto).

---

## 5. Cómo trabajamos (y reglas para `CLAUDE.md`)
- Una rama y un PR por tarea o grupo pequeño de tareas; nunca todo de golpe.
- Cada PR con tests y la CI en verde antes de fusionar.
- Claude Code explica cada decisión importante; Darío revisa y entiende el código antes de aceptarlo.
- Decisiones de arquitectura nuevas → un ADR corto en `docs/adr/`.
- Si aparece una idea nueva a mitad de una fase → al backlog, salvo que bloquee la fase actual.
- Al acabar cada fase: marcar casillas aquí, actualizar el README y apuntar qué se aprendió (material para LinkedIn y entrevistas).
