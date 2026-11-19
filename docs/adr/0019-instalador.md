# ADR 0019 — Instalador con la imagen de Sift

- **Estado:** aceptada
- **Fecha:** 2026-11-19

## Contexto

El instalador era el de Tauri tal cual: icono genérico, sin imágenes, en inglés o en el idioma que eligiera NSIS y con «Ejecutar Sift» al final. Darío pidió uno propio (FEEDBACK 2026-11-19, bloque 8): icono de Sift, imágenes en el estilo Vidrio, textos en español y «Abrir Sift» al acabar.

## Decisión

Aprobada por Darío tras ver las imágenes, el 2026-11-19. Se personaliza dentro de lo que permite la configuración de NSIS de Tauri, sin plantilla propia:

- **Icono** del instalador: el de Sift (`icons/icon.ico`).
- **Imagen lateral** (164 × 314, bienvenida y final): el fondo oscuro del icono, el tamiz con su última barra en el acento, «Sift», «Busca y abre lo tuyo» y «Todo se queda en tu PC».
- **Cabecera** (150 × 57, páginas interiores, a la derecha): el tamiz sobre blanco.
- **Solo en español**, sin selector de idioma.
- **Textos** (`installer/hooks.nsh`, que Tauri incluye antes de definir las páginas): bienvenida, final y «Abrir Sift».

Las imágenes se generan con `scripts/instalador-imagenes.ps1` (System.Drawing, sin dependencias) en `apps/desktop/src-tauri/installer/`.

| Opción | A favor | En contra |
|---|---|---|
| **Configuración de Tauri + hooks** | Se actualiza con Tauri sin tocar nada; poco que mantener | Ventana clásica de NSIS |
| Plantilla `.nsi` propia | Control total de páginas y textos | Hay que fusionar a mano cada cambio de la plantilla de Tauri |
| Instalador propio (otra herramienta) | Podría tener vidrio y tema oscuro | Mucho trabajo; perder las actualizaciones de Tauri |

## Consecuencias

- **No se puede** sin plantilla propia: cambiar el tamaño o el diseño de la ventana (el asistente clásico de Windows), las fuentes, un tema oscuro, animaciones o Acrylic, ni los mensajes genéricos de Tauri (p. ej. «Ya está instalado…»).
- Las imágenes son BMP de tamaño fijo: con escalado al 150 % Windows las estira un poco.
- Si Tauri cambia el orden de su plantilla y deja de incluir los hooks antes de las páginas, los textos volverían a los de NSIS: lo comprueba la prueba del instalador en Windows Sandbox (`docs/prueba-instalacion.md`).
