# CLAUDE.md — Reglas del proyecto Sift

Sift es un lanzador y buscador semántico de escritorio para Windows (Tauri 2 + Rust + React/TS). Léelo al empezar cada sesión.

## 1. El proyecto

`Alt+Espacio` abre una barra flotante con efecto vidrio que:
- busca en los archivos del usuario por nombre, por contenido y por significado;
- lanza **Espacios**: conjuntos de acciones (abrir carpetas, archivos, webs y apps, ejecutar comandos, esperar a que algo esté listo y colocar ventanas en zonas de la pantalla).

Todo es local: lo único que sale del PC son descargas puntuales (modelo de embeddings, PDFium), con el hash fijado y con permiso del usuario. La única excepción es mirar al arrancar si hay una versión nueva (encendido por defecto, se apaga en Ajustes; descargarla e instalarla siempre se pregunta, ADR 0018).

**Stack:** Tauri 2 (núcleo en Rust y UI en React + TypeScript + Vite + Tailwind 4). SQLite es la fuente de verdad; Tantivy se encarga de la búsqueda por palabras y usearch (HNSW) de los vectores. Embeddings `multilingual-e5-small` sobre ONNX Runtime; búsqueda híbrida (RRF + frecencia); PDFium para leer PDF.

**Documentos de referencia:**
| Archivo | Para qué |
|---|---|
| `PLAN.md` | Fases, criterios de "hecho" y backlog. Léelo antes de empezar una tarea. |
| `FEEDBACK.md` | Lo que nota Darío usando la app |
| `docs/adr/` | Decisiones de arquitectura (plantilla: `docs/adr/0001-*.md`) |
| `docs/design-system.md` | Sistema de diseño Vidrio |
| `docs/proceso/unslop-direccion.md` | Dirección de diseño aprobada (carácter, momentos y firma) |
| `docs/proceso/auditoria-fase-7.md` | Auditoría y orden de los bloques de la Fase 7 |
| `docs/capturas/` | Capturas de antes y después de las pasadas de diseño |
| `docs/aprendido.md` | Lo aprendido en cada fase |
| `docs/instalar.md` | Instalar la versión de uso diario |
| `eval/` | Consultas de evaluación de la búsqueda |

## 2. Estructura y comandos

**Estructura:**
- `apps/desktop/`: app Tauri. `src/` es la UI (React + TS; la barra en `src/launcher/`, Ajustes en `src/settings/`) y `src-tauri/` el arranque y los comandos IPC.
- `crates/*`: núcleo en Rust (`core`, `storage`, `extract`, `indexer`, `embeddings`, `search`, `spaces`).
- `eval/`: consultas de evaluación. `docs/adr/`: decisiones. `demo/`: datos de demostración inventados (`SIFT_DEV_DEMO=1`). `scripts/`: CI local, instalador, PDFium, e2e y capturas (`capturas-final.mjs` y `capturas-intro.mjs`, con los datos de demostración).

**Comandos** (desde la raíz del repo, salvo que se indique otra cosa):
- **Rust:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- **UI** (desde `apps/desktop/`): `npm run lint`, `npm run format`, `npm run typecheck`, `npm test`
- **App en desarrollo** (desde `apps/desktop/`): `npm run tauri dev`
- **CI local completa** (lo mismo que el workflow): `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1`
- **Prueba de extremo a extremo** (abre ventanas, así que pide permiso antes): `node scripts/e2e.mjs`
- **Versión diaria:** `scripts/instalar.ps1` desde `main` (ver `docs/instalar.md`)

## 3. Entorno y datos de Darío

- Windows 10 Pro. Donde el plan diga Mica, se usa Acrylic (Mica es solo de Windows 11). Las rutas llevan tildes (`Darío`): no asumas ASCII.
- Editor: VS Code. Darío usa GitHub Desktop para sus repos de la uni; en Sift, Claude se encarga de GitHub (sección 9).
- **Nunca toques los datos reales.** `npm run tauri dev` usa sus propios datos (`%APPDATA%\dev.sift.app-dev`, atajo `Ctrl+Alt+Espacio`). Los datos de la versión diaria (`%APPDATA%\dev.sift.app`) solo se leen, y solo si hace falta para diagnosticar.
- No ejecutes `npm ci` con un `tauri dev` abierto (`ci.ps1` ya lo comprueba).
- **Pruebas en pantalla** (abrir ventanas, teclado o ratón): pide permiso antes. Si Darío dice que se va o que sigues solo, puedes hacerlas sin preguntar. Al acabar, cierra todo lo que hayas abierto.

## 4. Cómo trabajamos

- **Idioma:** responde siempre a Darío en español. Código, identificadores y commits en inglés; los textos de la interfaz, en español.
- **Al empezar una fase o rama:** lee `PLAN.md` y `FEEDBACK.md`. Resuelve lo pendiente de `FEEDBACK.md` que encaje con el trabajo actual o cueste poco; del resto, di qué se queda para más adelante y por qué. Marca como resuelto lo que hagas.
- **Decisiones importantes** (arquitectura, cambios visibles de diseño o de comportamiento, dependencias grandes): propón, justifica con alternativas y **para** hasta que Darío decida. Cada decisión aceptada va a un ADR corto.
- **Explica el porqué** de cada decisión técnica relevante, no solo el qué: Darío tiene que entender y poder defender el código.
- **Ideas nuevas a mitad de una fase:** al backlog de `PLAN.md`, salvo que bloqueen la fase actual.
- **Al acabar cada fase:** marca las casillas en `PLAN.md`, actualiza el README y apunta lo aprendido en `docs/aprendido.md`.
- **No des nada por hecho sin probarlo.** En los resúmenes, separa siempre lo comprobado de lo no comprobado.

## 5. Reparto de trabajo entre modelos

Tú (el modelo principal) eres el **coordinador**. Antes de empezar, clasifica cada parte de la tarea:

| Tipo de trabajo | Quién lo hace |
|---|---|
| Entender el problema, diseñar, decidir arquitectura, dividir la tarea | Tú |
| Depurar errores difíciles, revisar el resultado final, decidir si algo está bien | Tú |
| Implementar código cuando el diseño ya está decidido | Subagente `implementador` (Sonnet) |
| Textos, README, comentarios, renombrados, formateo, búsquedas simples | Subagente `mecanico` (Haiku) |

Reglas:
1. **Delega solo partes bien definidas.** Indica qué ficheros, qué firmas, qué comportamiento esperas y qué tests deben pasar. Una especificación vaga produce trabajo que hay que rehacer.
2. **No delegues cambios pequeños** (unas pocas líneas): hazlos tú, delegar cuesta más.
3. **Tareas independientes en paralelo** a varios subagentes, siempre que no toquen los mismos ficheros.
4. **Los subagentes no hacen commits, push ni merge**, ni deciden arquitectura o diseño. Si ven una ambigüedad, paran e informan.
5. **Revisa siempre tú lo que hagan** (lee el diff y ejecuta los tests) antes de dar la tarea por cerrada.
6. **Al terminar**, di en una línea qué hizo cada modelo.

## 6. Reglas de código

**Rust:**
- Errores con `thiserror` en los crates de librería y `anyhow` solo en la app. Nada de `unwrap()`/`expect()` fuera de los tests.
- Logs con `tracing`. Nunca se registran búsquedas ni contenido de archivos. Sin telemetría.
- `unsafe` prohibido salvo donde esté justificado y comentado.
- Un fallo inesperado en un hilo no cierra Sift: se captura, se registra y se muestra en el estado.

**TypeScript/React:**
- Tipos estrictos. Los bindings los genera `tauri-specta`; nunca se editan a mano.
- Componentes pequeños, con la lógica en hooks.
- Tests de lo nuevo: Vitest y, para el comportamiento con teclado, React Testing Library.

**Rendimiento (siempre medido, nunca supuesto):** la barra se abre en menos de 100 ms y la búsqueda responde en menos de 50 ms p95. El indexado va en segundo plano, con límite de CPU y en pausa si el PC está ocupado.

**Búsqueda:** cualquier cambio en el ranking o en el modelo se mide con `eval/` (recall@10 y MRR), nunca a ojo.

**Antes de dar una tarea por terminada:** fmt, clippy y tests en verde. Antes de cada merge, `scripts/ci.ps1` completo.

## 7. Diseño (sistema Vidrio)

- Lee `docs/design-system.md` antes de tocar la UI y actualízalo si añades o cambias un componente.
- **Solo tokens** (`bg-surface`, `text-muted`, `bg-accent`, `rounded-item`…), nunca colores literales. Lo comprueba `design-tokens.test.ts`.
- Listas seleccionables con `.sift-option`; controles de formulario de `src/settings/ui.tsx`.
- Criterio Windows 11 / Fluent:
  - Acrylic, con alternativa blur u opaca.
  - El acento de Windows, en su variante legible.
  - Segoe UI Variable (Segoe UI en Windows 10), con escala de tipo 12 / 14 / 14 seminegrita / 20 y cabeceras con mayúscula inicial en seminegrita.
  - Iconos Segoe Fluent Icons (Segoe MDL2 Assets en Windows 10). Lucide solo para los iconos que elige el usuario en sus Espacios.
  - Radios de 4 y 8 px.
- Animaciones: solo opacidad y transformaciones, con los tokens de duración y nada si está activo "reducir movimiento". No pueden afectar a la apertura en menos de 100 ms.
- Accesibilidad WCAG 2.2 AA: foco visible, contraste, objetivos de al menos 24 px, diálogos que atrapan el foco y anuncios para lector de pantalla.
- **Textos para cualquier persona:** sin jerga ni ejemplos sacados de los archivos de Darío. Los términos técnicos van solo en "Avanzado" y explicados.
- Antes de dar por buena una pantalla, revisa alineación, espaciado y jerarquía visual. Para auditar o pulir la interfaz, usa la skill `unslop` (instalada a nivel de usuario) respetando Vidrio.

## 8. Seguridad y privacidad

- Los comandos de los Espacios se ejecutan con programa y argumentos por separado, **nunca** concatenados en una shell.
- La aprobación de comandos la pide Rust con un diálogo nativo de Windows, nunca la UI.
- Las esperas solo comprueban direcciones locales (`localhost`, `127.0.0.1`, `[::1]`), sin redirecciones ni proxy.
- CSP estricta.
- Toda descarga lleva su hash fijado y pide permiso la primera vez.

## 9. Flujo de GitHub

**Credenciales:** Claude hace push, abre el PR y lo fusiona con `gh`, autenticado con un token *fine-grained* que solo tiene acceso a este repo. El push usa un credential helper **local** (`.git/config`) que llama a `gh auth git-credential`. **Nunca** se tocan la configuración global de git ni el gestor de credenciales de Windows.

**La CI se ejecuta en local.** El presupuesto de Actions es 0 $ y los minutos gratis se agotaron, así que el workflow de GitHub solo se lanza a mano (`workflow_dispatch`; cómo reactivarlo, en `.github/workflows/ci.yml`). GitHub sigue guardando ramas y PR: es gratis y es la copia de seguridad.

**Para cada bloque de trabajo:**
1. Rama nueva desde `main` actualizado (`feat/…`, `fix/…`, `chore/…`, `docs/…`, `ci/…`). Una rama y un PR por tarea o grupo pequeño de tareas; nunca todo de golpe.
2. Commits pequeños con Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `ci:`, `refactor:`, `test:`…), en inglés. **Sin** líneas de atribución (`Co-Authored-By`, "Generated with…") en commits ni PR: lo desactiva `.claude/settings.json`.
3. `scripts/ci.ps1` completo, sin `-SkipInstall` ni `-SkipBuild` (fmt, PDFium, clippy, tests de Rust, cargo audit, npm ci, npm audit, prettier, eslint, tsc, vitest y build de release). Se para al primer error; se arregla y se vuelve a ejecutar hasta que todo esté en verde.
4. `git push -u origin <rama>` y `gh pr create`. La descripción incluye qué cambia, cómo probarlo, decisiones importantes, enlaces a los ADR y la tabla final de la CI local.
5. `gh pr merge --merge --delete-branch`, `git checkout main` y `git pull`. No se espera a la CI de GitHub.
6. En el resumen final, el enlace del PR y si quedó fusionado.

**Nunca:**
- Merge con la CI local en rojo, a medias o sin ejecutar.
- Force push (`--force`, `-f`, `--force-with-lease`, `+rama`), y menos a `main` (lo bloquea `.claude/settings.json`), salvo autorización explícita de Darío para un caso concreto.
- Seguir adelante si git se complica (historiales que divergen, conflictos, un push rechazado): primero se para y se avisa a Darío.

## 10. Resumen al terminar

En español y breve:
1. Qué ha cambiado.
2. Cómo probarlo (los pasos que tiene que hacer Darío).
3. Qué está comprobado y qué no.
4. Decisiones que necesitan su visto bueno.
5. Enlace del PR y si quedó fusionado.
6. Una línea con qué hizo cada modelo.

Si te acercas al límite de uso, cierra lo que tengas con un commit y deja un resumen de dónde te has quedado.
