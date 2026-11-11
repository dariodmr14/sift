# Sistema de diseño «Vidrio»

Sift tiene que parecer parte de Windows: Acrylic, Segoe UI y **tu** color de acento. Esta guía es la referencia para cualquier pantalla nueva. Los tokens viven en [`apps/desktop/src/index.css`](../apps/desktop/src/index.css). La decisión y las alternativas descartadas están en el [ADR 0005](adr/0005-direccion-visual-vidrio.md).

## Reglas

1. **Solo tokens.** Ningún componente escribe un color a mano (`#…`, `rgb(…)`, `text-red-400`…). El test [`design-tokens.test.ts`](../apps/desktop/src/design-tokens.test.ts) falla si alguno lo hace.
2. **Windows manda.** El tema (claro/oscuro) y el acento los lee Rust del registro (`appearance.rs`) y la UI los aplica con `applyAppearance()`. No se elige un acento propio.
3. **Selección = píldora.** Toda lista seleccionable usa `.sift-option`: fondo `selected` y la píldora de 3 px del acento a la izquierda. Es como lo hacen las listas de Windows.
4. **Movimiento corto y opcional.** Solo opacidad y escala, nunca propiedades que obliguen a recalcular el diseño. Con «reducir movimiento» (en Windows: *Mostrar animaciones* desactivado) no hay animación.

## Materiales (el vidrio)

**Un único vidrio para toda la app** (`apps/desktop/src-tauri/src/materials.rs`): Acrylic de Windows con el mismo tinte (`tint()`, alfa 200/255) en la barra, en Ajustes y en cualquier ventana nueva. Encima, la UI solo pone `--surface` casi transparente.

| Superficie | Vidrio | Sin vidrio |
|---|---|---|
| Ventanas (barra, Ajustes) | Acrylic con el tinte común | Opaco |
| Diálogos y barras fijas dentro de una ventana | `.sift-glass` o `bg-glass backdrop-blur-xl`: el mismo tinte compuesto con `--surface` (≈ 83 %) | Opaco |

Acrylic va a tirones al **arrastrar o redimensionar** una ventana en Windows 10 v1903+ y Windows 11 22000. Por eso, mientras Ajustes se mueve, pasa a *blur* (`materials::on_moved`) con un tinte algo más opaco (`moving_tint()`, ajustado midiendo capturas sobre un fondo de franjas: luminancia 56,6 frente a 55,7 del Acrylic), y 200 ms después de soltarla vuelve el Acrylic. En reposo, que es lo que se ve, es idéntico a la barra.

Las ventanas se crean ocultas y se muestran cuando su UI ya ha pintado (`settings_ready`), reaplicando el material ya visibles: en Windows 10, el Acrylic aplicado a una ventana aún no mostrada queda mucho más transparente hasta que se mueve.

**Lo que va directamente sobre la ventana no lleva fondo propio** (barra de título, navegación, pies fijos): así se ve el vidrio de la ventana. Un pie fijo se saca de la zona con scroll en vez de pintarle un fondo. `bg-glass` es solo para lo que flota *encima de contenido* (diálogos, la tarjeta que se arrastra).

Vidrio por defecto; **«Desactivar vidrio»** (Ajustes → General) deja toda la app opaca. La UI sabe qué material tiene su ventana (`get_window_material`) y lo marca en `<html data-glass>`. Con «Efectos de transparencia» desactivados en Windows (`prefers-reduced-transparency`), todo es opaco.

## Tokens

### Color (utilidades de Tailwind)

| Token | Utilidad | Oscuro | Claro | Uso |
|---|---|---|---|---|
| `--surface` | `bg-surface` | `rgb(32 32 36)` · con vidrio 22 % | `rgb(243 243 245)` · con vidrio 28 % | Fondo de ventana |
| `--fg` | `text-fg` | `rgb(243 244 246)` | `rgb(27 27 31)` | Texto principal |
| `--muted` | `text-muted` | `rgb(166 173 187)` | `rgb(92 99 112)` | Texto secundario, ayudas |
| `--line` | `border-line` | blanco 8 % | negro 8 % | Separadores y bordes |
| `--hover` | `bg-hover` | blanco 5 % | negro 4 % | Fondo al pasar el ratón |
| `--selected` | `bg-selected` | blanco 8 % | negro 6 % | Elemento seleccionado |
| `--field` | `bg-field` | blanco 6 % | blanco 70 % | Fondo de campos |
| `--accent` | `bg-accent`, `text-accent`… | Acento «Light 2» de Windows | Acento «Dark 1» de Windows | Selección, botón principal, foco |
| `--accent-fg` | `text-accent-fg` | negro o blanco, el de más contraste | | Texto sobre el acento |
| `--danger` | `text-danger` | `rgb(255 153 164)` | `rgb(196 43 28)` | Errores |
| `--success` | `text-success` | `rgb(108 203 95)` | `rgb(15 123 15)` | Confirmaciones |
| `--glass` | `bg-glass` | `rgb(32 32 36 / 0.83)` | `rgb(243 243 245 / 0.84)` | Diálogos y barras fijas (con `backdrop-blur`); el mismo aspecto que la ventana |
| | `bg-caption-close`, `text-caption-close-fg` | `rgb(196 43 28)` / blanco | igual | Botón de cerrar de la barra de título al pasar el ratón (el rojo de Windows) |
| | `bg-scrim` | negro 50 % | negro 50 % | Fondo tras un diálogo |

Sin acento legible en el registro, se usa el azul de Windows (`#60CDFF` en oscuro, `#005FB8` en claro).

### Forma

| Token | Utilidad | Valor | Uso |
|---|---|---|---|
| `--radius-window` | `rounded-window` | 8 px | Ventanas y diálogos |
| `--radius-item` | `rounded-item` | 4 px | Filas, botones, campos, chips |
| `--shadow-window` | `shadow-window` | `0 16px 48px` negro 45 % + filo de luz de 1 px | Diálogos flotantes |

En Windows 10 la ventana de la barra es rectangular: Windows no redondea las ventanas (en Windows 11 sí), y redondearla por CSS dejaría el Acrylic asomando en las esquinas. Los radios se aplican dentro.

### Tipografía

| Rol | Familia | Tamaño |
|---|---|---|
| Interfaz | `Segoe UI Variable Text`, `Segoe UI` (la del sistema, sin carga) | Búsqueda 20 px (`text-xl`), título 15 px, detalle 12 px (`text-xs`) |
| Código y comandos | `Cascadia Mono`, `Consolas` (`font-mono`) | 13 px |

### Movimiento

| Token | Valor | Uso |
|---|---|---|
| `--dur-enter` | 120 ms | Entrada de la barra |
| `--dur-exit` | 80 ms | Salida de la barra (Esc o tras abrir un Espacio) |
| `--ease-enter` | `cubic-bezier(0, 0, 0, 1)` | La curva "decelerate" de Fluent |

La entrada va de opacidad 0,5 y escala 0,98 a su estado normal. **Empieza en 0,5, no en 0**, para que el primer fotograma ya enseñe el contenido: la animación no retrasa la apertura percibida (objetivo < 100 ms; ver métricas en el README).

Cuando la barra se oculta (por cualquier vía), Rust emite `LauncherHidden` y la UI se reinicia con la ventana invisible. Así, al volver a abrirse, nunca se ve un fotograma con la búsqueda anterior.

## Componentes

- **`.sift-option`** (`index.css`): opción de lista con la selección de Windows. Se marca con `aria-selected="true"` o `aria-current`.
- **`Button`, `TextInput`, `Field`, `Modal`, `ErrorText`** (`src/settings/ui.tsx`): controles de los ajustes, ya con los tokens. Variantes de botón: `primary` (acento), `secondary`, `ghost`, `danger`.
- **`TitleBar`** (`src/settings/TitleBar.tsx`): barra de título propia de las ventanas de app (la nativa de Windows 10 es negra y no admite vidrio). Zona de arrastre con `data-tauri-drag-region` (el arrastre lo hace Windows: conserva el ajuste a los bordes, Win+flechas y el doble clic para maximizar) y botones de 46 × 32 px con los glifos de Windows. El de cerrar usa `bg-caption-close`.
- **`.sift-glass`** (`index.css`): superficie flotante de vidrio para diálogos (`Modal` ya la usa).
- **Reordenar arrastrando**: `SortableList` y `useSortableItem` (`src/settings/Sortable.tsx`), sobre [dnd-kit](https://dndkit.com). La tarjeta sigue al puntero, se levanta (`bg-glass` + `shadow-window`) y las demás se apartan con una transición de 180 ms solo de `transform` (ninguna con «reducir movimiento»). Mientras se arrastra, `<html class="sift-sorting">` impide seleccionar texto. El asa lleva `select-none touch-none` y las imágenes `draggable={false}`. Siempre con la alternativa de teclado Alt+↑/↓ y anuncios en español para lectores de pantalla.
- **«Por significado»** (`App.tsx`): los resultados que llegan solo por la búsqueda inteligente empiezan su subtítulo con esa marca en `text-accent`. No tienen palabras que resaltar: se enseña el principio de su fragmento.
- **`SpaceIcon`** (`src/components/SpaceIcon.tsx`): icono de un Espacio. Los SVG se pintan con `<img>` o máscara CSS, nunca como HTML.

## Textos

Números y tiempos se escriben como los diría una persona, siempre con `src/format.ts`: `count` («1.240»), `files` («1 archivo»), `duration` («5 min 35 s», nunca «335 s») y `since` («hace 3 h», «ayer»). Los milisegundos solo aparecen en Ajustes → Estado, que es diagnóstico.

## Cómo añadir algo nuevo

1. Busca un token que encaje. Si no lo hay, añádelo en `index.css` con su valor oscuro **y** claro, expónlo en `@theme inline` y apúntalo en la tabla de arriba.
2. Si es una lista, usa `.sift-option`. Si es un control de formulario, usa los de `ui.tsx`.
3. Si se anima, usa los tokens de movimiento y comprueba que con «reducir movimiento» no se mueve.
4. `npm test` comprueba que no se ha colado ningún color literal.
