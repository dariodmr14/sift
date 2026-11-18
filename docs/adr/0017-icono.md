# ADR 0017 — Icono de Sift: «el tamiz»

- **Estado:** aceptada
- **Fecha:** 2026-11-18

## Contexto

Hasta la 1.0.0-rc.1, Sift usaba el icono de Tauri por defecto, también en la bandeja. Para la versión candidata hacía falta un icono propio, coherente con Vidrio, que se leyera a 16 px y que funcionara en la bandeja con la barra de tareas clara y oscura.

## Decisión

Elegido por Darío entre tres propuestas, el 2026-11-18: **B, «el tamiz»**. Son tres barras redondeadas que se estrechan («filtrar hasta quedarse con lo que buscas», que es lo que significa *sift*). La última va en el azul de acento por defecto de Windows (`#60CDFF`). El fondo es el cuadrado oscuro de Vidrio (`#202024`, esquinas de 56/256) con un borde muy suave.

| Opción | A favor | En contra |
|---|---|---|
| A · La marca (línea de texto con el subrayado del acento) | Es la firma de la barra | A 16 px se lee como «texto» y no se distingue |
| **B · El tamiz** | Se lee a 16 px. Cuenta el nombre. Formas de Vidrio (radios, sin degradados) | Más abstracto que una lupa |
| C · La lupa | Se entiende al momento | Es el icono de casi todos los buscadores |

**Bandeja:** como los iconos de Windows, de un solo color. Blanco sobre la barra de tareas oscura y casi negro (`#1B1B1B`) sobre la clara. Se elige leyendo `SystemUsesLightTheme`, que es el tema de la barra de tareas y no el de las apps (en Windows 10 suelen ser distintos). Se vuelve a mirar cada vez que se abre la barra (`tray::refresh_icon`): es una lectura del registro, y así el cambio de tema se aplica sin escuchar mensajes de Windows.

**Fuentes:** `apps/desktop/src-tauri/icons/source/` (`sift.svg` y `tray-*.svg`). Los PNG, el `.ico` y el `.icns` se generan con `npx tauri icon icons/source/sift.svg` desde `apps/desktop/src-tauri/`. Los de la bandeja, con `-p 32` y copiados a `icons/tray/`. No se generan iconos de Android ni de iOS.

## Consecuencias

- El icono no sigue el acento de cada persona: un icono de programa es fijo en Windows.
- Si se cambia el diseño, se edita el SVG y se vuelven a generar los PNG.
