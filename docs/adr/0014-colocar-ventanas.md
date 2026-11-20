# ADR 0014 — Colocar ventanas en zonas (Fase 6.5)

- **Estado:** aceptada (propuesta aprobada por Darío: solo zonas predefinidas; si no aparece la ventana, seguir y avisar)
- **Fecha:** 2026-11-12

## Contexto

Un Espacio abre cosas, pero cada ventana sale donde Windows quiere. Lo que se busca: «VS Code en la mitad derecha y el navegador en la izquierda», como los FancyZones de PowerToys.

## Decisión

### 1. Qué se coloca

Las acciones que abren algo (carpeta, archivo, web, aplicación) tienen un campo opcional **`place`**:

- **Zona predefinida:** toda la pantalla, mitades, tercios, dos tercios o cuartos. Las zonas propias quedan en el backlog.
- **Pantalla:** la principal, donde esté el ratón, o una concreta. Una concreta se identifica por su nombre y, como respaldo, por su esquina superior izquierda. Si ya no está conectada, se usa la principal: mejor colocarla ahí que no colocarla.
- **«Si ya está abierta, moverla en vez de abrir otra».**

Las exportaciones anteriores siguen siendo válidas: el campo es opcional y no se escribe si no se usa.

**Los comandos no se colocan, de momento.** `code .` lanza otro programa y termina, así que no se sabe qué ventana buscar.

### 2. Encontrar la ventana

Es lo difícil. Seguir el proceso lanzado no sirve: VS Code, los navegadores y el Explorador **reutilizan** un proceso que ya existía, y el que se lanzó termina enseguida. Por eso:

1. Antes de abrir, se apuntan las ventanas que ya hay.
2. Después, durante como mucho 15 s (60 s y sin bloquear desde el cambio del 2026-11-19, al final), se busca una ventana **nueva** del programa esperado:
   - una carpeta → `explorer.exe`;
   - un archivo o una web → el programa predeterminado para ese tipo, según Windows;
   - una app → su `.exe` si su id es una ruta; si es de la Tienda, se busca por su nombre en el título.
3. Si hay varias candidatas, gana la que tiene en el título el nombre de la carpeta, del archivo o de la app.
4. Solo cuentan las ventanas normales y visibles. Quedan fuera las ocultas, las «encubiertas» (de otro escritorio virtual), las de herramientas, los diálogos y las de Sift.

Si no aparece a tiempo, **se sigue y se avisa**: la barra dice «Abierto, con avisos» y, en esa línea, «se abrió, pero no encontré su ventana para colocarla». Es un aviso, no un fallo: el resto del Espacio se abre igual. Cancelar con Esc mientras se busca la ventana para lo que queda.

Un caso conocido: abrir una web con el navegador ya abierto añade una pestaña, no una ventana. Para eso está «moverla si ya está abierta».

**Cambio (2026-11-18): reutilizar sin colocar.** Darío pidió poder evitar que se abra otra ventana sin moverla. La zona pasa a ser opcional (`Placement.zone: Option<Zone>`, `#[serde(default)]`, así que los Espacios guardados se leen igual). Sin zona y con «Si ya está abierta, usar esa en vez de abrir otra», se busca la ventana como siempre y se trae al frente sin moverla (`WindowManager::focus`). Si no hay ninguna, se abre la acción sin esperar a su ventana. Sin zona y sin reutilizar no se guarda colocación. Alternativa descartada: un campo `reuse_window` suelto en cada acción, que obligaba a migrar los Espacios guardados y a repetir el campo en cuatro variantes.

### 3. Colocarla

1. Se restaura si estaba maximizada o minimizada.
2. Se calcula la zona sobre el **área útil** de la pantalla (sin la barra de tareas), en sextos, así que mitades y tercios encajan sin huecos.
3. Se compensa el **borde invisible** de Windows 10, unos 7 px a los lados y abajo que sirven para agarrar la ventana. Con `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` se ve qué parte se ve de verdad, y se mueve para que **lo visible** ocupe la zona.
4. Se repite una vez, por si al pasar a una pantalla con otra escala la app se redimensionó sola.
5. «Toda la pantalla» se maximiza en la pantalla elegida.

Sift tiene en cuenta la escala de cada pantalla, así que todas las coordenadas son píxeles reales.

**Cambio (2026-11-19): cada pantalla, como la llama Windows.** El número es el de Windows (el de `\\.\DISPLAY2`, el mismo de Configuración → Pantalla y de su botón «Identificar»), no el orden de izquierda a derecha, y el nombre es el del modelo según su EDID (`DisplayConfigGetDeviceInfo`: «ZOWIE XL LCD»), que antes salía como «Monitor PnP genérico» y se ocultaba. Una pantalla guardada con el nombre antiguo se sigue encontrando por su posición. Desde la rc.3, además, al arrancar Sift pone al día esas referencias antiguas en los Espacios guardados (`placement::updated_choice`: la pantalla del mismo sitio o, si no, la única con ese nombre), y el editor aplica la misma regla: «no conectada» solo sale cuando de verdad no está.

### 4. En el editor

- Debajo de cada acción que abre algo, «Colocar su ventana»: «No colocar» y una miniatura por zona, con la zona pintada en el color de acento y ordenadas por filas (completa y mitades · tercios · cuartos).
- Al elegir una zona aparece «Mitad derecha de [la pantalla principal / la pantalla donde esté el ratón / Pantalla 1, la de la derecha]» y la casilla para mover la que ya esté abierta.
- «Probar» la abre y la coloca de verdad.

### 5. Cómo está hecho

- `sift-spaces` tiene la parte pura: zonas, geometría, elegir pantalla y ventana, y el bucle de esperar y colocar. Lo que toca Windows va detrás del trait `WindowManager`, así que los tests usan un falso.
- La app pone el de verdad (`window_manager.rs`): `EnumWindows`, `EnumDisplayMonitors`, `GetMonitorInfoW`, `SetWindowPos`, DWM y `AssocQueryStringW`.

## Comprobado en pantalla

Una prueba que se lanza a mano (`places_real_windows`) abre ventanas de verdad, las coloca y **mide** dónde quedan, sin ratón ni teclado:

- El Bloc de notas cayó en la mitad derecha de la pantalla principal y luego en el tercio izquierdo de la otra, exacto al píxel.
- Una carpeta, por todo el camino de Sift (abrir → encontrar la ventana nueva del Explorador → colocarla), también exacta.

Desde el editor de la versión de desarrollo, «Probar» con «Mitad derecha de Pantalla 1, la de la derecha» dejó la carpeta visible en 2880–3840 × 0–1080: justo esa mitad.

## Alternativas descartadas

- **Seguir el proceso lanzado.** No sirve con las apps que reutilizan su proceso, que son justo las que más se usan.
- **Esperar un tiempo fijo y coger la ventana que esté delante.** Se equivoca si te cambias de ventana mientras tanto.
- **Zonas propias ya.** Se dejó para después, a propuesta de Darío. El modelo admite añadirlas sin romper nada, como otro tipo de zona.

**Cambio (2026-11-19): de quién es cada ventana.** Antes, una app registrada con un id de app (AUMID) en vez de con la ruta de su `.exe` (Claude, de la Tienda; Opera GX; VS Code) no tenía programa conocido. Se buscaba su nombre en el título de las ventanas y, si no salía, valía **la primera que hubiera**. Así, con «usar la ya abierta», la acción de Opera se quedó con la ventana de Claude. Ahora una ventana es de la acción si coincide (`placement::belongs`):
- **su id de app**, el que declara la ventana o el de su paquete de la Tienda (Claude);
- **su programa o el de un proceso padre**: las ventanas de Eclipse son de `javaw.exe`, nieto de `eclipse.exe`;
- **su carpeta de instalación**: el acceso directo de Opera apunta a `launcher.exe` y sus ventanas son de `opera.exe`, en la misma carpeta. La carpeta de Windows nunca cuenta.

Si no se sabe nada del programa, solo vale una ventana cuyo título lo nombre; nunca la primera que haya. Tampoco cuentan las ventanas de menos de 320 × 200 (pantallas de carga). Comprobado con las ventanas reales de Claude, Opera GX y Eclipse.

**Cambio (2026-11-19): las acciones no se esperan unas a otras.** Antes, cada acción que colocaba una ventana esperaba a que esa ventana apareciera (hasta 15 s) antes de lanzar la siguiente: un Espacio con Eclipse, que tarda y además enseña primero el selector de workspace, retrasaba todo lo demás. Ahora (`sift_spaces::arrange::Placer`):
- **Cada acción se lanza en cuanto se ha lanzado la anterior.** Se apunta qué ventana hay que colocar y se sigue.
- **Un solo bucle coloca todas las ventanas pendientes** según aparecen: después de cada acción, en cada vuelta de un paso «Esperar a que…» (lo único que de verdad espera) y, al final, hasta que no quede ninguna. Se mira cada 250 ms.
- **60 s de margen por ventana**, en vez de 15: ya no retrasa nada, y da tiempo a elegir el workspace.
- **Ventanas intermedias**: las que por su título son un selector, una carga o una actualización («Eclipse IDE Launcher», «Loading…», «Actualizando…») no se colocan; tampoco las que no tienen barra de título, como la pantalla de carga de Eclipse (un `Static` de 452 × 302 sin `WS_CAPTION`; Claude, Opera y el propio Eclipse sí la tienen aunque se pinten la suya). Se sigue esperando a la principal. Si una ventana sin barra de título dura 20 s y no sale otra, se toma por la principal (hay apps así). Si solo aparece una intermedia, el aviso lo dice: «solo vi su ventana «Eclipse IDE Launcher»: no la coloqué».
- **Si la ventana colocada se cierra antes de 1,5 s**, se sigue esperando a la siguiente del mismo programa.
- **Una ventana, una acción**: dos acciones de la misma app nunca se quedan con la misma ventana.
- El panel «en marcha» lo enseña con el evento `SpacePlacement`: «Eclipse abierto · colocando…» y luego ✓ (o ⚠ si no se pudo).

Descartado: un hilo por ventana. Daría lo mismo con más complicación (varios hilos llamando a Win32 a la vez y compartiendo qué ventanas ya son de alguien); un bucle con el mismo reloj que las esperas es fácil de probar con el reloj de mentira.
