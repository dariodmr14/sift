# ADR 0011 — Espacios en los resultados y rutas sugeridas (Fase 6)

- **Estado:** aceptada
- **Fecha:** 2026-11-12

## Contexto

Hasta ahora la barra enseñaba los Espacios en su propia sección, siempre arriba, y los archivos debajo. La Fase 6 pide mezclarlos y que la frecencia (ADR 0010) cuente también para los Espacios, con una regla de Darío: **si lo que escribes es el nombre o un alias de un Espacio, ese Espacio va primero aunque lo uses poco; la frecencia decide solo cuando la coincidencia es parcial.**

El problema es comparar cosas de escalas distintas. Un Espacio encaja por nombre en 4 niveles (exacto, empieza por, principio de palabras, contiene). Un archivo tiene una puntuación de fusión RRF (~0,01–0,08).

## Decisión

### 1. Orden de la barra

1. **Coincidencia exacta** con el nombre o un alias: primero, sea cual sea su uso. Entre varios, por frecencia, y a igualdad, en el orden de Ajustes.
2. Los comandos de la app («Ajustes», «Nuevo Espacio»), si encajan.
3. **Archivos con los Espacios parciales intercalados.** Cada Espacio parcial recibe una puntuación en la escala de los archivos:

   ```text
   relevancia del mejor archivo (sin su frecencia) × peso de la coincidencia × frecencia del Espacio
   ```

   | Coincidencia | Peso | Ejemplo con «Bases de datos» |
   |---|---|---|
   | El nombre o un alias empieza por lo escrito | 1 | «bases d» |
   | Cada palabra empieza una palabra | 0,8 | «dat bas» |
   | Cada palabra aparece en algún sitio | 0,5 | «atos» |

   Se mete delante del primer archivo que puntúa menos que él. A igualdad, va primero el Espacio. **Los archivos no cambian de orden entre sí**: su orden ya lleva reglas propias, como «lo que nombras va primero».

Consecuencia práctica: un Espacio cuyo nombre empieza por lo que escribes empata con el mejor archivo. Si ese archivo lo abres mucho, gana el archivo; si el Espacio lo abres mucho, gana el Espacio. Es justo lo que pide la regla: la frecencia decide entre dos cosas igual de buenas.

La frecencia de los Espacios es la de los archivos: vida media de 14 días y empujón de hasta +25 % (ADR 0010). Ejecutar un Espacio cuenta como abrirlo, solo si se lanzó; no cuenta si se quedó esperando confirmación. `eval/` no tiene Espacios, así que los pesos 1 / 0,8 / 0,5 son un punto de partida razonado y se ajustarán con el uso real.

**Por qué la mezcla se hace en la UI y no en Rust:** los Espacios se filtran en TypeScript en cada tecla, al instante, mientras los archivos llegan de Rust unos milisegundos después. Mezclar en Rust haría esperar también a los Espacios. Rust manda lo que hace falta para comparar en la misma escala: el `boost` de cada archivo y de cada Espacio. La mezcla es una función pura (`launcher/mix.ts`) con sus tests.

**Cabeceras de sección:** solo cuando cada sección sale de un trozo. Si hay Espacios entre archivos, no se pintan cabeceras («Archivos · Espacios · Archivos» confunde) y cada Espacio dice «Espacio ·» en su subtítulo.

### 2. Rutas sugeridas en el editor de acciones

Al escribir la ruta de una acción «Abrir archivo» o «Abrir carpeta», o la carpeta de trabajo de un comando, salen sugerencias del índice. Se busca **por nombre**, no por contenido: al elegir qué abrir, lo que recuerdas es cómo se llama.

- Orden: el nombre empieza por lo escrito → cada palabra empieza una palabra del nombre → cada palabra aparece en la ruta («bd tema4» → `…\BD\tema4.pdf`). Dentro de cada nivel, frecencia y luego la ruta más corta.
- Las carpetas salen de las rutas de los archivos indexados, de todos los niveles.
- No se sugiere si ya hay una ruta completa (`C:\…`) ni con menos de 2 letras.
- El botón «Elegir…» sigue abriendo el diálogo de Windows, para lo que no está indexado.
- Con la carpeta real (12.871 archivos): ~60 ms leer las rutas de SQLite y 15–23 ms filtrarlas. Se pregunta 120 ms después de la última tecla. Si algún día molesta, la lista de rutas se puede guardar en memoria.

## Alternativas descartadas

- **Espacios siempre arriba, como antes.** Contradice «se mezclan con los resultados» y haría que un Espacio que encaja a medias tape siempre al archivo que buscas.
- **Tratar los Espacios como otra lista de la fusión RRF.** Los archivos suman hasta 3 fragmentos y dos recuperadores, así que un Espacio con un solo puesto quedaría casi siempre por debajo, incluso con el nombre casi exacto.
