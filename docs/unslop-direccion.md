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

Cuando la apruebes (o la cambies), se aplican las nueve pasadas en este orden: espaciado, color, iconos, texto, tipo, distribución, detalles, visuales y movimiento.
