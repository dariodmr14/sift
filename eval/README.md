# eval/ — ¿busca bien Sift?

Un conjunto pequeño de consultas reales con sus resultados esperados, para **medir** cada cambio en la búsqueda en vez de fiarse de la impresión. Decide el modelo de embeddings, los pesos de la fusión y cualquier ajuste futuro (ver [ADR 0009](../docs/adr/0009-busqueda-por-significado.md)).

## Las consultas

[`queries.json`](queries.json): 34 consultas sobre una carpeta real de la carrera (Ingeniería Informática del Software). Se escribieron mirando qué asignaturas y temas hay indexados, y cada resultado esperado se comprobó contra el índice: el documento existe y trata de eso.

Tres tipos, porque fallan de formas distintas:

| Tipo | Qué prueba | Ejemplo | Quién debería ganar |
|---|---|---|---|
| `concept` (26) | La idea, **sin** usar las palabras del documento | «cómo evitar redundancia en tablas» → apuntes de normalización | La semántica |
| `topic` (3) | Un tema nombrado tal cual | «bases de datos» | Las dos |
| `exact` (5) | Un nombre de archivo o un código | «shell-v4.3» | Las palabras. **La semántica no debe estropearlas** |

Las `exact` son la red de seguridad: una búsqueda híbrida que mejora las `concept` a costa de romper las `exact` no vale.

## Cómo se puntúa

Cada consulta tiene una lista `expect` de patrones (globs sobre la ruta relativa a la carpeta indexada, sin distinguir mayúsculas). Un patrón está **cumplido** si alguno de los 10 primeros archivos devueltos encaja con él.

- **recall@10** de una consulta = patrones cumplidos / patrones esperados. La métrica es la media de todas las consultas (y por tipo).
- **MRR** (como desempate): 1 / posición del primer resultado que cumple algún patrón.

Son patrones y no rutas exactas porque el mismo apunte suele estar repetido en varias carpetas: cualquiera de las copias vale.

Se comparan tres configuraciones sobre el mismo índice: **solo palabras** (lo que hay desde la Fase 2), **solo semántica** e **híbrida**.

## Cómo se ejecuta

Tests `#[ignore]` que trabajan sobre una **copia** de la base de datos real (en `target/eval/`, hecha con `VACUUM INTO`) y el modelo descargado:

```bash
cargo test -p sift-search --release --test eval -- --ignored recall_at_10 --nocapture
```

- `recall_at_10…`: la tabla (palabras / semántica / híbrida), la latencia y los fallos. Los vectores calculados se quedan en la copia: la segunda vez tarda segundos.
- `calibrate_weights`: prueba pesos de la semántica en la fusión.
- `show_one` (con `SIFT_EVAL_QUERY="…"`): los 10 primeros de una consulta y lo que trae la semántica sola.

Resultados actuales: en el README y en el ADR 0009.

## Privacidad

Las consultas y los patrones nombran asignaturas y apuntes de clase, no datos personales: se dejaron fuera a propósito los archivos con nombre o identificador de alumno. Los resultados dependen de tener esa carpeta; con otra, hay que escribir otro `queries.json` con este mismo formato.
