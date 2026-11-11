# ADR 0007 — Qué se indexa: solo formatos útiles, y fuera lo que generan las herramientas

- **Estado:** aceptada
- **Fecha:** 2026-11-12

## Contexto

En la Fase 2 se indexaba **todo** lo que no estuviera en los patrones a ignorar: el contenido de textos y código, y el resto por nombre. Con una carpeta real de la universidad salieron 86.509 archivos, de los que solo 17.440 tenían contenido legible. Lo demás era ruido:

| Qué era | Archivos |
|---|---|
| `Library` de proyectos de Unity (cachés de shaders, paquetes y artefactos) | 35.160 |
| `.metadata` de espacios de trabajo de Eclipse | 22.020 |
| Carpetas de compilación (`bin`, `obj`) | 8.546 |
| Formatos que no sirven para buscar (`.meta`, `.jar`, imágenes, `.dll`, sin extensión…) | 8.190 |
| Ajustes de proyecto de editores (`.settings`) | 611 |
| **Útil: documentos, textos y código** | **11.888** |

Ese ruido ensucia los resultados (escribir «ajustes» devolvía antes código generado que el comando de Ajustes), hace la base de datos de 620 MB y alarga cada revisión.

## Decisión

### 1. Lista de formatos permitidos, no de formatos prohibidos

Solo se indexan textos, Markdown, código, documentos y —por nombre— imágenes y vídeos (`sift_extract::is_indexable`). Todo lo demás ni se guarda ni se cuenta: binarios, compilados, audio, comprimidos, archivos sin extensión (salvo nombres conocidos: `README`, `Makefile`, `Dockerfile`…).

Una lista de prohibidos nunca se acaba (cada herramienta inventa extensiones: `.meta`, `.hyb`, `.prefs`…); una de permitidos es corta y se amplía cuando hace falta.

Los `.log` salen de la lista: son salida de programas, no algo que se busque, y muchos son binarios.

### 2. Carpetas generadas: se reconocen por lo que las rodea

Además de los patrones, `sift_indexer::is_generated_dir` se salta:

- **Unity**: `Library`, `Temp`, `Obj`, `Logs`, `UserSettings` cuando al lado hay `Assets` y `ProjectSettings`.
- **Entornos virtuales de Python**: cualquier carpeta con `pyvenv.cfg`, se llame como se llame.
- **Cachés**: cualquier carpeta con `CACHEDIR.TAG`.

No se hace con un patrón `Library` porque ese nombre, a secas, podría ser una carpeta del usuario.

### 3. Más patrones por defecto

La migración v3 añade, sin tocar los que ya hubiera: compilación (`bin`, `obj`, `out`, `.next`, `.nuxt`, `.gradle`, `*.egg-info`), dependencias (`site-packages`, `.tox`, `bower_components`, `Pods`), y cachés y carpetas de trabajo de editores (`.cache`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.ipynb_checkpoints`, `.metadata`, `.settings`, `.vs`, `.history`, `.svn`, `.hg`). Siguen siendo editables en Ajustes → Carpetas → Avanzado.

### 4. Los motivos de «no se pudo leer» son categorías, no mensajes

`files.error_kind` guarda un código estable (`unsupported`, `tooLarge`, `noPermission`, `protected`, `damaged`, `other`) que sale de `ExtractError::reason()`. La UI los agrupa con su número y tres ejemplos. Los dos últimos los usará la extracción de PDF y Word.

## Consecuencias

- Lo que ya estaba indexado y deja de ser útil **se borra solo** en la siguiente revisión (el indexador lo trata como «ya no está»). Tras un borrado grande, la base de datos se compacta (`VACUUM`).
- Las imágenes y los vídeos se encuentran **por nombre** (`Kind::Media`; nunca se lee su contenido), salvo en carpetas ignoradas o generadas. Se puede desactivar en Ajustes → Carpetas → Avanzado (`indexing.skipMedia`). Se añadió tras probar la primera versión, en la que no se encontraban de ninguna forma.
- `bin` y `out` como patrones pueden esconder una carpeta del usuario que se llame así. Se asume: son nombres de compilación casi siempre, y el patrón se puede quitar en Avanzado.
- La detección por contexto cuesta dos o tres consultas al disco por carpeta al recorrer; frente a no leer 35.000 archivos, es despreciable.
