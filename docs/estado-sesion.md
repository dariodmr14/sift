# Estado de la sesión (2026-11-24)

La tanda «preparar la publicación sin publicar nada» está terminada. La lista: [tareas-pendientes.md](tareas-pendientes.md).

## Fusionado en `main`

| PR | Qué |
|---|---|
| [#62](https://github.com/dariodmr14/Sift/pull/62) | Limpieza antes de hacerlo público (`docs/publicacion/limpieza.md`, decisión 14) |
| [#63](https://github.com/dariodmr14/Sift/pull/63) | Workflow de Release con etiqueta `v*` (listo e inactivo), actualizaciones encendidas por defecto (decisión 15, ADR 0018), `docs/publicacion/pasos.md` y notas de la v1.0.0 |
| [#64](https://github.com/dariodmr14/Sift/pull/64) | Datos de demostración en inglés (`demo-en/`, `SIFT_DEV_DEMO=en`) |
| [#66](https://github.com/dariodmr14/Sift/pull/66) | Script de la introducción (comprueba que «Empezar» la cierra), capturas de la rc.4 y `cookie` 0.18.3 / `url` 2.5.9 por dos avisos nuevos de `cargo audit` (RUSTSEC-2026-0594 y -0592) |
| [#67](https://github.com/dariodmr14/Sift/pull/67) | Estado de la sesión anterior y resultados de las pruebas de aguante |

## Espera la revisión de Darío

[#65](https://github.com/dariodmr14/Sift/pull/65) (`docs/readme-publico`): README en español e inglés, `LICENSE` y `docs/desarrollo.md`, con los GIF de la rc.4 (`docs/media/sift-es.gif` y `sift-en.gif`). CI local en verde. **No se fusiona sin su visto bueno.**

## Vídeo (fuera del repo: `Documents\Sift - vídeo de presentación\`)

Rehecho con la rc.4 y con las skills Video Shotcraft (planos y cámara), real-sound y Earshot (sonido), sobre HyperFrames. Cómo está hecho y cómo rehacerlo: `LEEME.md` de esa carpeta (`bash lanzamiento/rehacer-todo.sh` lo rehace todo).

- `lanzamiento/video/out/`: `sift-lanzamiento-es.mp4`, `sift-lanzamiento-es-sin-musica.mp4` y `sift-es.gif`.
- `lanzamiento/video-en/out/`: lo mismo en inglés, con los datos de demostración en inglés.
- Un revisor independiente (subagente sin contexto) lo revisó fotograma a fotograma con las reglas de Shotcraft. Se arreglaron:
  - la introducción, que no se leía;
  - el logo de la promesa, que solo se veía 0,2 s;
  - el tamaño de los subtítulos;
  - el zoom del pico, que caía 1,5 s antes;
  - la frase final;
  - el clic de las ventanas, que llegaba tarde;
  - la deriva del logo.

  Se dejaron, a propósito, algunos bordes cortados durante los acercamientos: lo que hay que leer queda dentro.
- Sonido: −14 LUFS, pico −1,5 dBFS, 0 ms de desfase entre la banda y el MP4. Los efectos son 10 grabaciones CC0 de Freesound con su procedencia. **Nadie lo ha escuchado todavía**: Earshot se instaló sin el extra `listen` y todo lo comprobado son medidas.

## Lo que tiene que hacer Darío

1. Ver y escuchar los dos vídeos.
2. Revisar el PR #65 y fusionarlo si le parece bien.
3. El día de publicar: [docs/publicacion/pasos.md](publicacion/pasos.md).

## Decisiones

No queda ninguna pendiente (`docs/proceso/pendiente-decidir.md`).

## Comprobado al terminar

- No queda abierta ninguna Sift de desarrollo ni procesos de grabación o de HyperFrames.
- `%APPDATA%\dev.sift.app-dev` está como antes de grabar: no existía y no existe.
- La Sift instalada y sus datos no se han tocado.
