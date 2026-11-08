# CLAUDE.md — Reglas del proyecto Sift

Sift es un lanzador y buscador semántico de escritorio para Windows (Tauri 2 + Rust + React/TS).
El plan completo, con fases y criterios de "hecho", está en [PLAN.md](PLAN.md). Léelo antes de empezar una tarea.

## Entorno de Darío
- Windows 10 Pro: donde el plan dice Mica, usar **Acrylic** (Mica es solo Windows 11).
- Editor: VS Code. Push: lo hace Darío con GitHub Desktop. **Claude nunca hace `git push`.**

## Cómo trabajamos
- Una rama y un PR por tarea o grupo pequeño de tareas; nunca todo de golpe.
- Cada PR con tests y la CI en verde antes de fusionar.
- Commits pequeños con [Conventional Commits](https://www.conventionalcommits.org/es/): `feat:`, `fix:`, `chore:`, `docs:`, `ci:`, `refactor:`, `test:`…
- Claude Code explica cada decisión importante; Darío revisa y entiende el código antes de aceptarlo. Explicar el *porqué*, no solo el *qué*.
- Decisiones de arquitectura nuevas → un ADR corto en `docs/adr/` (plantilla: `docs/adr/0001-*.md`).
- Si aparece una idea nueva a mitad de una fase → al backlog de PLAN.md, salvo que bloquee la fase actual.
- Al acabar cada fase: marcar casillas en PLAN.md, actualizar el README y apuntar qué se aprendió.

## Estructura
- `apps/desktop/` — app Tauri (`src/` UI React + TS, `src-tauri/` arranque y comandos IPC).
- `crates/*` — núcleo en Rust: `core`, `storage`, `extract`, `indexer`, `embeddings`, `search`, `spaces`.
- `eval/` — consultas de evaluación. `docs/adr/` — decisiones de arquitectura.

## Comandos
Desde la raíz del repo:
- Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- UI (desde `apps/desktop/`): `npm run lint`, `npm run format`, `npm run typecheck`, `npm test`
- App en desarrollo: `npm run tauri dev` (desde `apps/desktop/`)

## Reglas de código
- Errores: `thiserror` en los crates de librería, `anyhow` solo en la app. Nada de `unwrap()` fuera de tests.
- Logs con `tracing`. Sin telemetría.
- Los comandos de los Espacios se ejecutan con programa y argumentos separados, nunca concatenando en una shell.
- Todo es local: nada de red salvo descargar el modelo de embeddings la primera vez.
- Antes de dar una tarea por terminada: fmt, clippy y tests en verde.
