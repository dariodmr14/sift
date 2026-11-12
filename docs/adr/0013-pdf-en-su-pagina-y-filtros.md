# ADR 0013 — PDF en su página, filtros rápidos y barra vacía

- **Estado:** aceptada
- **Fecha:** 2026-11-12

## Contexto

Un bloque de mejoras pedido antes de la Fase 6.5:

- Abrir el PDF en la página que encaja con la búsqueda, y decir cuál es.
- Filtrar por tipo escribiendo («pdf contrato»).
- Que la barra sin nada escrito sirva para algo.

## Decisión

### 1. La página de cada fragmento

- **Al extraer**, PDFium apunta dónde empieza cada página en el texto. Se cuenta en caracteres, como el `start` de los fragmentos. El texto no cambia: los fragmentos son los mismos y sus vectores (por hash del texto) se reutilizan.
- **Cada fragmento guarda la página de su mitad.** Su principio aún puede ser el final de la página anterior, porque los fragmentos se solapan 150 caracteres.
- **Migración v6:** columna `chunks.page`. Además, los PDF ya leídos se marcan para releerlos en segundo plano. No basta con cambiar la fecha: si el contenido coincide, Sift solo apunta la fecha nueva y no vuelve a trocear. Por eso también se borra su hash de contenido.
- **Al buscar**, el resultado lleva el id del fragmento que enseña. Para los PDF, la app lee su página de SQLite (una consulta por clave primaria por resultado) y la barra pone «pág. 37».

### 2. Abrir en esa página

Se pregunta a Windows qué programa abre los `.pdf` (`AssocQueryStringW`, lo mismo que usa «Abrir con»), y se le pasa la página a su manera:

| Lector | Cómo |
|---|---|
| Edge, Chrome, Brave, Firefox, Vivaldi | `file:///C:/…/tema.pdf#page=37` |
| Adobe Acrobat / Reader, Foxit, PDF-XChange | `/A "page=37" archivo` |
| SumatraPDF | `-page 37 archivo` |

- Programa y argumentos van separados, sin shell.
- Si el lector no se reconoce o algo falla, el PDF se abre normal: nunca es un error.
- Visto en el PC de Darío: la preferencia dice Acrobat, pero Acrobat ya no está instalado, y Windows abre los PDF con Chrome. Por eso se pregunta a Windows qué usa de verdad, en vez de leer la preferencia.

### 3. Filtros rápidos

- Una palabra clave al principio y un espacio se convierten en una etiqueta junto a la caja:
  - `pdf`
  - `word`
  - `powerpoint` / `diapositivas`
  - `excel`
  - `código`
  - `imagen` / `foto`
  - `vídeo`

  Retroceso al principio de la caja la quita.
- Se dejaron fuera palabras ambiguas, que también se buscan como contenido: «documento», «presentación», «notas».
- **El filtro va dentro de los recuperadores**, no después. En la búsqueda por palabras es una condición más sobre la extensión, que está indexada. En la búsqueda por significado se aplica al pasar de vectores a fragmentos. Filtrar al final dejaría pocos resultados: si entre los 100 mejores hay 3 PDF, «pdf …» enseñaría 3.
- Rust decide qué extensiones tiene cada filtro (`sift_extract::type_filter`, que ya conoce los tipos). La UI decide qué palabras lo activan. Ajustes las enseña a partir de la misma lista.

### 4. Barra vacía

Sin nada escrito, la barra enseña «Espacios» y «Abierto recientemente»: los 8 últimos archivos abiertos desde Sift, sin repetir y solo los que siguen en el índice. Se ordenan por el id de la apertura, no por la hora, porque dos aperturas en el mismo milisegundo empatarían.

No cuenta en los < 100 ms: la lista se pide después de pintar la barra y se guarda entre aperturas. La barra se pinta con la de la vez anterior y se pone al día sola.

## Alternativas descartadas

- **Guardar la página en el índice de Tantivy.** Obligaría a reconstruir el índice entero, y con leer de SQLite las páginas de 12 resultados basta.
- **Separar las páginas con una marca dentro del texto.** Cambiaría el texto de cada fragmento y obligaría a recalcular todos los vectores de los PDF.
