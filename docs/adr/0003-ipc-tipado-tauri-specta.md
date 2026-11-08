# ADR 0003 — IPC tipado con tauri-specta

- **Estado:** aceptada
- **Fecha:** 2026-11-08

## Contexto

En la Fase 1 aparecen los primeros comandos IPC (UI → Rust) y eventos (Rust → UI). Con `invoke("nombre", args)` a pelo, el nombre y la forma de los datos son texto libre: si cambias una firma en Rust, la UI falla en ejecución, no al compilar.

## Decisión

Usar **tauri-specta** (`=2.0.0-rc.26`, con `specta =2.0.0-rc.26` y `specta-typescript =0.0.13`):

- Los comandos llevan `#[tauri::command]` + `#[specta::specta]`; los tipos, `#[derive(specta::Type)]`.
- Se genera `apps/desktop/src/bindings.ts` con `commands.*` y `events.*` tipados. Los `Result<T, E>` de Rust llegan como `{ status: "ok", data } | { status: "error", error }`, y TypeScript obliga a tratar el error.
- En desarrollo el archivo se regenera en cada arranque. Además, el test `bindings_are_up_to_date` falla en la CI si alguien cambia un comando y no lo regenera (`SIFT_UPDATE_BINDINGS=1 cargo test -p sift-desktop` lo regenera).

## Por qué, a pesar del riesgo

- tauri-specta para Tauri 2 sigue en **RC** y exige versiones **exactas** de specta. Las fijamos con `=` en el `Cargo.toml` del workspace para que un `cargo update` no las mueva.
- Alternativas: escribir los tipos de TS a mano (se desincronizan) o `ts-rs` (genera tipos, pero no los envoltorios de comandos y eventos). tauri-specta es lo único que cubre las dos partes.
- **Plan B:** si el proyecto se abandona, `bindings.ts` es TypeScript normal que funciona sin la librería, así que se puede congelar y mantener a mano.

## Consecuencias

- Cada comando nuevo se añade a `collect_commands!` en `commands.rs`, y se regenera `bindings.ts`.
- `bindings.ts` se excluye de ESLint y Prettier, porque es código generado.
