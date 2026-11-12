# CLAUDE.md — Reglas del proyecto Sift

Sift es un lanzador y buscador semántico de escritorio para Windows (Tauri 2 + Rust + React/TS).
El plan completo, con fases y criterios de "hecho", está en [PLAN.md](PLAN.md). Léelo antes de empezar una tarea.

## Entorno de Darío
- Windows 10 Pro: donde el plan dice Mica, usar **Acrylic** (Mica es solo Windows 11).
- Editor: VS Code. Darío usa GitHub Desktop para sus repos de la uni; en Sift, Claude se encarga de GitHub (ver «Flujo de GitHub»).

## Cómo trabajamos
- Una rama y un PR por tarea o grupo pequeño de tareas; nunca todo de golpe.
- Cada PR con tests y la CI local (`scripts/ci.ps1`) en verde antes de fusionar.
- Commits pequeños con [Conventional Commits](https://www.conventionalcommits.org/es/): `feat:`, `fix:`, `chore:`, `docs:`, `ci:`, `refactor:`, `test:`…
- Claude Code explica cada decisión importante; Darío revisa y entiende el código antes de aceptarlo. Explicar el *porqué*, no solo el *qué*.
- Decisiones de arquitectura nuevas → un ADR corto en `docs/adr/` (plantilla: `docs/adr/0001-*.md`).
- Si aparece una idea nueva a mitad de una fase → al backlog de PLAN.md, salvo que bloquee la fase actual.
- Al acabar cada fase: marcar casillas en PLAN.md, actualizar el README y apuntar qué se aprendió.

## Flujo de GitHub
Claude hace push, abre el PR y lo fusiona, con `gh` (autenticado con un token que solo tiene acceso a este repo). El push usa un *credential helper* **local de este repo** (`.git/config`) que llama a `gh auth git-credential`; la configuración global de git y el gestor de credenciales de Windows no se tocan.

**La CI se ejecuta en local.** El presupuesto de Actions es 0 $ y los minutos gratis se agotaron, así que el workflow de GitHub solo corre a mano (`workflow_dispatch`; cómo reactivarlo, en `.github/workflows/ci.yml`). GitHub sigue guardando ramas y PR (gratis, y es la copia de seguridad).

Para cada fase o rama, al terminar:
1. `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1`, **completa** (sin `-SkipInstall` ni `-SkipBuild`): fmt, PDFium, clippy, tests de Rust, `npm ci`, prettier, eslint, tsc, vitest y build de la app, lo mismo que el workflow. Para al primer error; se arregla y se vuelve a ejecutar hasta verlo todo en verde.
2. `git push -u origin <rama>` y `gh pr create` con una descripción clara: qué cambia, cómo probarlo, decisiones importantes, enlaces a los ADR y **el resumen de la CI local** (la tabla final del script).
3. `gh pr merge --merge --delete-branch`, `git checkout main` y `git pull`. No se espera a la CI de GitHub.
4. En el resumen final: el enlace del PR y si quedó fusionado.

Nunca:
- Merge con la CI local en rojo, a medias o sin ejecutar.
- Force push (`--force`, `-f`, `--force-with-lease`, `+rama`), y menos a `main`: lo bloquea `.claude/settings.json`.
- Si algo de git se complica (historiales que divergen, conflictos, un push rechazado), **parar y avisar a Darío antes de tocar nada**.

## Estructura
- `apps/desktop/` — app Tauri (`src/` UI React + TS, `src-tauri/` arranque y comandos IPC).
- `crates/*` — núcleo en Rust: `core`, `storage`, `extract`, `indexer`, `embeddings`, `search`, `spaces`.
- `eval/` — consultas de evaluación. `docs/adr/` — decisiones de arquitectura.

## Comandos
Desde la raíz del repo:
- Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- UI (desde `apps/desktop/`): `npm run lint`, `npm run format`, `npm run typecheck`, `npm test`
- App en desarrollo: `npm run tauri dev` (desde `apps/desktop/`). Usa sus propios datos (`%APPDATA%\dev.sift.app-dev`, atajo Ctrl+Alt+Space): nunca toca los de la versión diaria de Darío (`%APPDATA%\dev.sift.app`).
- Versión diaria: `scripts/instalar.ps1` desde `main` (ver `docs/instalar.md`).
- CI local completa (lo mismo que el workflow): `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1`

## Diseño
- Sistema de diseño «Vidrio»: lee [docs/design-system.md](docs/design-system.md) antes de tocar la UI.
- Solo tokens (`bg-surface`, `text-muted`, `bg-accent`, `rounded-item`…): nada de colores literales. Lo comprueba `design-tokens.test.ts`.
- Listas seleccionables con `.sift-option`; controles de formulario de `src/settings/ui.tsx`.
- Animaciones: solo opacidad y transformaciones, con los tokens de duración, y nada con «reducir movimiento». Que no afecten a la apertura en < 100 ms.

## Reglas de código
- Errores: `thiserror` en los crates de librería, `anyhow` solo en la app. Nada de `unwrap()` fuera de tests.
- Logs con `tracing`. Sin telemetría.
- Los comandos de los Espacios se ejecutan con programa y argumentos separados, nunca concatenando en una shell.
- Todo es local: nada de red salvo descargar el modelo de embeddings la primera vez.
- Antes de dar una tarea por terminada: fmt, clippy y tests en verde (y `scripts/ci.ps1` completa antes de cada merge).
- Al empezar cada fase o rama nueva, lee FEEDBACK.md. Resuelve los puntos pendientes que encajen con el trabajo actual o con poco coste, y del resto di cuáles dejas para más adelante y por qué. Marca como resueltos los que hagas.
- Antes de dar por buena una pantalla, revisa alineación, espaciado y jerarquía visual siguiendo el sistema de diseño.