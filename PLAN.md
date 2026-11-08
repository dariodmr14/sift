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
- [ ] Repo en GitHub, licencia (MIT) y README mínimo.
- [ ] Workspace de Rust con los crates vacíos + app Tauri 2 con React/TS/Vite.
- [ ] Formato y lint: `rustfmt`, `clippy`, ESLint, Prettier.
- [ ] CI en GitHub Actions (Windows): lint + tests + build.
- [ ] `CLAUDE.md` con las reglas del proyecto (ver sección 5).
- [ ] Primer ADR: por qué Tauri y por qué SQLite + Tantivy + HNSW.

**Hecho cuando:** `cargo test` y la CI pasan en verde y la app abre una ventana vacía.

### Fase 1 — El lanzador
- [ ] Ventana sin bordes, centrada, siempre encima, con fondo translúcido (Mica/Acrylic).
- [ ] Atajo global `Alt+Espacio` (con alternativa configurable).
- [ ] Mostrar/ocultar: se oculta con `Esc` o al perder el foco.
- [ ] Caja de búsqueda y lista de resultados de prueba, todo con teclado (↑ ↓ Enter).
- [ ] Icono en la bandeja del sistema con Ajustes y Salir.
- [ ] Opción de arrancar con Windows.

**Hecho cuando:** el atajo abre la barra en < 100 ms desde cualquier app y se maneja sin ratón.

### Fase 2 — Indexado y búsqueda por palabras
- [ ] Esquema SQLite con migraciones: `roots`, `files`, `chunks`, `opens`.
- [ ] Ajustes: elegir carpetas a indexar y patrones a ignorar (`node_modules`, `.git`…).
- [ ] Recorrido de carpetas y extracción de texto/Markdown/código.
- [ ] Índice Tantivy con analizador en español; búsqueda con prefijos y erratas.
- [ ] Resultados con nombre, ruta, fragmento con el término resaltado e icono por tipo.
- [ ] Acciones: abrir, abrir carpeta contenedora, copiar ruta.

**Hecho cuando:** buscas una palabra que está dentro de un apunte y aparece en < 50 ms.

### Fase 3 — Más formatos e indexado incremental
- [ ] Extracción de PDF (PDFium) y Word.
- [ ] Watcher de carpetas + hash de contenido: solo se reprocesa lo que cambia.
- [ ] Cola de trabajos en segundo plano con prioridad y límite de CPU.
- [ ] Progreso del indexado visible en la UI.
- [ ] Recuperación ante fallos: si la app se cierra a mitad, retoma donde iba.

**Hecho cuando:** guardas un PDF nuevo en una carpeta y en segundos aparece en la búsqueda, sin reindexar todo.

### Fase 4 — Búsqueda semántica
- [ ] Troceado de documentos (por párrafos/secciones, con solapamiento).
- [ ] Modelo `multilingual-e5-small` en ONNX Runtime, descargado la primera vez.
- [ ] Cálculo de embeddings en segundo plano dentro de la cola de trabajos.
- [ ] Índice HNSW (usearch), persistido en disco y reconstruible desde SQLite.
- [ ] Búsqueda por significado conectada a la UI.

**Hecho cuando:** "cómo evitar redundancia en tablas" encuentra el tema de normalización sin compartir palabras.

### Fase 5 — Ranking híbrido y medición
- [ ] Búsqueda de texto + semántica en paralelo y fusión con RRF.
- [ ] Frecencia: lo abierto a menudo y hace poco sube.
- [ ] Carpeta `eval/` con ~30 consultas reales y sus resultados esperados; métrica *recall@10*.
- [ ] Benchmark de latencia (p50/p95) con `criterion`.
- [ ] Ajustar pesos con los datos, no a ojo.

**Hecho cuando:** hay números en el README (recall y latencia) y mejoran frente a usar cada búsqueda por separado.

### Fase 6 — Espacios
- [ ] Modelo: nombre, alias, icono y lista de acciones.
- [ ] Acciones: abrir archivo/carpeta, abrir URL, abrir app, ejecutar comando (p. ej. `docker compose up` en una ruta).
- [ ] Editor de Espacios en Ajustes.
- [ ] Los Espacios aparecen en los resultados al escribir su nombre o alias; Enter lanza todo.
- [ ] Confirmación la primera vez que un Espacio ejecuta un comando.

**Hecho cuando:** "bases de datos" + Enter abre todo tu entorno de la asignatura.

### Fase 7 — Pulido
- [ ] Diseño final: tipografía, espaciado, tema claro/oscuro según Windows.
- [ ] Animaciones de entrada/salida y de lista (rápidas, sin estorbar).
- [ ] Vista previa del resultado seleccionado.
- [ ] Pantalla de bienvenida: elegir carpetas y crear el primer Espacio.
- [ ] Accesibilidad básica: contraste, foco visible, lector de pantalla.

**Hecho cuando:** una persona que no lo conoce lo instala y lo usa sin explicaciones.

### Fase 8 — Lanzamiento
- [ ] Instalador (NSIS/MSI) y release en GitHub con GitHub Actions.
- [ ] Actualizaciones automáticas (plugin updater de Tauri).
- [ ] README completo: GIF, instalación, arquitectura, decisiones y métricas.
- [ ] Vídeo demo de 30–60 s y post en LinkedIn.

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
- Firma de código del instalador (evita el aviso de SmartScreen; tiene coste).

---

## 5. Cómo trabajamos (y reglas para `CLAUDE.md`)
- Una rama y un PR por tarea o grupo pequeño de tareas; nunca todo de golpe.
- Cada PR con tests y la CI en verde antes de fusionar.
- Claude Code explica cada decisión importante; Darío revisa y entiende el código antes de aceptarlo.
- Decisiones de arquitectura nuevas → un ADR corto en `docs/adr/`.
- Si aparece una idea nueva a mitad de una fase → al backlog, salvo que bloquee la fase actual.
- Al acabar cada fase: marcar casillas aquí, actualizar el README y apuntar qué se aprendió (material para LinkedIn y entrevistas).
