# ADR 0009 — Búsqueda por significado (Fase 4)

- **Estado:** aceptada (2026-11-13), con la regla de elección del modelo revisada por Darío
- **Fecha:** 2026-11-13

## Contexto

Objetivo: buscar «bases de datos» y que salga todo lo relacionado (normalización, SQL, transacciones, las prácticas de la asignatura) aunque no contenga esas palabras. Hoy la búsqueda es solo por palabras (Tantivy, ADR 0006).

La arquitectura ya está preparada: la unidad es el fragmento, hay un trait `Retriever` y fusión RRF. Falta el recuperador semántico y todo lo que necesita.

**Los datos reales** (carpeta de la carrera, medidos en solo lectura):

| | |
|---|---|
| Archivos indexados | 12.871 (11.819 con contenido) |
| Fragmentos | 101.663 (media 1.063 caracteres) |
| …en documentos (PDF, Word, PowerPoint) | 12.658 |
| …en apuntes y texto (`.md`, `.txt`) | 7.506 útiles; el resto son volcados (un `clarin.txt` de 2.517 fragmentos, repetido 5 veces) |
| …en código | 29.755 útiles |
| …en datos (`.xml`, `.json`, `.html`…) | 34.397, la mitad en 6 archivos (`google_10k.xml`: 4.972 fragmentos) |
| Fragmentos con texto **repetido** | 40 %: las mismas prácticas están en varias carpetas |

**El PC:** i7-7700K (4 núcleos, 8 hilos, AVX2 sin AVX-512), 32 GB, RTX 3060.

Lo que no se pudo medir sin escribir código (velocidad real de cada modelo en este PC, calidad en estas consultas) va marcado como **estimación** y es lo primero que se mide al implementar (ver «Orden de trabajo»).

## 1. Modelo de embeddings

### Candidatos

| | `multilingual-e5-small` | `granite-embedding-97m-multilingual-r2` | `embeddinggemma-300m` |
|---|---|---|---|
| Quién y cuándo | Microsoft, 2022 | IBM, mayo de 2026 | Google, septiembre de 2025 |
| Parámetros | 118 M | 97 M | 308 M |
| Dimensiones | 384 | 384 | 768 (recortable a 256) |
| Texto máximo | 512 tokens | 32.768 tokens | 2.048 tokens |
| Recuperación multilingüe (MTEB, nDCG@10) | 50,9 | 60,3 (59,6 según su README) | 62,5 |
| Recuperación en español | 51,0 | sin dato publicado | sin dato publicado |
| Velocidad relativa (tabla de IBM) | 2.604 docs/s | sin dato | 1.349 docs/s |
| Tamaño en disco (ONNX cuantizado) | 118 MB | 98 MB, con versión para AVX2 | 197–309 MB |
| Licencia | MIT | Apache 2.0 | Gemma (condiciones de uso de Google) |

Las cifras vienen de fuentes distintas y no son comparables al decimal; sirven para ordenar.

- **`bge-m3`** (568 M) y **`snowflake-arctic-embed-m-v2.0`** (305 M, 54,8) se descartan: pesan el triple que los pequeños y no superan a Granite.
- **EmbeddingGemma** es la de mejor nota, pero queda fuera como modelo por defecto: es 3 veces mayor (tardaría horas en este PC), y su licencia no es libre: imponer las condiciones de uso de Google a quien use Sift, un proyecto MIT, no compensa 2 puntos.

### Propuesta

**`granite-embedding-97m-multilingual-r2`, con `multilingual-e5-small` como alternativa, y que decida `eval/`.**

Granite saca 9 puntos más que e5-small con el mismo tamaño de vector, pesa menos, es Apache 2.0 y publica un ONNX cuantizado para AVX2, que es justo lo que tiene este procesador. Pero es de este año, su base (ModernBERT) es más nueva y no hay cifra publicada en español. e5-small es el valor seguro: cuatro años de uso y 9 millones de descargas al mes.

Los dos dan vectores de 384 dimensiones, así que todo lo demás (tabla, índice, fusión) es idéntico. Por eso la decisión se puede tomar **midiendo**: el primer paso de la implementación calcula los vectores de `eval/` con los dos y elige con esta regla:

> **Decide la calidad**: el modelo con mejor recall@10 y MRR en `eval/` (híbrida), siempre que la búsqueda completa, **incluido calcular el vector de la consulta**, se quede por debajo de 50 ms de p95. Si Granite gana en calidad y cumple esa latencia, Granite.

La velocidad de vectorizar en segundo plano **no decide**: es una sola vez, va con prioridad baja y no se nota (revisión de Darío al aprobar el diseño; la regla anterior también descartaba un modelo más de un 40 % más lento al vectorizar).

### `fastembed` frente a `ort` directamente

| | `fastembed` 7.7 | `ort` 2.0 + `tokenizers` |
|---|---|---|
| Código propio | ~20 líneas | ~200 (tokenizar, ejecutar, agrupar, normalizar) |
| Granite R2 | No está en su lista; habría que usar su modo «modelo propio» | Cualquier ONNX |
| Descarga del modelo | La hace él, de Hugging Face, sin fijar hash ni versión, con su propia caché | La hacemos nosotros: URL y SHA-256 fijados, como `pdfium.dll` |
| Control de CPU | Limitado | Número de hilos de inferencia: es el límite de CPU |
| Dependencias | Trae modelos de imagen y un cliente de Hugging Face que no usamos | Las justas |

**Propuesta: `ort` directamente**, detrás de un trait `Embedder`. Lo que ahorra `fastembed` es poco código, y a cambio se pierde lo que más importa aquí: descargar una sola vez un archivo verificado, y decidir cuánta CPU se usa. `fastembed` fija además una versión exacta de `ort`, así que tampoco evita esa dependencia.

ONNX Runtime se enlaza con la opción `download-binaries` de `ort` (baja al compilar la librería precompilada que corresponde a esa versión del crate). Si resultara necesitar una DLL aparte, se empaqueta como `pdfium.dll` (ADR 0008).

### Descarga del modelo

Es la única conexión a internet que permite el proyecto. Se hace una vez, en segundo plano, con el archivo y su SHA-256 fijados en el código, reanudable, a `%APPDATA%\dev.sift.app\models\<id>\`. Mientras no esté, Sift busca por palabras como hasta ahora. Carpetas lo cuenta («Descargando la búsqueda inteligente · 98 MB») y hay un interruptor «Búsqueda inteligente» en Ajustes para quien no la quiera.

## 2. Troceado

**Se mantiene el de la Fase 2** (~1.200 caracteres, 150 de solape, cortando en párrafos): son 300–400 tokens de prosa en español, dentro de los 512 de e5-small, y es buen tamaño para recuperar (ni frases sueltas ni páginas enteras). Cambiarlo obligaría a rehacer el índice de palabras sin una razón medida.

Tres ajustes, solo para los vectores:

1. **No todo se vectoriza.** Documentos, apuntes y código, sí. Los formatos de datos (`.xml`, `.json`, `.html`, `.csv`…) no: son volcados y configuración, donde «significado» no aporta y sí mete ruido. Siguen encontrándose por palabras.
2. **Tope por archivo**: los primeros 1.000 fragmentos de un documento (un libro entero) y los primeros 100 de un texto o un código. Deja fuera los volcados de miles de fragmentos.
3. **Un vector por texto distinto.** El 40 % de los fragmentos están repetidos; se calcula una vez por contenido (hash del texto) y lo comparten todas las copias.

Con eso, de 101.663 fragmentos se calculan **29.994 vectores**.

El código tokeniza más denso que la prosa y algún fragmento pasará de 512 tokens: se corta ahí y se mide cuántos; si pasan del 5 %, se baja el tamaño del fragmento solo para código.

Queda como **experimento medible con `eval/`**, no como decisión: anteponer al texto una cabecera con la carpeta y el nombre («BD / Prácticas / practica3.sql»). Ayudaría a que «bases de datos» encuentre las prácticas de la asignatura, pero anula el ahorro del punto 3 (cada copia tendría un texto distinto). Se prueba y se queda solo si sube el recall.

## 3. Índice vectorial

**usearch (HNSW)**, como fijó el ADR 0001.

| Parámetro | Valor | Por qué |
|---|---|---|
| Métrica | Coseno (vectores normalizados) | Es como se entrenan estos modelos |
| Precisión | `f16` | La mitad de memoria que `f32` sin pérdida apreciable |
| Conectividad (M) | 16 | El valor habitual; más solo compensa con millones de vectores |
| `ef` al construir | 128 | Calidad del grafo; se construye en segundo plano, puede ser generoso |
| `ef` al buscar | 64 | Sobra para pedir 50 vecinos |
| Clave | id del vector (una fila de SQLite) | Lo comparten todos los fragmentos con ese texto |

**SQLite sigue siendo la fuente de verdad** (migración v5):

- `embedding_models`: los modelos conocidos (id, nombre, dimensiones, hash del archivo).
- `embeddings`: `id`, `model_id`, `text_hash`, `vector` (384 valores `f16` = 768 bytes), único por (modelo, hash).
- `chunks.text_hash`: de qué texto es cada fragmento, para llegar a su vector.

**Persistencia:** el índice es un archivo (`vectors-<modelo>.usearch`) junto al de Tantivy, que se guarda cada cierto número de vectores y al terminar. Es **derivado**: si falta, está corrupto, es de otro modelo o no cuadra con el número de filas de SQLite, se reconstruye desde `embeddings` sin recalcular nada (30.000 vectores: segundos). Es el mismo trato que tiene Tantivy, y lo caro (calcular los vectores) nunca se pierde. Un cierre a mitad se recupera igual que en la Fase 3: los vectores se guardan en SQLite por lotes y al arrancar se reanuda con los que falten.

**Borrados:** un vector se quita cuando ningún fragmento usa ya su texto.

Con 30.000 vectores, una búsqueda exacta (comparar con todos) tardaría unos milisegundos y sería más simple. Se usa HNSW porque el índice tiene que aguantar carpetas diez veces mayores sin cambiar de diseño; la búsqueda exacta queda en los tests, como referencia para medir cuánto se equivoca HNSW.

## 4. Combinar con la búsqueda por palabras

Las dos búsquedas van en paralelo y se funden con **RRF ponderado**: cada resultado suma `peso / (60 + posición)` por cada lista en la que aparece. Se agrupa por archivo, como ahora.

El riesgo es que la semántica **siempre devuelve algo**: para «shell-v4.3» traería 50 fragmentos «parecidos» que ensucian un resultado que las palabras clavan. Cuatro defensas:

1. **Tipo de consulta.** Reglas simples, sin modelo:
   - *Parece un nombre o un código* (lleva extensión, dígitos pegados a letras, guiones o guiones bajos, barras, mayúsculas internas, o va entre comillas): **solo palabras**.
   - *Muy corta* (menos de 3 letras): solo palabras; es alguien tecleando.
   - *Lenguaje natural* (dos o más palabras): híbrida.
   - *Una sola palabra normal*: híbrida, con la semántica a medio peso.
2. **Umbral de parecido.** Un vecino por debajo de cierta similitud no entra, aunque sea «el más parecido que hay». El umbral depende del modelo y se calibra con `eval/` (las consultas `exact` no deben traer resultados semánticos; las `concept`, sí).
3. **Las coincidencias exactas mandan.** Si todas las palabras de la consulta están en el nombre de un archivo, ese archivo va por delante de cualquier resultado que solo llegue por significado.
4. **Se nota de dónde viene.** Un resultado que solo llega por significado no tiene palabras que resaltar: muestra el principio de su fragmento y una marca discreta, para que se entienda por qué está ahí.

Pesos de partida: palabras 1,0 y semántica 0,7. Se ajustan con `eval/`, vigilando que las consultas `exact` no bajen del resultado de «solo palabras».

Si el modelo aún no está (descargándose, o desactivado), la búsqueda es la de siempre: el recuperador semántico simplemente no participa.

## 5. Rendimiento

Todo son **estimaciones** hasta medir en este PC.

**Calcular los vectores (una vez, en segundo plano).** Va en la cola de la Fase 3, con la prioridad más baja: primero se indexa por palabras, después se vectoriza. Mismo modo de segundo plano de Windows, misma pausa si el PC está ocupado, y 2 hilos de inferencia de los 8.

| | Estimación |
|---|---|
| Velocidad con el límite de CPU | 10–20 fragmentos por segundo |
| Los 29.994 vectores | 25–50 minutos de trabajo en segundo plano |
| Primero | Documentos y apuntes (lo que más se busca por significado), de lo más reciente a lo más antiguo; el código, después |

La búsqueda por significado funciona desde el primer vector: mejora según avanza. Lo que se guarda o edita después se vectoriza al momento (son pocos fragmentos).

La RTX 3060 lo haría en un par de minutos con DirectML. Se deja para después: obliga a otra variante de ONNX Runtime y la mayoría de los PC no tienen esa gráfica.

**Memoria.**

| | Estimación |
|---|---|
| Modelo cargado | 150–250 MB |
| Índice (30.000 × 384 × `f16` + grafo) | ~30 MB |
| En SQLite | ~25 MB más |

El modelo se carga al arrancar, en segundo plano; con la búsqueda inteligente desactivada, no se carga.

**Latencia por búsqueda** (objetivo: p95 < 50 ms, medido como en la Fase 2).

| Paso | Estimación |
|---|---|
| Vector de la consulta (pocas palabras) | 4–10 ms |
| Vecinos en HNSW | < 1 ms |
| Palabras (en paralelo; hoy p95 12 ms) | 12 ms |
| Fusión y agrupado | < 1 ms |
| **Total** | **15–25 ms** |

Si el vector de la consulta tardara más de lo previsto, hay margen: la consulta se calcula con todos los hilos (es un instante) y se cachea la última.

## 6. Evaluación

[`eval/`](../../eval/README.md) ya existe: 34 consultas escritas mirando qué asignaturas y temas hay en la carpeta, con sus resultados esperados comprobados contra el índice.

- 26 `concept`: la idea sin las palabras del documento («dos procesos que se quedan esperando el uno al otro» → el tema de concurrencia).
- 3 `topic`: el tema tal cual («bases de datos»).
- 5 `exact`: nombres y códigos («shell-v4.3»). Son la red de seguridad contra el ruido.

Métrica: **recall@10** (de lo esperado, cuánto sale entre los 10 primeros), global y por tipo, más MRR. Se compara «solo palabras», «solo semántica» e «híbrida» sobre el mismo índice. El README llevará esa tabla y la latencia.

**Criterio de «hecho» de la fase**, medible:

- Híbrida mejor que solo palabras en las `concept` por un margen claro.
- Híbrida **no peor** que solo palabras en las `exact`.
- p95 < 50 ms.
- «cómo evitar redundancia en tablas» encuentra los apuntes de normalización.

## 7. Cambiar de modelo sin perder nada

- Cada vector lleva su `model_id`, y cada modelo tiene su propio archivo de índice. Dos modelos pueden convivir.
- Un modelo es una ficha en el código: URL, SHA-256, dimensiones, cómo se agrupa, prefijos de consulta y de documento, umbral de parecido. Añadir uno es añadir una ficha.
- **Cambio en caliente:** el modelo nuevo se descarga y va calculando sus vectores en segundo plano mientras el viejo sigue respondiendo. Cuando el nuevo cubre todo, se cambia y se borran los vectores del viejo. En ningún momento se queda sin búsqueda por significado.
- El texto de los fragmentos está en SQLite: nunca hay que volver a leer los archivos.
- `eval/` dice si el cambio mereció la pena antes de hacerlo definitivo.

## Orden de trabajo (cuando haya visto bueno)

1. **Medir antes de construir**: `Embedder` con `ort`, los dos modelos sobre `eval/`, velocidad en este PC. Sale la elección de modelo y las cifras reales de la sección 5. Si algo se desvía mucho de lo estimado, se para y se revisa este ADR.
2. Migración v5 y vectores en SQLite (con el ahorro por texto repetido).
3. Índice usearch: construir, guardar, reconstruir desde SQLite.
4. Trabajo de vectorizar en la cola de la Fase 3 y descarga del modelo.
5. Recuperador semántico, tipo de consulta, umbral y RRF ponderado.
6. UI: estado en Carpetas («Preparando búsqueda inteligente · 3.200 de 13.000»), marca en los resultados por significado, interruptor en Ajustes.
7. `eval/` como test, tabla en el README, PLAN.md.

## Qué cambia respecto a PLAN.md

- La fusión RRF y `eval/` estaban en la Fase 5; se adelantan a la 4, porque sin ellas no se puede decidir ni comprobar nada de lo anterior. En la Fase 5 queda la frecencia y el ajuste fino.
- El modelo deja de ser `multilingual-e5-small` «porque sí»: lo decide la medición.

## Consecuencias

- Primera vez que Sift usa la red al ejecutarse: una descarga, verificada, de ~100 MB. Estaba prevista en las reglas del proyecto.
- El instalador crece lo que ocupe ONNX Runtime (decenas de MB). El modelo no va dentro.
- Los formatos de datos no se buscan por significado. Si se echa de menos, es cambiar una lista.
- Granite es un modelo de este año: si diera problemas, e5-small entra sin tocar nada más.
