# Cobertura de tests

Qué está probado y qué no, medido en vez de supuesto (auditoría de la Fase 7, M2). Última medida: 2026-11-18.

## Interfaz (Vitest)

```bash
cd apps/desktop
npm run coverage        # resumen en la terminal; informe navegable en apps/desktop/coverage/index.html
```

Cuenta todo `src/` salvo los tests y `bindings.ts` (generado).

| | Antes (2026-11-18) | Después de este PR |
|---|---|---|
| Líneas | 47,75 % (584 de 1.223) | **62,87 %** (769 de 1.223) |
| Ramas | 45,26 % | **57,52 %** |
| Funciones | 38,81 % | **54,68 %** |

**Bien cubierto (90–100 %):** toda la lógica de la barra (`launcher/`: mezcla de resultados, filtros, anuncios, vista previa, resumen de un Espacio), `format.ts`, `draft.ts`, `folders.ts`, `iconLibrary.ts`, `pathSuggest.ts`; la barra en sí (`App.tsx`, ~73 %) con sus tests de teclado.

**Huecos, por importancia:**

| Archivo | Antes | Por qué importa | Estado |
|---|---|---|---|
| `settings/SpaceEditor.tsx` | 0 % | Guardar y borrar Espacios: un fallo pierde datos | **67 %** en este PR: guardar (botón y Ctrl+S), descartar, crear, error de Rust a la vista, borrar con confirmación |
| `settings/ActionEditor.tsx` | 0 % | Los comandos van con programa y argumentos por separado (seguridad) | **36 %**: el comando (programa, argumentos y carpeta por separado, nunca juntos) y la web; faltan esperas, apps y «Probar» |
| `settings/PathField.tsx` | 0 % | Teclado de las sugerencias de rutas | **98 %**: sugerencias con dos letras, rutas completas sin sugerir, ↓/Enter/clic/Esc, «Elegir…» |
| `settings/FoldersSection.tsx` | 0 % | Añadir y quitar carpetas | **68 %**: lista, vacía, carpeta que no existe, quitar con confirmación, añadir |
| `settings/SpacesSection.tsx`, `Settings.tsx` | 0 % | Navegación de Ajustes: la cubre la prueba de extremo a extremo (`scripts/e2e.mjs`, `scripts/capturas-final.mjs`) | pendiente |
| `settings/IconPicker.tsx`, `AppPicker.tsx`, `PlacementEditor.tsx`, `WaitEditor.tsx` | 0 % | Editores secundarios; su lógica pura (`zones.ts`, `iconLibrary.ts`) sí está probada | pendiente |
| `settings/TitleBar.tsx`, `main.tsx`, `appearance.ts` | 0 % | Arranque y ventana: solo se pueden probar con la app de verdad (e2e) | no merece la pena en unitarios |

## Núcleo en Rust

**Sin medir.** `cargo-llvm-cov` necesita instalar dos cosas fuera del proyecto, y eso lo decide Darío (no se instala nada fuera del repo sin permiso):

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --workspace --summary-only
```

Mientras tanto, revisado a mano qué módulos **no tienen ningún test propio** (por tamaño):

| Módulo | Líneas | Comentario |
|---|---|---|
| `crates/search/src/index.rs` | 536 | El índice de Tantivy. **Cubierto en este PR** con 8 tests directos: encontrar por contenido y por nombre, borrar, rehacer un archivo, moverlo, `fetch`, archivos solo por nombre, reabrir del disco y vaciar |
| `apps/desktop/src-tauri/src/indexing/sync.rs` | 370 | La revisión de carpetas del hilo de indexado. Necesita la app (`AppHandle`); la lógica de qué cambia vive en `sift_indexer`, que sí está probada. La cubre el e2e |
| `apps/desktop/src-tauri/src/hotkey.rs` | 367 | El atajo global y el hook de teclado de Windows: solo se puede probar a mano |
| `apps/desktop/src-tauri/src/launcher.rs` | 221 | Mostrar y ocultar la barra: e2e |
| `crates/spaces/src/model.rs` | 170 | Tipos de los Espacios; su validación (`validate.rs`) y su ejecución (`run.rs`) sí están probadas |
| `apps/desktop/src-tauri/src/settings_window.rs` | 125 | La ventana de Ajustes: e2e |

**El hueco crítico de Rust** era `index.rs`, porque es lo que hace que lo que borras o mueves deje de salir en la búsqueda. Ya tiene sus tests. Lo siguiente, cuando se pueda medir, es mirar las ramas de error de `sift_indexer` y `sift_extract`.

## Notas de los tests nuevos

- Los tests de componentes de Ajustes necesitan dos apaños para jsdom: `window.matchMedia`, que usa `Sortable.tsx` para «reducir movimiento», y `listMonitors`, que pide `PlacementEditor`. Si más tests los necesitan, irían a un archivo de preparación común.
- `FoldersSection.ui.test.tsx` lleva `.ui` porque ya existe `FoldersSection.test.ts` (lógica pura). Con el mismo nombre, TypeScript y ESLint se confunden.
- **Accesibilidad:** `Field` mete el texto de ayuda dentro de la `<label>`, así que el nombre accesible del campo incluye toda la ayuda y un lector de pantalla la lee entera. Lo mejor es pasar la ayuda con `aria-describedby`. Está en `docs/pendiente-decidir.md`.
