# Sistema de diseño «Vidrio»

Sift tiene que parecer parte de Windows: Acrylic, Segoe UI y **tu** color de acento. Esta guía es la referencia para cualquier pantalla nueva. Los tokens viven en [`apps/desktop/src/index.css`](../apps/desktop/src/index.css). La decisión y las alternativas descartadas están en el [ADR 0005](adr/0005-direccion-visual-vidrio.md).

## Reglas

1. **Solo tokens.** Ningún componente escribe un color a mano (`#…`, `rgb(…)`, `text-red-400`…). El test [`design-tokens.test.ts`](../apps/desktop/src/design-tokens.test.ts) falla si alguno lo hace.
2. **Windows manda.** El tema (claro/oscuro) y el acento los lee Rust del registro (`appearance.rs`) y la UI los aplica con `applyAppearance()`. No se elige un acento propio. El tema sí se puede fijar en Ajustes → General → Apariencia («Como Windows», por defecto, «Claro» u «Oscuro»); al cambiarlo, Rust emite `AppearanceChanged` y todas las ventanas se repintan al momento ([ADR 0022](adr/0022-vidrio-legible-y-tema.md)).
3. **Selección = píldora.** Toda lista seleccionable usa `.sift-option`: fondo `selected` y la píldora de 3 px del acento a la izquierda. Es como lo hacen las listas de Windows.
4. **Movimiento corto y opcional.** Solo opacidad y escala, nunca propiedades que obliguen a recalcular el diseño. Con «reducir movimiento» (en Windows: *Mostrar animaciones* desactivado) no hay movimiento: como mucho, un fundido de opacidad breve.

## Materiales (el vidrio)

**Un único vidrio para toda la app** (`apps/desktop/src-tauri/src/materials.rs`): Acrylic de Windows con el mismo tinte (`tint()`, alfa 200/255) en la barra, en Ajustes y en cualquier ventana nueva. Encima, la UI pone `--surface` al 45 %: juntos cubren el **88 %** del escritorio, lo mínimo para que todo el texto llegue a 4,5:1 sobre el peor fondo (negro en claro, blanco en oscuro), también sobre una fila seleccionada. Lo comprueba `src/glass-contrast.test.ts` con los valores reales; si cambias un color o una opacidad, ese test dice si sigue leyéndose ([ADR 0022](adr/0022-vidrio-legible-y-tema.md)).

| Superficie | Vidrio | Sin vidrio |
|---|---|---|
| Ventanas (barra, Ajustes) | Acrylic con el tinte común | Opaco |
| Diálogos y barras fijas dentro de una ventana | `.sift-glass` o `bg-glass backdrop-blur-xl`: el mismo tinte compuesto con `--surface` (88 %) | Opaco |

Acrylic va a tirones al **arrastrar o redimensionar** una ventana en Windows 10 v1903+ y Windows 11 22000. Por eso, mientras Ajustes se mueve, pasa a *blur* (`materials::on_moved`) con un tinte algo más opaco (`moving_tint()`, ajustado midiendo capturas sobre un fondo de franjas: luminancia 56,6 frente a 55,7 del Acrylic), y 200 ms después de soltarla vuelve el Acrylic. En reposo, que es lo que se ve, es idéntico a la barra.

Las ventanas se crean ocultas y se muestran cuando su UI ya ha pintado (`settings_ready`), reaplicando el material ya visibles: en Windows 10, el Acrylic aplicado a una ventana aún no mostrada queda mucho más transparente hasta que se mueve.

**Lo que va directamente sobre la ventana no lleva fondo propio** (barra de título, navegación, pies fijos): así se ve el vidrio de la ventana. Un pie fijo se saca de la zona con scroll en vez de pintarle un fondo. `bg-glass` es solo para lo que flota *encima de contenido* (diálogos, la tarjeta que se arrastra).

Vidrio por defecto; **«Desactivar vidrio»** (Ajustes → General) deja toda la app opaca. La UI sabe qué material tiene su ventana (`get_window_material`) y lo marca en `<html data-glass>`. Con «Efectos de transparencia» desactivados en Windows (`prefers-reduced-transparency`), todo es opaco. **Donde Windows no puede pintar el vidrio** (Sandbox, escritorio remoto, sin aceleración gráfica, ahorro de batería), Sift no lo pide y va opaco solo (`glass_support.rs`); Ajustes lo explica debajo de «Desactivar vidrio».

## Tokens

### Color (utilidades de Tailwind)

| Token | Utilidad | Oscuro | Claro | Uso |
|---|---|---|---|---|
| `--surface` | `bg-surface` | `rgb(32 32 36)` · con vidrio 45 % | `rgb(243 243 245)` · con vidrio 45 % | Fondo de ventana |
| `--fg` | `text-fg` | `rgb(243 244 246)` | `rgb(27 27 31)` | Texto principal |
| `--muted` | `text-muted` | `rgb(186 192 204)` | `rgb(74 80 92)` | Texto secundario, ayudas |
| `--line` | `border-line` | blanco 8 % | negro 8 % | Separadores y bordes |
| `--line-strong` | `border-line-strong` | blanco 36 % | negro 44 % | Trazo inferior de los campos: 3:1 frente al fondo (WCAG 1.4.11) |
| `--hover` | `bg-hover` | blanco 5 % | negro 4 % | Fondo al pasar el ratón |
| `--selected` | `bg-selected` | blanco 8 % | negro 6 % | Elemento seleccionado |
| `--field` | `bg-field` | blanco 6 % | blanco 70 % | Fondo de campos |
| `--accent` | `bg-accent` | Acento «Light 2» de Windows | Acento «Dark 1» de Windows | Relleno: botón principal, casillas |
| `--accent-ink` | `text-accent-ink`, `ring-accent-ink`, `border-accent-ink`, `bg-accent-ink` | el acento, aclarado si hace falta | el acento, oscurecido si hace falta | El acento cuando es texto o trazo: foco, la píldora de selección, barras de progreso y la firma. Rust lo ajusta hasta 4,5:1 sobre el fondo opaco y sobre el peor vidrio (F9, ADR 0022) |
| `--accent-fg` | `text-accent-fg` | negro o blanco, el de más contraste | | Texto sobre el acento |
| `--danger` | `text-danger` | `rgb(255 168 178)` | `rgb(160 30 20)` | Errores |
| `--success` | `text-success` | `rgb(124 214 110)` | `rgb(10 96 10)` | Confirmaciones |
| `--glass` | `bg-glass` | `rgb(32 32 36 / 0.88)` | `rgb(243 243 245 / 0.88)` | Diálogos y barras fijas (con `backdrop-blur`); el mismo aspecto que la ventana |
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
| Interfaz | `Segoe UI Variable Text`, `Segoe UI` (la del sistema, sin carga) | La escala de Fluent: Caption 12 (`text-xs`), Body 14 (`text-sm`), Body Strong 14 en seminegrita, Subtitle 20 (`text-xl`, también la caja de búsqueda) |
| Iconos | `Segoe Fluent Icons`, `Segoe MDL2 Assets` (`Glyph`) | 16 px, o 12 dentro de texto pequeño |
| Código y comandos | `Cascadia Mono`, `Consolas` (`font-mono`) | 13 px |

Solo dos pesos: normal (400) y seminegrita (600). Cabeceras con mayúscula inicial, nunca en MAYÚSCULAS con espaciado (como Configuración de Windows 11). Cifras tabulares en toda la app (`font-variant-numeric` en `:root`).

### Movimiento

| Token | Valor | Uso |
|---|---|---|
| `--dur-enter` | 120 ms | Entrada de la barra |
| `--dur-exit` | 80 ms | Salida de la barra (Esc o tras abrir un Espacio) |
| `--dur-state` | 150 ms | Cambios de estado: coger y soltar al arrastrar, pasar el ratón y pulsar (el "fast" de Fluent) |
| `--ease-enter` | `cubic-bezier(0, 0, 0, 1)` | La curva "decelerate" de Fluent |

La entrada va de opacidad 0,5 y escala 0,98 a su estado normal. **Empieza en 0,5, no en 0**, para que el primer fotograma ya enseñe el contenido: la animación no retrasa la apertura percibida (objetivo < 100 ms; ver métricas en el README).

Cuando la barra se oculta (por cualquier vía), Rust emite `LauncherHidden` y la UI se reinicia con la ventana invisible. Así, al volver a abrirse, nunca se ve un fotograma con la búsqueda anterior.

**Arrastre:** al coger una tarjeta, su fondo de vidrio y su sombra aparecen con una transición de `--dur-state`, y al soltarla se desvanecen igual. La transición se suma a la de dnd-kit en el estilo en línea (`composeTransition` en `settings/Sortable.tsx`), porque si no la de dnd-kit la pisa.

**Cambio de panel en la barra:** al pasar de vacía a resultados, a "sin resultados" o a los paneles de confirmar, en marcha y resumen, el contenido entra con opacidad y 4 px (`.sift-swap`, `--dur-enter`). No se anima al escribir, para que la lista se lea al instante, ni al abrir la barra.

**Pulsar:** los botones oscurecen un paso al pulsarlos, sin escalar, como en Fluent. Hover y pulsado cambian de color con `--dur-state`. Las filas de la barra no se animan: la selección con flechas es instantánea.

Con "reducir movimiento" no hay ninguna de estas transiciones.

## Componentes

- **`.sift-option`** (`index.css`): opción de lista con la selección de Windows. Se marca con `aria-selected="true"` o `aria-current`.
- **`Button`, `TextInput`, `Field`, `Modal`, `ErrorText`** (`src/settings/ui.tsx`): controles de los ajustes, ya con los tokens. Variantes de botón: `primary` (acento), `secondary`, `ghost`, `danger`.
- **`TitleBar`** (`src/settings/TitleBar.tsx`): barra de título propia de las ventanas de app (la nativa de Windows 10 es negra y no admite vidrio). Zona de arrastre con `data-tauri-drag-region` (el arrastre lo hace Windows: conserva el ajuste a los bordes, Win+flechas y el doble clic para maximizar) y botones de 46 × 32 px con los glifos de Windows. El de cerrar usa `bg-caption-close`.
- **`.sift-glass`** (`index.css`): superficie flotante de vidrio para diálogos (`Modal` ya la usa).
- **Reordenar arrastrando**: `SortableList` y `useSortableItem` (`src/settings/Sortable.tsx`), sobre [dnd-kit](https://dndkit.com). La tarjeta sigue al puntero, se levanta (`bg-glass` + `shadow-window`) y las demás se apartan con una transición de 180 ms solo de `transform` (ninguna con «reducir movimiento»). Mientras se arrastra, `<html class="sift-sorting">` impide seleccionar texto. El asa lleva `select-none touch-none` y las imágenes `draggable={false}`. Siempre con la alternativa de teclado Alt+↑/↓ y anuncios en español para lectores de pantalla.
- **La firma: «la marca de lo que encaja»** (`.sift-match`, `Highlighted` en `src/launcher/panels.tsx`): lo encontrado, subrayado con un trazo de 2 px de `--accent-ink`, sin caja de color. En una búsqueda exacta, las palabras; si el archivo llegó solo por significado (no contiene tus palabras), el pasaje entero que encaja, con la misma marca, y el subtítulo empieza por «Por significado». Ver [unslop-direccion.md](proceso/unslop-direccion.md).
- **Vista previa** (`PreviewPanel` en `src/launcher/PreviewPanel.tsx`): con un archivo seleccionado, **Tab** (o **→** con el cursor al final de la caja) cambia la lista por su vista previa, dentro de la misma barra; ↑/↓ cambian de resultado, **Esc** o **←** vuelven a la lista (Esc no cierra la barra) y Enter abre el archivo. La cabecera lleva icono, nombre (14 seminegrita), carpeta y el porqué en una línea (12, `text-muted`): «Contiene «x»», «Trata de lo mismo» o «Por el nombre», con «pág. X» en los PDF. Debajo, un fragmento de unas 10 líneas con la firma `.sift-match`; conserva los saltos de línea (salvo en los PDF, cuyo texto viene cortado por la maquetación) y el código va en `font-mono` (12). Sin texto: «Este archivo no tiene texto que enseñar.» El texto sale de SQLite (`preview_file`), nunca del archivo, y solo se pide con la vista previa abierta: no cuenta en la apertura de la barra ni en la búsqueda. El cambio de panel usa `.sift-swap`; pasar de un resultado a otro no se anima. **Con un Espacio**, la vista previa enseña sus acciones numeradas en orden (`spaceSteps` en `src/launcher/spacePreview.ts`): qué abre, qué espera, dónde coloca cada ventana y qué hace si una espera no llega; Enter lo abre. **Botón ←** a la izquierda de la cabecera (32 px, `rounded-item`, `text-muted`), para volver a la lista con el ratón.
- **Teclado y foco en la barra**: las teclas se escuchan en toda la ventana y el foco vuelve siempre a la caja; Tab nunca lo saca de la barra (saldría de la ventana y Windows la ocultaría). Un clic en la barra no quita el foco a la caja, salvo en el texto que se puede seleccionar (`data-selectable`).
- **`Glyph`** (`src/components/Glyph.tsx`): iconos de la interfaz con la fuente de iconos de Windows; los códigos están comprobados en Segoe MDL2 Assets de Windows 10. Lucide solo para el icono que eliges en un Espacio.
- **`Keycap` y `Shortcut`** (`src/components/Keycap.tsx`): atajos como teclas pequeñas, solo en el pie de la barra y en la fila seleccionada. Los nombres de las teclas se ven como en un teclado español («Espacio», «Mayús», «Intro», «º») con `shortcutLabel` (`src/shortcut.ts`), el único sitio que traduce; por dentro se guardan en inglés («Alt+Space»). **Pistas del pie** (`Hint` en `App.tsx`): teclas + lo que hacen, con mayúscula inicial («Abrir», «Vista previa», «Volver»). Las que hacen algo son botones que se pueden pulsar con el ratón sin quitar el foco a la caja (fondo `hover:bg-hover`, 24 px de alto); las que solo informan («↑↓ Moverse») son texto.
- **`SpaceIcon`** (`src/components/SpaceIcon.tsx`): icono de un Espacio. Los SVG se pintan con `<img>` o máscara CSS, nunca como HTML.
- **Bienvenida (`src/settings/Welcome.tsx`)**: columna centrada de 560 px como máximo. Pasos numerados con un círculo de 24 px (`bg-hover`), título en 14 seminegrita y cuerpo en 14. El atajo se muestra con `Shortcut` (keycaps). Un solo botón principal por pantalla («Empezar»). En modo Ajustes, la misma guía aparece con la navegación lateral.
- **`Select`** (`src/settings/ui.tsx`): desplegable nativo de Windows con el aspecto de `TextInput` (`.sift-field` y `bg-hover`), así que sigue el tema por `color-scheme`. Sirve para elegir entre pocas opciones. El teclado es el del control nativo.
- **Campo de ruta con sugerencias** (`src/settings/PathField.tsx`; la regla de cuándo sugerir, en `src/settings/pathSuggest.ts`): mientras escribes un nombre de al menos dos letras, propone archivos y carpetas del índice que encajan. La lista es `.sift-glass` con `.sift-option`: ↑/↓ se mueven, Enter elige y Esc cierra la lista sin cerrar el editor. Las rutas completas («C:\…») no se sugieren, y «Elegir…» abre el diálogo de Windows para lo que no está indexado.
- **Icono** ([ADR 0017](adr/0017-icono.md)): «el tamiz». Son tres barras que se estrechan sobre el cuadrado oscuro de Vidrio, con la última en el acento por defecto (`#60CDFF`). En la bandeja va de un solo color: blanco o casi negro según el tema de la barra de tareas. Fuentes en `src-tauri/icons/source/`.
- **Introducción guiada** (`src/settings/Intro.tsx`, pasos y lógica en `introSteps.ts`): la primera vez ocupa la ventana de Ajustes sin navegación; se repite desde Primeros pasos («Ver la introducción otra vez»). Columna de 560 px: arriba, 5 segmentos de progreso (4 px, `bg-hover`; hechos y actual en `bg-accent`, el actual se llena con `.sift-fill`); «Paso N de 5» (12, `text-muted`) y el título (20 seminegrita, recibe el foco y se anuncia «Paso 2 de 5: Dónde buscar»); abajo, «Saltar la introducción» (fantasma) a la izquierda y «Atrás» / «Siguiente» (o «Empezar») a la derecha. Movimiento: cada paso entra con `.sift-step-next` o `.sift-step-back` (opacidad y 16 px en horizontal, `--dur-step` 200 ms, *decelerate*); dentro del paso, el orden es título → explicación (14, `text-fg`) → acción, y lo que va bajo el título entra escalonado con `.sift-stagger` (opacidad y 4 px, `--stagger` 70 ms entre uno y otro); encima de «Paso N de 5», una ilustración por paso (`IntroArt.tsx`, franja de 112 px con `border-line` y `bg-hover/40`, `aria-hidden`): se ve una vez al entrar, en menos de 1,6 s, y se queda en su último fotograma (teclas que se hunden y la barra que baja; carpetas que llegan y un contador; «factura» que se escribe y resultados que caen; una línea de «viaje» a «billetes de tren.pdf»; un Espacio que se abre en tres ventanas). Cada pieza dura `--dur-art` (350 ms) con su retraso en `--d`; solo opacidad y transformaciones; con «reducir movimiento», el último fotograma y el paso entra con un fundido de opacidad (`sift-fade`, `--dur-step`), sin moverse: así no aparece de golpe (en Windows Sandbox las animaciones de Windows vienen apagadas); cada logro (✓) con `.sift-pop` (escala 0,9 → 1, 150 ms); las teclas del atajo «se hunden» 1 px al detectar la pulsación (`.sift-pressed`). Con «reducir movimiento» no se mueve nada; solo queda ese fundido.
- **Tarjetas de estado** (`src/settings/StatusSection.tsx`, textos en `statusCards.ts`): Ajustes → Estado. Una frase arriba («Todo va bien.») y una rejilla de 2 columnas (1 en ventanas estrechas) con una tarjeta por cosa que importa a quien usa Sift: atajo, carpetas, búsqueda inteligente y tus datos. Cada tarjeta: `rounded-item`, `border-line`, `bg-hover/40`, `p-4`; icono de estado de 24 px (✓ `text-success` bien; sincronizar `text-accent-ink` en marcha; aviso `text-accent-ink`; aviso `text-danger` problema; guion `text-muted` apagado), título 14 seminegrita y detalle 12 `text-muted`. El icono lleva nombre para el lector de pantalla y el título ya dice lo mismo: el color nunca va solo. Lo técnico (motor del atajo, material, tiempos p50/p95, vigilancia, lector de PDF), en «Avanzado» plegado.
- **Miniaturas de zonas para colocar ventanas** (`src/settings/PlacementEditor.tsx`, datos en `src/settings/zones.ts`): radios en filas con su nombre a la izquierda (12, `text-muted`): General («No colocar», con la miniatura vacía y un guion, y toda la pantalla), Mitades, Tercios y Cuartos. Cada uno es una miniatura de 44 × 28 px con la zona en `bg-accent`. **Plegado** si ya hay una posición elegida: una línea, «Posición: Mitad derecha · Pantalla 2» (o «No colocar»; «· usar la ya abierta» si está marcada), con el botón «Cambiar»; desplegado, «Listo» lo vuelve a plegar. Debajo de las miniaturas, «Posición seleccionada: Mitad derecha» (14 seminegrita, `aria-live`). Con una zona, el desplegable «Pantalla» como en Configuración de Windows: el número que da Windows y el modelo («Pantalla 1 — DELL U2720Q (principal)», «Donde esté el ratón»), y a su lado «Identificar», que enseña 3 s el número en cada pantalla (`identify.html`, `.sift-identify`: número de 128 px sobre `--surface-solid` con borde de 4 px `--accent`). La casilla «Si ya está abierta, usar esa en vez de abrir otra» se ve siempre, también con «No colocar». Son botones con `role="radio"` y se recorren con Tab: no tienen flechas.
- **Editor de esperas** (`src/settings/WaitEditor.tsx`): el paso «Esperar a que…» de un Espacio. Un desplegable elige qué se espera (una web, un servicio, el fin del comando anterior o unos segundos) y muestra solo el campo que hace falta, con su ayuda en lenguaje normal. Solo las esperas de web y de servicio se pueden probar con «Probar» (lo decide `canCheckNow`).
- **Aviso de fallo de la ventana** (`src/components/ErrorBoundary.tsx`): si algo falla al pintar la barra o Ajustes, en vez de quedarse en blanco se ve «Algo ha fallado en esta ventana», «Tus datos están bien…» y el botón «Volver a cargar» (con el foco). En la barra, Esc la sigue ocultando (`role="alert"`, tokens de siempre).
- **Indicador de carga** (`src/components/SiftLoader.tsx`, estilos `.sift-loader` en `index.css`): las tres barras del tamiz, como el icono (13, 9 y 5 sobre 16, `--accent`), que se cierran hasta el 35 % y se vuelven a abrir por turnos (`scaleX`, `--dur-loader` 1,2 s). Dos tamaños: `sm` (16 px, junto a un texto) y `md` (32 px, encima del texto, para un panel vacío). `SiftLoader` es solo las barras, decorativas (`aria-hidden`): el texto de al lado dice qué pasa y se anuncia (`role="status"`). `SiftLoading` lleva su texto y, con `delay` (300 ms), no sale si lo que carga es rápido. Solo para lo que tarda de verdad: el primer indexado y las actualizaciones (Carpetas, introducción, tarjetas «en marcha» de Estado), la descarga de la búsqueda inteligente (introducción, bienvenida y General), una vista previa lenta, las listas que cargan en Ajustes, «Probar» mientras prueba y el panel «Abriendo…» (el paso en marcha, cada ventana que se coloca y «Colocando las ventanas…»). Con «reducir movimiento», las barras quietas y un pulso lento de opacidad.
- **Paneles «en marcha» y «resumen»** (`src/launcher/panels.tsx`): `RunningPanel` enseña un Espacio abriéndose, con lo ya abierto marcado con ✓, el paso en curso y cuántos faltan (`role="status"`). Un paso ya lanzado cuya ventana aún se coloca lleva la flecha `text-accent-ink` y «Eclipse abierto · colocando…»; al colocarse pasa a ✓, y si no se pudo, aviso `text-accent-ink`. Con todo lanzado y ventanas pendientes, el pie dice «Colocando las ventanas…». `ResultPanel` es el resumen al acabar: una línea por acción con su estado y el error en `font-mono` (`role="alert"`). Ambos entran con `.sift-swap`; Esc cancela el Espacio en marcha, y en el resumen Enter o Esc cierran la barra.
- **Etiqueta del filtro rápido** (`src/App.tsx`; los filtros, en `src/launcher/filters.ts`): «pdf …» se convierte en la etiqueta «PDF» junto a la caja, y la caja pasa a decir «Buscar en tus PDF». La etiqueta es un botón con un icono de cerrar que quita el filtro, y no toma el foco (`tabIndex={-1}`). Retroceso al principio de la caja también lo quita.

## Textos

Números y tiempos se escriben como los diría una persona, siempre con `src/format.ts`: `count` («1.240»), `files` («1 archivo»), `duration` («5 min 35 s», nunca «335 s») y `since` («hace 3 h», «ayer»). Los milisegundos solo aparecen en Ajustes → Estado, que es diagnóstico.

## Cómo añadir algo nuevo

1. Busca un token que encaje. Si no lo hay, añádelo en `index.css` con su valor oscuro **y** claro, expónlo en `@theme inline` y apúntalo en la tabla de arriba.
2. Si es una lista, usa `.sift-option`. Si es un control de formulario, usa los de `ui.tsx`.
3. Si se anima, usa los tokens de movimiento y comprueba que con «reducir movimiento» no se mueve.
4. `npm test` comprueba que no se ha colado ningún color literal.

## Accesibilidad

- **Campos (`.sift-field`)**: TextInput, Select y el resto de cajas de Ajustes usan `.sift-field`: borde suave (`--line`) y trazo inferior de 3:1 (`--line-strong`), como los TextBox de Fluent. Con el foco, el mismo anillo de 2 px del acento legible que los botones (F10, F11).
- **Ayuda de los campos (`Field`)**: la etiqueta lleva solo el nombre del campo. La ayuda va debajo, fuera de la etiqueta, y unida al control con `aria-describedby`: el lector de pantalla dice «Programa» y después la ayuda, no un nombre de dos líneas.
- **Diálogos (`Modal`)**: atrapan el foco (Tab y Mayús+Tab no salen del diálogo), Esc y el clic fuera lo cierran y, al cerrarse, el foco vuelve al control que lo abrió (`components/useFocusTrap.ts`, F12).
- **Anuncios de la barra**: una región `role="status"` oculta lee el número de resultados y el filtro («12 resultados en tus PDF») 400 ms después de dejar de escribir (`launcher/announce.ts`, F13).
- **Objetivos táctiles**: todo botón de solo icono mide al menos 24 × 24 px; el asa de arrastre, también (F14).
