# unslop: auditoría y dirección de diseño (Fase 7)

Con [unslop](https://github.com/heydamir/unslop), de Damir Sokolovsky ([@heydamir](https://x.com/heydamir)), con licencia CC BY 4.0.

- **Qué se miró:** la barra, Ajustes, el editor de Espacios y los paneles, en el código. También las capturas de Ajustes de la sesión anterior. La barra aún no se ha visto en pantalla porque hace falta tu permiso, y se mirará antes de dar por buenas las pasadas.
- **Veredicto:** **poco slop**. Vidrio es un sistema diseñado: no hay degradados de color, brillos, colores inventados ni textos con exclamaciones o palabras de moda. Quedan algunos signos concretos y, sobre todo, falta **carácter propio**: si cambias el nombre, la barra podría ser la de otro lanzador. unslop pide entonces **modo pulido**: la dirección sale de la marca que ya existe (Vidrio y Windows), no se inventa otra, y se tocan solo los signos encontrados. Las ideas más grandes, los momentos y la firma, se hacen en los bloques que les tocan (vista previa, bienvenida).

## Auditoría (signos de `signs.md`)

| Grupo | Signo | Dónde |
|---|---|---|
| Iconos | Emoji como iconos: ⚙️ (Ajustes) y ➕ (Crear Espacio) | `launcher/items.tsx` (comandos de la barra) |
| Iconos | Glifos de texto sueltos como iconos, cada uno de una fuente: ⚙ (pie), × (quitar filtro), ⠿ (arrastrar), › (desplegar), ✓ ✕ – ! (resumen) | `App.tsx`, `ActionEditor.tsx`, `FoldersSection.tsx`, `Settings.tsx`, `panels.tsx` |
| Tipo | Rótulos en MAYÚSCULAS con espaciado (`ESPACIOS`, `ATAJO PARA ABRIR SIFT`, `AVANZADO`…) | `ResultsList.tsx`, `Settings.tsx` (×2), `FoldersSection.tsx` (×2) |
| Tipo | Tamaños fuera de la escala: 15 px (títulos de fila), 11 px (cabeceras de sección) | `ResultsList.tsx`, `panels.tsx` |
| Tipo | Números que cambian sin cifras tabulares («Leyendo 32 de 120», segundos, ms, MB) | `folders.ts`, `runSummary.ts`, Estado |
| Espaciado | Un valor suelto: `pl-[104px]` (sangría de la ayuda de alias, a mano) | `SpaceEditor.tsx` |
| Detalles | Atajos como texto plano («↑↓ moverse», «↵»): no son teclas | pie de la barra, filas |
| Texto | Marcador largo: «Buscar icono (en español o inglés): carpeta, base, código…» | `IconPicker.tsx` |
| Plantilla | La barra y Ajustes no tienen nada que diga «esto es Sift»: ni un momento ni una firma | toda la app |

Lo que **no** aparece: degradados, texto con degradado, brillos de color, bordes de neón, peso 700–900, cursiva, `transition-all`, animaciones en bucle, insignias «New ✨», exclamaciones ni saludos.

## Dirección (para que la apruebes)

- **Producto.** Un lanzador para Windows: encuentra tus archivos por lo que dicen, no solo por su nombre, y abre de una vez todo lo de aquello en lo que estás trabajando, sin que nada salga de tu PC.
- **Carácter.** **discreto, exacto, de casa.** Aparece, acierta y se va; nunca llama la atención sobre sí mismo, y se siente parte de Windows, no una app que se ha colado.
- **Mundo.** Tu propio escritorio y tus papeles. La barra es una lámina del vidrio de Windows (Acrylic) sobre tu fondo, con la tipografía y el color de acento que tú elegiste en Windows. Lo que tiene materia es **el contenido de tus archivos**: el texto de un apunte, una página de un PDF, unas líneas de código.
- **Momentos** (los únicos sitios con riqueza, todo lo demás callado):
  1. **El fragmento que encaja.** En la vista previa (bloque 7), el trozo de tu archivo tal como es: la página del PDF o el código en su tipografía de ancho fijo, con lo que buscaste marcado. Es lo que demuestra que Sift te ha entendido.
  2. **Abrir un Espacio.** Las acciones que se van abriendo una a una, como una secuencia viva con su ✓ (el panel «en marcha»), en vez de un texto de estado.
- **Firma.** **La marca de lo que encaja**: las palabras encontradas subrayadas con un trazo fino del color de acento, siempre igual, en la lista, en la vista previa y en la ayuda de los filtros. Sustituye a la caja tintada de hoy. Es lo que hace Sift (encontrar), dibujado de una sola manera.

## Decisiones de la auditoría que esto fija (criterio Windows 11 / Fluent)

- **F3:** las cabeceras pasan de MAYÚSCULAS con espaciado a **mayúscula inicial en seminegrita (600)**, como en Configuración de Windows 11.
- **Iconos:** todos los de la interfaz con **Segoe Fluent Icons** (o Segoe MDL2 Assets en Windows 10), la fuente de iconos del sistema, que ya usan los botones de la barra de título. Mismo tamaño y color del texto. No añade dependencias y suena a Windows. Lucide se queda solo para lo que elige el usuario en el icono de un Espacio.
- **Escala de tipo de Fluent:** Caption 12, Body 14, Body Strong 14 en seminegrita, Subtitle 20. Los 15 y 11 px pasan a 14 y 12.
- **Atajos como teclas pequeñas** (keycaps) en el pie y en las filas. Es lo que hace Windows 11 en sus menús, y Sift se usa con el teclado.
- **F10:** bordes de los campos con contraste de 3:1, solo en los campos.
- **F17:** pulsar oscurece un paso (Fluent no encoge al pulsar).
- **Se mantiene:** el vidrio y sus tintes, el acento del usuario (con la variante legible de F9), los radios (4 en controles y 8 en ventanas, los de Fluent), la tipografía Segoe UI Variable y la distribución de cada pantalla.

## Aprobada, con estos ajustes de Darío (2026-11-17)

- **La firma también por significado.** Si el archivo no contiene las palabras buscadas, se marca **el pasaje entero que encaja** (el fragmento que Sift ha elegido por su significado), con la misma marca. Exacta o por significado, la firma es una sola: un subrayado de 2 px con el acento legible (`--accent-ink`, 4,5:1 sobre el fondo; con el lavanda de este PC en tema claro, el acento tal cual daba 2,8:1).
- **Iconos comprobados en Segoe MDL2 Assets 1.79 (Windows 10):** todos existen y tienen dibujo (se pintaron para verlos). Una alternativa: el asa de puntos de Fluent no está en MDL2 y se usa el asa de barras (`GripperBarHorizontal`, E76F), la de las listas de Windows 10.
- **Teclas (keycaps) solo en el pie y en la fila seleccionada.**
- **Capturas antes y después** en `docs/capturas/`, con el permiso de Darío para las pruebas en pantalla.

## Las nueve pasadas

1. **Espaciado:** los dos valores sueltos pasan a la escala de 4 px.
2. **Color:** el acento legible (F9) para texto, foco, la píldora de selección y el progreso.
3. **Iconos:** la fuente de iconos de Windows en lugar de emoji y glifos de texto.
4. **Texto:** marcadores cortos, plurales de verdad y sin punto en etiquetas.
5. **Tipo:** la escala de Fluent, cabeceras con mayúscula inicial en seminegrita y cifras tabulares.
6. **Distribución:** sin cambios. La dirección mantiene la de cada pantalla, y las columnas de iconos y las alturas de fila ya coinciden.
7. **Detalles:** teclas en el pie y en la fila seleccionada, y la firma.
8. **Visuales:** el momento 2, abrir un Espacio como una secuencia viva de pasos. El momento 1 (el fragmento en la vista previa) llega con el bloque 7.
9. **Movimiento:** no queda ningún signo de unslop (ni `transition-all`, ni escalas al pasar el ratón, ni bucles). Lo pendiente de movimiento (F15–F17) es el bloque 5.
