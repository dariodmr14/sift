# ADR 0010 — Ranking híbrido y medición (Fase 5)

- **Estado:** aceptada
- **Fecha:** 2026-11-11

## Contexto

La Fase 4 dejó la búsqueda híbrida en 0,65 de recall@10 (MRR 0,56) con las 34 consultas de `eval/`, y un criterio sin cumplir: «cómo evitar redundancia en tablas» no encontraba los apuntes de normalización. Faltaban también la frecencia, ajustar los pesos con datos y un benchmark reproducible.

Todo lo de este ADR se decidió midiendo: con `eval/` (calidad, sobre una copia de la base de datos real) y con el benchmark nuevo (latencia, sobre un corpus sintético). Las herramientas de diagnóstico están en `crates/search/tests/eval.rs` (`diagnose`, `hnsw_recall`, `calibrate_weights`, `calibrate_frecency`).

## Diagnóstico de «cómo evitar redundancia en tablas»

Datos, no suposiciones (`diagnose`):

1. **El modelo apenas distingue.** El parecido de los vecinos va de 0,878 (1.º) a 0,861 (10.º), 0,852 (40.º) y 0,839 (400.º): todo está muy junto.
2. **HNSW perdía al bueno.** Un fragmento de `apuntes_bd_completos.pdf` estaba el 14.º en la búsqueda exacta, pero HNSW (con `ef = 64`) no lo devolvía entre sus 40 vecinos. Medido en todas las consultas, HNSW acertaba el 88 % de los 40 vecinos de verdad.
3. **El fragmento de normalización no estaba mal troceado, pero era técnico.** «Eliminar Ri redundantes… BCNF, 3FN» está el 42.º; el mejor que dice «normaliz…», el 262.º. El docx de `TEST_B` tiene su mejor fragmento el 148.º.
4. **Descartado: copias que tapan.** Los 40 vecinos son 59 fragmentos; no hay un texto repetido que acapare los puestos.
5. **La causa grande: las palabras no aportaban nada.** La búsqueda por palabras exige *todas* las palabras en el mismo fragmento. Ningún fragmento tiene «evitar», «redundancia» y «tablas» a la vez, así que devolvía **cero** resultados y la fusión solo veía la semántica.

## Decisiones

### 1. HNSW busca con más esfuerzo (`ef` 64 → 512)

| `ef` | recall@40 frente a la búsqueda exacta | tiempo |
|---|---|---|
| 64 | 0,88 | 0,24 ms |
| 128 | 0,94 | 0,37 ms |
| 256 | 0,98 | 0,78 ms |
| **512** | **0,99** | **1,29 ms** |
| 1024 | 1,00 | 2,57 ms |
| exacta (sin índice) | 1,00 | 11,45 ms |

Con 512, el archivo esperado pasa a estar el 12.º. Mejora, pero no basta.

### 2. Búsqueda de palabras «relajada» para frases

Con tres palabras o más, si no hay suficientes fragmentos con todas, detrás van los que tienen **todas menos una**. Lo primero siguen siendo los que lo tienen todo. En la consulta relajada no se buscan erratas, solo raíces exactas.

Rejilla con `eval/` (híbrida, total; con 100 candidatos semánticos y la suma por archivo):

| Relajar | recall@10 (MRR) | p95 |
|---|---|---|
| no (Fase 4 + decisiones 1 y 3) | 0,71 (0,60) | 37,5 ms |
| desde 2 palabras, falta 1 | 0,78 (0,66) | 55,8 ms |
| desde 3 palabras, falta 1, con erratas | 0,78 (0,65) | 66,8 ms |
| desde 3 palabras, faltan 2 | 0,74 (0,59) | 56,5 ms |
| **desde 3 palabras, falta 1, sin erratas** | **0,75 (0,66)** | **43,7 ms** → 29,7 ms en paralelo (decisión 4) |

- Con dos palabras, «falta una» es «cualquiera de las dos»: se descarta.
- Las erratas, con la consulta relajada, abren miles de documentos por palabra. Dan una consulta más de las 34 por +20 ms de p95; incluso en paralelo dejan la p95 en 47,5 ms, al borde del objetivo.

**Por qué ahora sí, si en la Fase 4 «alguna de las palabras» no mejoraba nada:** aquello era un tercer recuperador aparte, con todas las palabras opcionales (BM25 puro) y menos peso. Esto va en la misma lista, detrás de los resultados completos, y exige casi todas las palabras: es una búsqueda de palabras que no se rinde, no ruido.

### 3. La puntuación de un archivo suma sus mejores fragmentos

Antes, un archivo valía lo que su mejor fragmento. Ahora suma sus 3 mejores, y la semántica pide 100 vecinos en vez de 40 (con 40 casi nunca entra un segundo fragmento del mismo apunte).

| Fragmentos · candidatos | recall@10 (MRR) |
|---|---|
| 1 · 40 (Fase 4) | 0,65 (0,54) |
| 2 · 100 | 0,69 (0,57) |
| **3 · 100** | **0,71 (0,60)** |
| 5 · 100 | 0,71 (0,58) |

### 4. Los recuperadores van en paralelo

Las palabras y la semántica (calcular el vector de la consulta + HNSW) tardan parecido; ahora la búsqueda tarda lo que el más lento y no la suma. p95 de 43,7 a 29,7 ms con la misma calidad.

### 5. Frecencia

Cada vez que abres un archivo desde Sift suma un punto que pierde la mitad de su valor cada **14 días**. La suma se convierte en un empujón con techo:

```text
puntuación final = puntuación de la fusión × (1 + 0,25 × s / (s + 3))
```

- **Solo reordena**: no añade archivos que no encajan con la búsqueda, por mucho que los abras.
- **Tiene techo** (+25 %): no adelanta a un resultado mucho mejor.

Hay solo 4 aperturas reales, así que el peso se calibró **simulando** sobre `eval/`, con dos casos que tiran en sentidos opuestos:

| Peso | sin aperturas | abriste el bueno (3 veces esta semana) | abriste mucho otros 5 que salen |
|---|---|---|---|
| 0 | 0,75 (0,66) | 0,75 (0,66) | 0,75 (0,66) |
| **0,25** | 0,75 (0,66) | 0,75 (**0,69**) | **0,75** (0,66) |
| 0,5 | 0,75 (0,66) | 0,75 (0,71) | 0,72 (0,65) |
| 1 | 0,75 (0,66) | 0,75 (0,71) | 0,70 (0,61) |

Con 0,25 ayuda sin coste; con 0,5 el ruido ya echa resultados buenos del top 10. Cuando haya meses de aperturas reales, se puede recalibrar con ellas.

En la app, la frecencia vive en memoria y se recalcula al abrir algo y cada hora (las aperturas envejecen); no se consulta SQLite en cada búsqueda.

### 6. Empates por id

Las copias de un mismo texto en varias carpetas puntúan igual, y Tantivy desempata por su orden interno, que cambia al rehacer el índice. Por eso el MRR de `eval/` bailaba ±0,02 entre ejecuciones. Ahora se desempata por id de archivo y de fragmento.

### 7. Benchmark reproducible y lo que encontró

`cargo bench -p sift-search --bench search` construye un corpus sintético determinista del tamaño de la carpeta real (100.000 fragmentos, 30.000 con vector) y mide cada pieza con `criterion`, más la p50/p95 de la búsqueda completa. Usa e5-small de verdad si está descargado.

El primer resultado sorprendió: las piezas sumaban ~10 ms, pero la búsqueda completa tardaba 26 ms. **Resaltar** los resultados costaba 12,8 ms: se creaba un analizador nuevo por cada palabra del texto, se calculaba la distancia de erratas con cada palabra aunque la longitud ya lo descartara, y se resaltaban 24 resultados para enseñar 12. Arreglado: 5,3 ms, y la híbrida pasó de p50 26,0 / p95 32,6 ms a p50 13,3 / p95 17,0 ms.

## Alternativas descartadas (por ahora)

- **Reordenar con un *reranker* local** (un modelo que lee consulta y fragmento juntos). Es lo que más suele mejorar la precisión, pero se estima (sin medir) en ~5–10 ms por fragmento en este CPU: reordenar 20 se comería el presupuesto de 50 ms. Al backlog, para medirlo si las ideas siguen fallando.
- **Dar a cada fragmento el contexto de su documento** (título, asignatura) al calcular su vector. Obliga a recalcular los 29.477 vectores y cambia cómo se identifican (hoy por el hash del texto). Al backlog.
- **Un umbral de parecido.** La rejilla lo vuelve a confirmar: no mejora (el mejor es 0).

## Consecuencias

- La híbrida pasa de 0,65 (0,56) a 0,75 (0,66) de recall@10 (MRR), con las exactas en 1,00 y la p95 más baja que en la Fase 4.
- «cómo evitar redundancia en tablas» trae `apuntes_bd_completos.pdf` el 2.º.
- Los pesos son constantes calibradas con nombre y número de su experimento. Recalibrar es cambiar la constante y volver a lanzar `eval/`.
- Pendiente de tu uso real: ampliar `eval/` con las consultas que fallen.
