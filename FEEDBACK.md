# Feedback de uso

Lo que va saliendo al usar Sift. Cada punto acaba en una tarea de [PLAN.md](PLAN.md) o en el backlog.

Formato: fecha, qué pasa o qué falta, y a dónde va.

## 2026-11-09 — Tras probar la Fase 1

- **Ajustes desde la barra.** Hoy solo se llega por el icono de la bandeja. Debería poder entrarse desde la propia barra: `Ctrl+,`, un icono discreto o escribiendo "ajustes". → Fase 1.5.
- **Espacios reales antes que la búsqueda.** Los resultados de ejemplo no sirven para el día a día; los Espacios básicos no dependen del indexado y se pueden adelantar. → Fase 1.5 (adelantada de la Fase 6).
  - Sin datos de ejemplo: si no hay Espacios, estado vacío con un atajo para crear el primero.
  - Editor en Ajustes: crear, editar, duplicar, borrar y reordenar.
  - Cada Espacio: nombre, alias, icono y lista ordenada de acciones (carpeta, archivo, URL, aplicación con selector, comando con programa + argumentos + carpeta de trabajo). Botón "probar" por acción.
  - Seguridad: sin shell, y confirmación la primera vez que se ejecuta un comando.
  - Persistencia en SQLite con migraciones. Exportar/importar a JSON.
- **Iconos personalizables.** Por Espacio: librería con buscador, emoji, imagen propia (PNG/SVG/ICO) o ninguno. Opción global "solo texto". Icono real de Windows por defecto para apps y archivos. → Fase 1.5.
- **Más personalidad visual.** Simple pero más bonita, que luzca en un vídeo para LinkedIn. Primero 2–3 direcciones visuales para elegir; después, sistema de diseño con tokens. Animaciones rápidas que no afecten a los < 100 ms y que respeten "reducir movimiento". → Fase 1.5 (adelanta parte de la Fase 7).
- **Colocar ventanas.** Acciones que abren una app en una zona de la pantalla (p. ej. "VS Code en la mitad derecha"), como FancyZones: esperar a que abra, identificar su ventana, varios monitores. → Fase 6.5 (nueva).

## 2026-11-10 — Tras usar los Espacios (rama `feat/pulido-ajustes`)

- ✅ **Vidrio en todas las ventanas.** Ajustes y sus diálogos también son de vidrio, con la misma alternativa opaca. Como en Fluent, cada superficie lleva su material: Acrylic en la barra; Mica (Windows 11) o blur con tinte (Windows 10) en Ajustes, porque Acrylic va a tirones al arrastrar o redimensionar en Windows 10; y vidrio por CSS en diálogos y barras fijas. Resuelto.
- ✅ **«Nuevo Espacio» al final de la lista**, donde aparecerá el Espacio creado. Resuelto.
- ✅ **Icono como avatar.** El icono va a la izquierda, con nombre y alias a su derecha (tarjeta de contacto); clic en el avatar para cambiarlo. Resuelto.
- ✅ **Reordenar arrastrando** Espacios y acciones, con línea del acento donde caerá; Alt+↑/↓ con teclado. El orden de los Espacios se guarda en SQLite al soltar; el de las acciones, al guardar el Espacio. Resuelto.

## 2026-11-10 — Feedback tras usar feat/pulido-ajustes

### Bugs
- [x] ✅ **Ctrl+, no abre Ajustes** (resuelto en `feat/fase-2-indexado`: se detecta por tecla física `e.code`; Ctrl+N abre Ajustes en «Nuevo Espacio», que es lo esperado) desde la barra (teclado español). Ctrl+N sí abre Ajustes. Flechas, Enter y Esc funcionan bien. Revisar cómo se detecta la tecla coma con distribuciones no inglesas (usar `event.code` y no solo `event.key`, o lo que corresponda) y comprobar que Ctrl+N hace lo que debe.
- [x] ✅ **El vidrio de Ajustes y el de la barra no se parecen** (tintes ajustados por material): Ajustes es mucho más translúcido. Deben verse coherentes.

### Apariencia: vidrio vs opaco
- [x] ✅ **Barra principal**: vidrio, pero menos translúcido que ahora, para que el fondo no choque ni reste legibilidad.
- [x] ✅ **Ajustes y diálogos: opacos por defecto.** Para algunas personas el vidrio en ventanas de trabajo es incómodo.
- [x] ✅ **Opción en Ajustes → Apariencia**: "Vidrio en toda la app" (desactivada por defecto). Si se activa, Ajustes y diálogos usan vidrio con la misma intensidad que la barra.
- [x] ✅ Dejarlo preparado para un futuro sistema de temas (bloque `appearance` en los ajustes) y personalización de la apariencia (no hacerlo ahora, solo no cerrarle la puerta).

### Editor de Espacios
- [x] ✅ **"Nuevo Espacio"**: sacarlo de la lista y ponerlo como botón fijo justo encima de "Importar" / "Exportar", con su mismo formato. Con muchos Espacios, al final de la lista quedaría inaccesible.
- [x] ✅ **Quitar el arrastre para reordenar Espacios.** (se mantiene Alt+↑/↓ hasta el orden por uso de la Fase 5) Solo cambia el orden de la lista y no aporta nada. Valorar si mantener Alt+↑/↓ o quitar también el orden manual (por ejemplo, orden alfabético o por uso).
- [x] ✅ **Arrastre de acciones con animación real** — adelantado a la Fase 2 a petición de Darío (ver más abajo, con dnd-kit): al arrastrar, la tarjeta se mueve con el puntero y las demás se desplazan con una animación suave para hacerle hueco (tipo listas de Notion o Linear), no solo una línea que marca dónde cae. Respetar "reducir movimiento" de Windows.
- [x] ✅ **Avatar y campos desalineados** (avatar de 88 px = bloque de campos 40+12+36, en la rejilla de 4 px): el icono (80 px) es más bajo que el bloque de Nombre + Alias, así que queda a una altura rara. Ajustarlo aplicando buenas prácticas de diseño de interfaces (alineación, ritmo vertical y espaciado del sistema Vidrio): por ejemplo, que el avatar ocupe la altura del bloque o que ambos queden alineados arriba con una relación de tamaños coherente. Decidir y justificar.
## 2026-11-11 — Tras usar la Fase 2 (búsqueda)

### Vidrio
- [x] ✅ **Vidrio único.** — Acrylic con el mismo tinte en todas las ventanas; Ajustes pasa a blur con ese tinte solo mientras se mueve (Acrylic da tirones en Windows 10) y vuelve a Acrylic 200 ms después. Los diálogos usan el mismo tinte compuesto. Comparado con capturas. El de la barra (Alt+Espacio) es el bueno: translúcido, se intuye lo de detrás sin ser transparente. Ajustes y diálogos tienen que verse *exactamente* igual, no parecido. Comparar con capturas una al lado de la otra.
- [x] ✅ **Por defecto, toda la app con vidrio.** — Ajustes → General → «Desactivar vidrio», que deja opacas también la barra. La opción pasa a ser «Desactivar vidrio» (todo opaco).

### Editor de Espacios
- [x] ✅ **Arrastre de acciones con animación real, ya** — dnd-kit; sin animación con «reducir movimiento». (no en la Fase 7): la tarjeta sigue al puntero y las demás se apartan suavemente para hacerle hueco (estilo Notion/Linear). Mejor con una librería probada (p. ej. dnd-kit). Darío lo prueba con ratón real.
- [x] ✅ **Al arrastrar por el asa ⠿ se selecciona texto o el icono.** — asa `select-none`, nada seleccionable mientras se arrastra e iconos no arrastrables. Que el asa y la tarjeta no seleccionen mientras se arrastra, sin romper la edición de los campos.

### Ajustes → Carpetas, para cualquiera
- [x] ✅ Los patrones a ignorar vienen por defecto y van en «Avanzado», plegado, con una explicación en lenguaje normal.
- [x] ✅ Estado en lenguaje humano: «Al día · 1.240 archivos», «Buscando cambios…», «Leyendo 32 de 120 archivos», con barra de progreso.
- [x] ✅ Por carpeta: nombre, ruta abreviada, número de archivos y última actualización.

## 2026-11-12 — Tras usar la Fase 2 fusionada (rama `feat/fase-3-formatos`)

### Ventana de Ajustes
- [x] ✅ **Se abre MUY transparente** y no coge el vidrio bueno hasta que se mueve un milímetro. El material correcto tiene que estar desde el primer fotograma.
- [x] ✅ **La barra de título es negra.** (barra propia sobre el vidrio) Que sea del mismo vidrio que la ventana (o, como mínimo, del color del fondo). Valorar una barra propia con botones estilo Windows, sin perder el ajuste a los bordes (snap) ni el arrastre. Tiene que funcionar en Windows 10.
- [x] ✅ **Al mover la ventana, el blur temporal se ve más transparente** que el Acrylic. Ajustar el tinte para que el cambio casi no se note.
- [x] ✅ **El pie fijo del editor** (Guardar / Guardado / Duplicar / Borrar) sigue opaco. Debe seguir la opción de vidrio.
- [x] ✅ **Orden de la navegación**: como las apps estándar de Windows (General primero; Estado/Acerca de al final).

### Textos
- [x] ✅ **Duraciones legibles**: nada de «335 s». «5 min 35 s», «2 min», «hace 1 h»… en toda la app.

### Carpetas e indexado
- [x] ✅ **Archivos que no se pudieron leer** (salen 439): un desplegable pequeño que los agrupe por motivo con su número («formato no compatible», «protegido con contraseña», «sin permiso», «demasiado grande», «dañado»…) y unos pocos ejemplos por grupo.
- [x] ✅ **La carpeta de la uni dice 86.509 archivos** (quedan 11.888 útiles; ver ADR 0007), exagerado. Indexar solo formatos útiles (texto, código, documentos) e ignorar por defecto binarios, compilados, imágenes, vídeos, cachés, entornos virtuales y dependencias. Explicar qué había en esos 86.509.

### Para la Fase 7 (pulido)
- [x] ✅ Las animaciones del arrastre de acciones cambian de color de golpe al coger y al soltar la tarjeta: tienen que ser transiciones suaves. Habrá que pulir a fondo todas las animaciones — fondo y sombra con transición de 150 ms al coger y soltar; cambios de panel en la barra y estados de los botones también suaves (Fase 7, bloque 5).

## 2026-11-12 (2) — Tras probar los ajustes de la ventana y del indexado

- [x] ✅ **El cambio entre blur y Acrylic al mover Ajustes se nota demasiado** (un velo del color del fondo tapa los dos cambios de material), sobre todo al volver al Acrylic al soltar. Suavizarlo (p. ej. una capa de tinte con un fundido de ~200 ms que tape el cambio de material) o explicar por qué no se puede más.
- [x] ✅ **Volver a reordenar los Espacios arrastrando**, con la misma animación y librería (dnd-kit) que las acciones. Alt+↑/↓ se mantiene. A la gente le puede gustar tener su orden.
- [x] ✅ **El desplegable de «no se pudieron leer» va dentro de cada carpeta**: debajo de su fila, con su flechita y solo lo de esa carpeta. Fuera el bloque general del medio; arriba basta el estado global.
- [x] ✅ **Imágenes y vídeos por nombre** (sin leer el contenido), siempre que no estén en carpetas generadas. Opción en Avanzado para desactivarlo.

## 2026-11-13 — Tras fusionar la Fase 3 (rama `feat/fase-4-semantica`)

- [x] ✅ **Al cerrar Ajustes, volver a la barra** si se abrieron desde ella (Alt+Espacio → Ajustes), con el foco en la búsqueda: lo normal es que hayas cambiado algo y quieras usar la app. Si se abrieron desde la bandeja, al cerrar no se muestra la barra.

## 2026-11-12 — Bloque de mejoras antes de la Fase 6.5 (rama `feat/mejoras-barra`)

- [x] ✅ **Textos para cualquiera**: sin jerga ni ejemplos sacados de los apuntes de Darío (la búsqueda inteligente ponía «normalización» y «SQL»). Los términos técnicos, solo en «Avanzado» y explicados.
- [x] ✅ **Barra vacía útil**: «Espacios» y «Abierto recientemente» (lo último que abriste desde Sift), con teclado.
- [x] ✅ **Abrir el PDF en la página** del fragmento que encaja, con «pág. 37» en el resultado.
- [x] ✅ **Filtros rápidos** por tipo («pdf …», «código …», «word …», «imagen …») con una etiqueta que se quita con Retroceso.

### Para la Fase 7 (pulido)
- [ ] ⏳ **«Por qué sale este resultado»**: enseñar el fragmento que encaja (y, si llegó por significado, que se entienda la relación). En la barra actual no cabe: pensar el formato (panel lateral, vista previa, al pulsar una tecla como → o Espacio…) sin romper la apertura en < 100 ms.

## 2026-11-18 — Tras probar la versión con vista previa (PR #24 a #31)

### 1. Teclado después de usar el ratón (fallo importante)
- [x] ✅ **Resuelto** (`fix/teclado-tras-raton`): las teclas se escuchan en toda la ventana y no solo en la caja, el foco vuelve a la caja antes de actuar, Tab nunca saca el foco de la ventana y un clic en la barra no le quita el foco a la caja (salvo para seleccionar el texto de la vista previa). Tras mover o usar el ratón en la barra, Esc deja de volver de la vista previa a la lista y Tab cierra Sift en vez de cerrar la vista previa. Los atajos tienen que funcionar siempre, tenga el foco donde lo tenga.
- [x] ✅ **Resuelto** (`feat/vista-previa-espacios`: botón ← y pistas del pie pulsables para abrir, vista previa, volver, carpeta, copiar ruta, nuevo Espacio, cancelar y cerrar). Sift se tiene que poder usar entero con ratón: barra, vista previa y Ajustes. Ajustes ya se puede (todo atajo tiene su botón o su arrastre). En la barra faltan volver de la vista previa, cancelar un Espacio y cerrar el resumen: van con las pistas pulsables del punto 2.
- [x] ✅ Tests del caso «ratón y después teclado» (`App.test.tsx`): fallan con el código de antes y pasan con el arreglo.

### 2. Vista previa
- [x] ✅ Botón «atrás» (←) visible para volver a la lista con el ratón.
- [x] ✅ Tab sobre un Espacio abre una vista previa con sus acciones en orden (qué abrirá, las esperas y dónde colocará cada ventana). Enter lo lanza.
- [x] ✅ Pistas del pie con mayúscula inicial («Moverse», «Abrir», «Vista previa», «Volver») y que se puedan pulsar con el ratón cuando tenga sentido.

### 3. Mayúsculas en toda la app
- [x] ✅ **Resuelto** (`fix/mayusculas`; lo que faltaba: opciones de «Esperar a que…» y «Si no ocurre», textos de Estado y la lista de filtros; las pistas del pie, en el punto 2; «la pantalla principal», en el punto 4). Etiquetas, pistas, opciones de los desplegables y textos de ejemplo con mayúscula inicial, como en Windows. Los valores literales que se escriben tal cual (comandos, rutas, direcciones web) se quedan como son.

### 4. Editor de acciones
- [x] ✅ **Resuelto** (`feat/editor-acciones`; «Palabras clave» y filas con nombre, elegidos por Darío; ADR 0014 actualizado). «Ejecutar comando»: la ayuda de «Programa» queda encajada en una columna estrecha. Recolocar campos y ayudas.
- [x] ✅ Colocar ventana: «No colocar» queda solo a la izquierda y el resto a la derecha. Distribución más ordenada.
- [x] ✅ El desplegable de pantalla dice «la pantalla principal»: mostrar el nombre real («Pantalla 1 · DELL U2720Q (principal)») y «Donde esté el ratón».
- [x] ✅ «Si ya está abierta…» también con «No colocar», renombrado a «Si ya está abierta, usar esa en vez de abrir otra», en carpeta, archivo, web y aplicación.
- [x] ✅ «Esperar a que…»: una línea que explique por qué hace falta una dirección y qué poner.
- [x] ✅ «Alias» → algo más claro («Palabras clave» u «Otras palabras para encontrarlo»), con su ayuda.

### 5. Estado (Ajustes)
- [x] ✅ **Resuelto** (`feat/estado-tarjetas`). De lista plana «nombre → texto» a tarjetas que digan de un vistazo si todo va bien, con icono de estado. Lo técnico, en «Avanzado» plegado.

### 6. Primeros pasos y bienvenida
- [x] ✅ **Resuelto** (`feat/boton-cambiar`: botón con fondo, como los demás). «Cambiar» (junto al atajo) parece texto: darle aspecto de botón.
- [x] ✅ **Hecho** (`feat/introduccion-guiada`, con los cambios de Darío: 5 pasos, búsqueda de prueba dentro y «Saltar» sin confirmar; capturas en `docs/capturas/introduccion/`). Introducción guiada y animada la primera vez (un paso por pantalla, progreso, Atrás / Siguiente / Saltar, que enseñe haciendo, repetible desde Ajustes). Antes, propuesta corta para que Darío la apruebe.

### 7. Pendiente del resumen de anoche
- [x] ✅ Lo decidido de noche: aprobado (con el cambio de Tab sobre un Espacio del punto 2).
- [x] ✅ Copias ocultas: se queda como está.
- [x] ✅ **Hecho** (`test/cobertura-rust`: 70,8 % total; núcleo 85–100 %; vista previa cubierta). Cobertura de Rust con cargo-llvm-cov: medir, apuntarlo en `docs/cobertura.md` y cubrir los huecos críticos.
- [x] ✅ **Hecho** (`chore/eslint-minimatch`: eslint 10.15.0 y typescript-eslint 8.73.0; minimatch ya no sale. Queda un aviso alto de `picomatch` en herramientas de desarrollo (vite, vitest, typescript-eslint) sin versión arreglada todavía; la app no lo lleva). npm audit: actualizar ESLint y sus dependencias para quitar el aviso de minimatch, si no rompe nada.
- La Fase 7 la cierra Darío cuando la pruebe con alguien que no conozca Sift.

## 2026-11-19 — Usando la 1.0.0-rc.1 instalada, con datos reales

### 1. Fallos
- [x] ✅ **Resuelto** (`fix/fallos-bloque1`: las opciones llevan fondo y texto sólidos del tema; comprobado en pantalla en oscuro y claro). **Desplegables ilegibles:** las opciones del desplegable de pantalla salen en blanco sobre blanco. Revisar todos los desplegables, en tema oscuro y claro.
- [x] ✅ **Resuelto** (`fix/fallos-bloque1`: cada ventana se reconoce por su id de app, por su programa o el de un proceso padre, o por su carpeta de instalación; nunca «la primera que haya»; tests con el caso de Claude y Opera). **Ventana equivocada:** Espacio con Claude (mitad derecha) y Opera (mitad izquierda) en la Pantalla 2, los dos con «usar la ya abierta». Opera no se abrió y Claude acabó en la mitad de Opera: la acción de Opera se quedó con la ventana de Claude. Encontrar cómo se identifica el programa de cada ventana (apps Electron y de la Tienda, ventanas ya abiertas) y añadir un test.
- [x] ✅ **Resuelto** (`fix/fallos-bloque1` + `feat/acciones-sin-esperas`: sus ventanas son de `javaw.exe`, nieto de `eclipse.exe`, y cuentan; la pantalla de carga (sin barra de título) y el selector («Eclipse IDE Launcher») no se colocan y se espera a la principal hasta 60 s; si solo sale el selector, el aviso lo dice; probado con el Eclipse real). **Eclipse, «no encontré su ventana»:** abre primero el selector de workspace y luego la principal. Las ventanas intermedias (selector, carga, diálogos) no cuentan: hay que esperar a la principal.

### 2. Las acciones no se esperan unas a otras
- [x] ✅ **Resuelto** (`feat/acciones-sin-esperas`: se lanzan seguidas, en menos de 0,5 s, y las ventanas se colocan en segundo plano según aparecen, también durante las esperas; 60 s por ventana; evento `SpacePlacement` para el panel; ADR 0014 y 0012 actualizados; probado con Eclipse y Opera reales). Cada acción se lanza en cuanto se lanza la anterior; colocar cada ventana va en segundo plano cuando aparece. Solo se espera de verdad con un paso «Esperar a que…». Tiempo para encontrar la ventana más largo (p. ej. 60 s). El panel «en marcha» lo dice («Eclipse abierto · colocando…» y luego ✓). Actualizar el ADR.

### 3. Editor de acciones
- [x] ✅ **Resuelto** (`feat/editor-acciones`: número de `DISPLAYn` y modelo por EDID; «Identificar» probado en las dos pantallas). Pantallas con el número y el nombre de Configuración → Pantalla («Pantalla 1 — DELL U2720Q (principal)»); manda el número de Windows. Botón «Identificar» que enseña un número grande en cada monitor unos segundos.
- [x] ✅ **Resuelto** (`feat/editor-acciones`). «Colocar su ventana» plegable: plegada, un resumen («Posición: Mitad derecha · Pantalla 2» o «No colocar») con «Cambiar»; por defecto plegada si ya hay posición.
- [x] ✅ **Resuelto** (`feat/editor-acciones`). Título claro de la posición elegida («Posición seleccionada: Mitad derecha»).
- [x] ✅ **Resuelto** (`feat/editor-acciones`: las apps de la Tienda usan el logo «sin placa» de su paquete; el resto, el icono a 64 px; revisados Claude, Opera, VS Code y Eclipse en tema oscuro y claro). Icono de Claude con esquinas azules raras: icono real en buena resolución, también para apps de la Tienda (MSIX). Revisar Claude, Opera, VS Code y Eclipse.

### 4. Primeros pasos
- [x] ✅ **Respondido** (Ajustes → Primeros pasos → «Ver la introducción otra vez» sin perder nada; tal cual la primera vez, en Windows Sandbox con `scripts/sandbox.ps1`, que es lo recomendado). No salió al instalar porque ya había datos (correcto). Explicar cómo verla como la primera vez sin perder datos y si conviene en Windows Sandbox.

### 5. Vídeo
- [x] ✅ Se deja para la versión final, para no usar pantallas antiguas (apuntado en `PLAN.md`).

### 6. Introducción
- [x] ✅ Las barritas de progreso se quedan.
- [x] ✅ **Resuelto** (`feat/intro-jerarquia`: la explicación va en texto normal y la acción en su propio bloque; lo de debajo del título entra escalonado, 70 ms entre uno y otro, nada con «reducir movimiento»). Jerarquía: en el paso 1 el ojo salta del título a «Se abre desde cualquier sitio…» y se salta la explicación. Orden natural en todos los pasos (título, explicación, acción), con el texto escalonado tras el título.
- [x] ✅ **Resuelto** (`feat/intro-animaciones`, propuesta aprobada: una ilustración por paso, una vez, < 1,6 s; último fotograma con «reducir movimiento»). Más vida: una animación ilustrativa por paso, una sola vez, rápida, coherente con Vidrio y nada con «reducir movimiento». Propuesta antes de implementarla.

### 7. Indicador de carga propio
- [x] ✅ **Resuelto** (`feat/indicador-tamiz`: `SiftLoader`/`SiftLoading`, documentado en Vidrio; en el indexado, la descarga, la vista previa lenta y las listas que cargan). Con la identidad de Sift (el tamiz con sus barras abriéndose y cerrándose) para el primer indexado, la descarga de la búsqueda inteligente y la vista previa si tarda. Versión estática o con pulso suave con «reducir movimiento». Un componente, documentado en Vidrio.

### 8. Instalador personalizado
- [x] ✅ **Resuelto** (`feat/instalador`, imágenes aprobadas; ADR 0019: icono, imagen lateral y cabecera, solo en español, textos propios y «Abrir Sift»). Icono de Sift, imágenes lateral y de cabecera con el estilo de Vidrio, textos en español y «Abrir Sift» al final. Decir qué se puede personalizar y enseñar las imágenes antes.

Al terminar: instalador 1.0.0-rc.2 y cómo actualizar el instalado.

## 2026-11-20 — Usando la 1.0.0-rc.2

### 1. Indicador de carga en el panel «Abriendo…»
- [x] ✅ **Resuelto** (`fix/rc3`: el tamiz en el paso en marcha, en cada ventana que se coloca y junto a «Colocando las ventanas…»; además en las tarjetas «en marcha» de Estado, en «Probar» y en la descarga de General, que no enseñaba progreso; con test). El panel «Abriendo «…»» / «Colocando las ventanas…» no usa el indicador de carga con el logo. Usarlo ahí (junto a «Colocando las ventanas…» y en el paso en marcha) y revisar toda la app buscando otros sitios donde algo tarda sin indicador (vista previa, primer indexado, descarga, «Probar»…), con criterio.

### 2. Pantalla duplicada en el desplegable de un Espacio
- [x] ✅ **Resuelto** (`fix/rc3`: al arrancar, las referencias antiguas pasan a la pantalla de su mismo sitio o, si no, a la única con ese nombre; el editor aplica la misma regla; tests en Rust y en la UI con este caso: tres acciones guardadas como «Pantalla 1» en x = 1920 pasan a «22EN33»). Sale «Pantalla 1 — 22EN33», «Pantalla 2 — ZOWIE XL LCD (principal)», «Donde esté el ratón» y además «Pantalla 1 (no conectada: se usará la principal)». Es un Espacio guardado antes del cambio de nombres de las pantallas. Al cargar, convertir las referencias antiguas a la pantalla correcta cuando se pueda identificar; «no conectada» solo cuando de verdad no esté. Con test.

Al terminar: instalador 1.0.0-rc.3 y cómo actualizar el instalado.

## 2026-11-24 — Probando el instalador 1.0.0 en Windows Sandbox

### 1. Sift no arranca en un Windows limpio (bloquea la 1.0)
- [x] ✅ **Resuelto** (`fix/vc-runtime`, ADR 0020: las cuatro DLL del runtime de Visual C++ van junto al ejecutable, y `scripts/comprobar-instalador.ps1` hace fallar la CI local y el workflow de Release si falta alguna o es demasiado antigua). Al abrir Sift: «La ejecución de código no puede continuar porque no se encontró MSVCP140.dll». En el PC de Darío no pasa porque Visual Studio instala ese runtime. Lo pide ONNX Runtime.

### 2. «Could not find the WebView2 Runtime» en un Windows limpio
- [x] ✅ **Resuelto** (`fix/webview2`, ADR 0021: el instalador lleva el bootstrapper de Microsoft, comprueba de verdad que WebView2 queda instalado y, si no puede, avisa en español con el enlace oficial y no instala Sift; `scripts/sandbox.ps1` comparte solo el instalador más reciente y tiene `-SinRed` para probar el aviso). Con el runtime de Visual C++ ya arreglado, Sift fallaba al abrir en el Sandbox, que no trae WebView2 (algunos Windows 10 reales tampoco).
