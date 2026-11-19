# Cobertura de tests

Qué está probado y qué no, medido en vez de supuesto (auditoría de la Fase 7, M2). Última medida: 2026-11-18 (interfaz y Rust).

## Interfaz (Vitest)

```bash
cd apps/desktop
npm run coverage        # resumen en la terminal; informe navegable en apps/desktop/coverage/index.html
```

Cuenta todo `src/` salvo los tests y `bindings.ts` (generado).

| | Antes (2026-11-18) | PR #29 | PR de la segunda tanda |
|---|---|---|---|
| Líneas | 47,75 % (584 de 1.223) | 62,87 % | **75,02 %** (919 de 1.225) |
| Ramas | 45,26 % | 57,52 % | **71,75 %** |
| Funciones | 38,81 % | 54,68 % | **69,78 %** |

**Bien cubierto (90–100 %):** toda la lógica de la barra (`launcher/`: mezcla de resultados, filtros, anuncios, vista previa, resumen de un Espacio), `format.ts`, `draft.ts`, `folders.ts`, `iconLibrary.ts`, `pathSuggest.ts`; la barra en sí (`App.tsx`, ~73 %) con sus tests de teclado.

**Huecos, por importancia:**

| Archivo | Antes | Por qué importa | Estado |
|---|---|---|---|
| `settings/SpaceEditor.tsx` | 0 % | Guardar y borrar Espacios: un fallo pierde datos | **67 %** en este PR: guardar (botón y Ctrl+S), descartar, crear, error de Rust a la vista, borrar con confirmación |
| `settings/ActionEditor.tsx` | 0 % | Los comandos van con programa y argumentos por separado (seguridad) | **82 %**: el comando (programa, argumentos y carpeta por separado, nunca juntos), la web, la app y «Probar» (con su resultado, su error y las esperas) |
| `settings/PathField.tsx` | 0 % | Teclado de las sugerencias de rutas | **98 %**: sugerencias con dos letras, rutas completas sin sugerir, ↓/Enter/clic/Esc, «Elegir…» |
| `settings/FoldersSection.tsx` | 0 % | Añadir y quitar carpetas | **68 %**: lista, vacía, carpeta que no existe, quitar con confirmación, añadir |
| `settings/SpacesSection.tsx` | 0 % | Lista de Espacios, reordenar, importar y exportar | **77 %**: lista, nuevo, vacío, Alt+↑/↓ (guarda el orden), exportar e importar (también cancelado y con error); falta el arrastre con ratón |
| `settings/Settings.tsx` | 0 % | Navegación de Ajustes: la cubre la prueba de extremo a extremo (`scripts/e2e.mjs`, `scripts/capturas-final.mjs`) | pendiente |
| `settings/IconPicker.tsx`, `AppPicker.tsx`, `WaitEditor.tsx` | 0 % | Editores secundarios | **100 %, 90 % y 100 %**: los cinco tipos de icono (la librería, cargada de verdad), elegir app y cada tipo de espera |
| `settings/PlacementEditor.tsx` | 0 % | Elegir zona; su lógica (`zones.ts`) sí está probada | pendiente |
| `settings/TitleBar.tsx`, `main.tsx`, `appearance.ts` | 0 % | Arranque y ventana: solo se pueden probar con la app de verdad (e2e) | no merece la pena en unitarios |

## Núcleo en Rust

Medido con `cargo-llvm-cov` (instalado con permiso de Darío el 2026-11-18, a nivel de usuario: `rustup component add llvm-tools-preview` y `cargo install cargo-llvm-cov --locked`).

```bash
cargo llvm-cov --workspace --summary-only   # por archivo, en la terminal
cargo llvm-cov --workspace --html           # informe navegable en target/llvm-cov/html
```

**Total: 70,82 % de las líneas.** La cifra junta dos mundos muy distintos:

| Parte | Líneas | Comentario |
|---|---|---|
| **Crates del núcleo** (`crates/*`) | 85–100 % por archivo | Toda la lógica que decide: búsqueda (`index.rs` 98 %, `engine.rs` 98 %, `fusion.rs` 100 %, `snippet.rs` 99,6 %), indexado (`sync.rs` 98 %, `chunk.rs` 99 %), extracción (`pdf.rs` 96 %, `office.rs` 96 %), Espacios (`run.rs` 98 %, `validate.rs` 97 %, `placement.rs` 90 %) y almacenamiento (`db.rs` 96 %, `index.rs` 96 %). Única excepción: `spaces/model.rs` (0 %), que solo declara tipos |
| **App** (`apps/desktop/src-tauri`) | 0–100 % | Pegamento con Tauri y Windows. Bien: `watcher.rs` 93 %, `waits.rs` 88 %, `load.rs` 85 %, `dev.rs` 84 %, `latency.rs` 100 %. A 0 %: `hotkey.rs`, `launcher.rs`, `tray.rs`, `settings_window.rs`, `smart.rs`, `logging.rs` y el hilo del indexado (`indexing/sync.rs` 0 %, `vectorize.rs` 2 %, `worker.rs` 13 %) |

**Por qué la app sale baja y qué hacer:** casi todo lo que está a 0 % necesita la app de Tauri en marcha (`AppHandle`) o Windows de verdad (atajo global, bandeja, ventanas). Lo cubren la prueba de extremo a extremo (`scripts/e2e.mjs`) y la comprobación en pantalla. Probarlo en unitarios obligaría a montar un runtime de Tauri falso: mucho código para poco.

**Huecos críticos y qué se ha hecho:**

| Hueco | Por qué importa | Estado |
|---|---|---|
| La vista previa (`search_commands.rs`, 6,9 %) | Decide la página del PDF, la fuente fija del código y si se conservan los saltos de línea | **Cubierto** (este PR): la lógica sale del comando a `build_preview` con 4 tests (PDF con página y líneas unidas, Markdown con saltos y sin página, código en fuente fija, archivo sin texto) |
| Aprobación de comandos (`spaces_commands.rs`, 18,6 %) | Seguridad: nada se ejecuta sin confirmar | La decisión vive en `sift_spaces` (`pending_approvals`, `approval_message`, `display_command`), cubierta al 98 %; en la app solo queda el diálogo de Windows, que no se puede probar sin pantalla |
| Hilo del indexado (`indexing/*`) | Que lo que guardas aparezca y lo que borras desaparezca | La lógica (qué cambió, qué leer, cómo trocear) está en `sift_indexer` (98 %); el hilo solo la orquesta. La cubre el e2e. Si un día da problemas, el siguiente paso es sacar la orquestación a `sift_indexer` con un `Worker` sin Tauri |
| `spaces/model.rs` (0 %) | — | Solo tipos y `serde`; lo ejercitan los tests de `export.rs`, `validate.rs` y `run.rs` |

## Notas de los tests nuevos

- Los tests de componentes de Ajustes necesitan dos apaños para jsdom: `window.matchMedia`, que usa `Sortable.tsx` para «reducir movimiento», y `listMonitors`, que pide `PlacementEditor`. Si más tests los necesitan, irían a un archivo de preparación común.
- `FoldersSection.ui.test.tsx` lleva `.ui` porque ya existe `FoldersSection.test.ts` (lógica pura). Con el mismo nombre, TypeScript y ESLint se confunden.
- **Accesibilidad:** `Field` mete el texto de ayuda dentro de la `<label>`, así que el nombre accesible del campo incluye toda la ayuda y un lector de pantalla la lee entera. Lo mejor es pasar la ayuda con `aria-describedby`. Está en `docs/proceso/pendiente-decidir.md`.
