# ADR 0008 — Cómo se lee el PDF y cómo llega `pdfium.dll` al usuario

- **Estado:** aceptada
- **Fecha:** 2026-11-12

## Contexto

La Fase 3 lee el contenido de los PDF. El plan ya eligió **PDFium** (el motor de PDF de Chrome, crate `pdfium-render`) por fiabilidad: los extractores puros en Rust fallan con muchos PDF reales (fuentes incrustadas, texto en columnas, PDF generados por LaTeX o escaneados con capa de texto).

PDFium es C++ y `pdfium-render` no lo compila: necesita una **`pdfium.dll`** ya hecha en tiempo de ejecución. Google no publica binarios; el proyecto [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) los compila en GitHub Actions desde el código de Chromium y es la fuente que recomienda `pdfium-render`.

Hay que decidir cómo llega esa DLL al PC del usuario, respetando dos reglas del proyecto: **todo es local** (nada de red salvo el modelo de embeddings la primera vez) y que la CI sea reproducible.

## Opciones

| Opción | A favor | En contra |
|---|---|---|
| **A. Dentro del instalador**, como recurso de Tauri; la DLL se baja al compilar con una versión y un hash fijados | La app nunca usa la red. Funciona sin conexión desde el primer momento. Versión controlada | El instalador pesa unos 4 MB más. Quien compila necesita ejecutar un script una vez |
| B. Descargarla la primera vez que hace falta | Instalador más pequeño | Rompe «todo es local». Falla sin conexión o tras un proxy. Hay que gestionar reintentos y verificación en el PC del usuario |
| C. Subir la DLL al repositorio | Cero pasos al clonar | Un binario de varios MB en git para siempre, y otro por cada actualización. Revisar un cambio de versión es revisar un blob |
| D. Enlazado estático (`pdfium-render` con `static`) | Un solo `.exe` | Hay que compilar PDFium (horas, toolchain de Chromium) o depender de `.lib` precompiladas difíciles de encontrar para MSVC |
| E. Extractor puro en Rust (`pdf-extract`, `lopdf`) | Sin DLL | Menos fiable, que es justo por lo que se eligió PDFium |

## Decisión

**Opción A.**

1. **Versión fijada y verificada.** [`scripts/fetch-pdfium.ps1`](../../scripts/fetch-pdfium.ps1) baja `pdfium-win-x64.tgz` de la release `chromium/8125` y comprueba su **SHA-256** (`9a964261…356b8b`) antes de extraer nada. Si no coincide, se borra y falla. La versión es la misma que la feature `pdfium_latest` de `pdfium-render` 0.9.5, para que los enlaces de Rust y la DLL hablen exactamente la misma API.
2. **Fuera de git, dentro del instalador.** La DLL va a `apps/desktop/src-tauri/pdfium/` (ignorada por git; la carpeta sí está, con un README). `bundle.resources` empaqueta esa carpeta, así que el instalador la lleva y en ejecución está en `<recursos>/pdfium/pdfium.dll`. Junto a ella va la licencia de PDFium (`LICENSE-pdfium.txt`; BSD de 3 cláusulas y Apache 2.0).
3. **Carga explícita.** `sift_extract::pdf::Engine::load` carga la DLL **por su ruta completa** dentro de los recursos de la app, no por nombre: Windows no busca en el `PATH` ni en la carpeta actual, así que no se puede colar otra `pdfium.dll` (secuestro de DLL).
4. **Si falta, se degrada.** Sin DLL (alguien compiló sin ejecutar el script), la app arranca igual, los PDF se indexan por nombre y Ajustes → Carpetas lo dice. Los tests que necesitan PDFium se saltan si no está.
5. **CI.** El job que compila la app ejecuta el script antes de `tauri build`. Es la única descarga, ocurre al compilar y está verificada.

## Consecuencias

- Sift sigue sin usar la red al ejecutarse.
- Actualizar PDFium (parches de seguridad de Chromium) es cambiar dos líneas del script y la feature de `pdfium-render`, en un PR que se puede revisar. Conviene hacerlo de vez en cuando: PDFium procesa archivos que vienen de fuera.
- El instalador crece unos 4 MB (lo que ocupa la DLL comprimida).
- Solo hay binario para Windows x64, que es el único objetivo del proyecto. Para ARM64 habría que añadir su `.tgz` y su hash.
- Los PDF escaneados sin capa de texto salen sin contenido (se encuentran por nombre): leerlos necesitaría OCR, que queda en el backlog.
- Confiamos en los binarios de `pdfium-binaries`. Se mitiga fijando el hash: lo que se probó es lo que se distribuye, y un binario cambiado bajo la misma etiqueta no pasaría la comprobación.
