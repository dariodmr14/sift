# Sistema de diseño «Vidrio»

Sift tiene que parecer parte de Windows: Acrylic, Segoe UI y **tu** color de acento. Esta guía es la referencia para cualquier pantalla nueva. Los tokens viven en [`apps/desktop/src/index.css`](../apps/desktop/src/index.css). La decisión y las alternativas descartadas están en el [ADR 0005](adr/0005-direccion-visual-vidrio.md).

## Reglas

1. **Solo tokens.** Ningún componente escribe un color a mano (`#…`, `rgb(…)`, `text-red-400`…). El test [`design-tokens.test.ts`](../apps/desktop/src/design-tokens.test.ts) falla si alguno lo hace.
2. **Windows manda.** El tema (claro/oscuro) y el acento los lee Rust del registro (`appearance.rs`) y la UI los aplica con `applyAppearance()`. No se elige un acento propio.
3. **Selección = píldora.** Toda lista seleccionable usa `.sift-option`: fondo `selected` y la píldora de 3 px del acento a la izquierda. Es como lo hacen las listas de Windows.
4. **Movimiento corto y opcional.** Solo opacidad y escala, nunca propiedades que obliguen a recalcular el diseño. Con «reducir movimiento» (en Windows: *Mostrar animaciones* desactivado) no hay animación.

## Materiales (el vidrio)

Como en Fluent, cada tipo de superficie tiene su material de Windows (`apps/desktop/src-tauri/src/materials.rs`):

| Superficie | Material | Alternativa |
|---|---|---|
| Barra (flotante, nunca se arrastra) | Acrylic | Opaco |
| Ventanas de app (Ajustes) | Mica (Windows 11) | Blur con tinte (Windows 10), luego opaco |
| Diálogos y barras fijas dentro de una ventana | Vidrio por CSS: `.sift-glass` o `bg-glass backdrop-blur-xl` | Opaco |

Acrylic no se usa en ventanas que se arrastran o redimensionan: en Windows 10 v1903+ y Windows 11 22000 va a tirones. La UI sabe qué material tiene su ventana (`get_window_material`) y lo marca en `<html data-glass>`. Con «Efectos de transparencia» desactivados en Windows (`prefers-reduced-transparency`), todo es opaco.

## Tokens

### Color (utilidades de Tailwind)

| Token | Utilidad | Oscuro | Claro | Uso |
|---|---|---|---|---|
| `--surface` | `bg-surface` | `rgb(32 32 36)` · con Acrylic 22 % | `rgb(243 243 245)` · con Acrylic 28 % | Fondo de ventana |
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
| `--glass` | `bg-glass` | `rgb(44 44 50 / 0.72)` | `rgb(250 250 252 / 0.74)` | Diálogos y barras fijas (con `backdrop-blur`) |
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
- **`SpaceIcon`** (`src/components/SpaceIcon.tsx`): icono de un Espacio. Los SVG se pintan con `<img>` o máscara CSS, nunca como HTML.

## Cómo añadir algo nuevo

1. Busca un token que encaje. Si no lo hay, añádelo en `index.css` con su valor oscuro **y** claro, expónlo en `@theme inline` y apúntalo en la tabla de arriba.
2. Si es una lista, usa `.sift-option`. Si es un control de formulario, usa los de `ui.tsx`.
3. Si se anima, usa los tokens de movimiento y comprueba que con «reducir movimiento» no se mueve.
4. `npm test` comprueba que no se ha colado ningún color literal.
