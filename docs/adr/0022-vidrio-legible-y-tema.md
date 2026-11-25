# ADR 0022 — Vidrio legible sobre cualquier escritorio, opaco donde Windows no lo pinta, y tema elegible

- **Estado:** aceptada (las cifras y los textos son revisables: ver `docs/proceso/pendiente-decidir.md`, puntos 17 y 18)
- **Fecha:** 2026-11-25

## Contexto

Al probar el instalador 1.0.0 en Windows Sandbox con el tema claro y el fondo azul de Windows, la introducción salió casi transparente y el texto se leía mal. Con «Desactivar vidrio» se veía bien. Había dos problemas:

1. **El Sandbox no pinta el Acrylic.** Windows aceptó el material sin dar error, pero no dibujó ni el desenfoque ni el tinte. Solo quedaba la capa de la UI, `--surface` al 28 %, y el escritorio se veía casi tal cual. Pasa en cualquier Windows sin aceleración gráfica: el Sandbox, máquinas virtuales o un PC sin el driver de su tarjeta gráfica.
2. **Aun pintándolo bien, el tinte se quedaba corto.** Se ajustó «a ojo» sobre fondos normales: con el Acrylic al 78 % y la UI encima, cubría el 84 % del escritorio. En tema claro sobre un escritorio negro, el texto secundario quedaba en 3,8:1, el rojo de error en 3,5:1 y el verde de éxito en 3,4:1. WCAG AA pide 4,5:1. En tema oscuro sobre un escritorio blanco, 4,2:1.

Darío pidió además poder elegir el tema en Sift: «Como Windows», «Claro» u «Oscuro».

## Decisión

### 1. El contraste se calcula sobre el peor escritorio, y lo comprueba un test

`src/glass-contrast.test.ts` lee los colores reales de `index.css` y el tinte del Acrylic de `materials.rs`. Compone el vidrio sobre 11 escritorios: negro, blanco, gris, los seis colores saturados primarios y secundarios, y el azul de Windows 10 claro y oscuro. Lo hace con y sin una fila seleccionada encima, y exige 4,5:1 a todo el texto (`--fg`, `--muted`, `--danger`, `--success` y `--accent-ink`). Si alguien baja la opacidad o aclara un color, el test falla.

Para pasarlo hicieron falta dos cosas:
- **Más opacidad: el vidrio cubre el 88 % del escritorio** en los dos temas, frente al 84 % de antes. El tinte del Acrylic no cambia (200/255); sube la capa de la UI, `--surface`, del 22 % y 28 % al 45 %. El desenfoque se sigue notando, sobre todo con fondos de colores.
- **Colores de texto algo más marcados.** Tema claro:
  - texto secundario de `rgb(92 99 112)` a `rgb(74 80 92)`;
  - error de `rgb(196 43 28)` a `rgb(160 30 20)`;
  - éxito de `rgb(15 123 15)` a `rgb(10 96 10)`.

  Tema oscuro:
  - texto secundario de `rgb(166 173 187)` a `rgb(186 192 204)`;
  - error a `rgb(255 168 178)`;
  - éxito a `rgb(124 214 110)`.

  La diferencia apenas se nota, y sobre el fondo opaco el contraste también sube.

El acento del usuario se ajusta ahora contra el fondo opaco y contra el peor fondo del vidrio (`GLASS_WORST_*` en `appearance.rs`; el test comprueba que coinciden con el CSS). Por ejemplo, el azul de Windows en tema claro se oscurece un poco.

**Alternativa descartada:** opacidad suficiente sin tocar los colores. En tema claro haría falta un 92 %, y el vidrio dejaría de verse.

### 2. Donde Windows no puede pintar el vidrio, Sift va opaco solo

Windows no dice si está pintando el material, así que `glass_support.rs` mira las situaciones conocidas en las que no lo hace bien. En ellas no se pide vidrio y la UI pinta su fondo opaco:

| Situación | Cómo se detecta |
|---|---|
| «Efectos de transparencia» desactivados | Registro `Personalize\EnableTransparency = 0` |
| Windows Sandbox | La cuenta `WDAGUtilityAccount`, la del Sandbox |
| Escritorio remoto | `GetSystemMetrics(SM_REMOTESESSION)` |
| Sin aceleración gráfica (máquina virtual, sin driver) | DXGI: la tarjeta principal es por software o es el «Microsoft Basic Render Driver» (0x1414:0x8C) |
| Ahorro de batería | `GetSystemPowerStatus`, `SystemStatusFlag = 1` |

Se comprueba al arrancar, cada vez que se muestra la barra y cada vez que Ajustes recibe el foco. Mirar la tarjeta gráfica cuesta unos ms y se hace una sola vez; el resto, microsegundos (hay un test que lo mide). Así, si activas el ahorro de batería, la próxima vez que abras la barra ya es opaca, y vuelve a ser de vidrio al quitarlo.

En Ajustes → General → Apariencia, debajo de «Desactivar vidrio», se explica en una línea: «Ahora mismo el vidrio no se usa: Sift se está ejecutando en Windows Sandbox. Sift pone un fondo opaco…».

**Alternativas descartadas:**
- **Comprobar con una captura de pantalla si el tinte está pintado.** Es frágil, lento y depende de lo que haya detrás.
- **Que la capa de la UI baste sola para leer.** Haría falta casi un 90 % de opacidad propia, y el vidrio desaparecería en todos los PC.

**Límite:** si hay otro caso en que Windows acepta el Acrylic sin pintarlo y no está en la lista, el usuario lo verá como en la captura. Para eso sigue estando «Desactivar vidrio», y el caso se puede añadir a la lista.

### 3. Tema: «Como Windows» (por defecto), «Claro» u «Oscuro»

El tema se elige en Ajustes → General → Apariencia y se guarda en `settings.json` (`appearance.theme`). Rust lo tiene en memoria (`appearance::set_preference`), así que leer el tema sigue costando microsegundos al abrir la barra.

Al cambiarlo:
- se vuelve a poner el Acrylic con el tinte del tema nuevo;
- Rust avisa a todas las ventanas con el evento `AppearanceChanged`;
- la barra, Ajustes, sus diálogos y la introducción se repintan al momento (`followAppearanceChanges` en `appearance.ts`).

«Desactivar vidrio» usa el mismo evento.

**Lo que no sigue el tema de Sift:**
- los diálogos nativos de Windows, como la aprobación de comandos (los pinta Windows con su tema);
- el icono de la bandeja, que va con el tema de la barra de tareas.

## Consecuencias

- El vidrio es algo más opaco que antes (88 % frente al 84 %): un poco menos espectacular, pero se lee sobre cualquier fondo.
- Bajar la opacidad o aclarar un color de texto hace fallar `glass-contrast.test.ts`.
- En el Sandbox y en las máquinas virtuales sin tarjeta gráfica, Sift es opaco y lo explica.
- Sin comprobar todavía en el Sandbox ni en una máquina virtual. Es la prueba A3b de `docs/prueba-instalacion.md`.
