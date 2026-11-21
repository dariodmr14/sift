# Estado de la sesión (2026-11-21, mediodía)

Darío para hasta la semana que viene. Aquí está dónde se ha quedado todo. La lista de tareas está en [tareas-pendientes.md](tareas-pendientes.md).

## Fusionado en `main`

| PR | Qué |
|---|---|
| [#62](https://github.com/dariodmr14/Sift/pull/62) | Limpieza antes de hacerlo público: informe en `docs/publicacion/limpieza.md` y decisión 14 aplicada |
| [#63](https://github.com/dariodmr14/Sift/pull/63) | Workflow de Release con etiqueta `v*` (listo e inactivo), actualizaciones encendidas por defecto (decisión 15, ADR 0018), `docs/publicacion/pasos.md` y las notas de la v1.0.0 |
| [#64](https://github.com/dariodmr14/Sift/pull/64) | Datos de demostración en inglés (`demo-en/`, `SIFT_DEV_DEMO=en`) y `--en` en los scripts de capturas |

## PR abiertos (en borrador, sin fusionar)

| PR | Rama | Estado | Qué falta |
|---|---|---|---|
| [#65](https://github.com/dariodmr14/Sift/pull/65) | `docs/readme-publico` | **Espera la revisión de Darío. No fusionar sin su visto bueno** | 1) Los GIF de `docs/media/` (ahora son el de la rc.2, con datos en español en los dos). 2) La CI local |
| [#66](https://github.com/dariodmr14/Sift/pull/66) | `chore/capturas-rc4` | Arreglo del script de la introducción y capturas de la rc.4 | La CI local; después, fusionar |
| — | `docs/estado-sesion` | Este archivo, la lista al día y los resultados de las pruebas de aguante | La CI local; después, fusionar |

Ninguna de las tres ramas ha pasado la CI: se paró antes. Al volver, en cada una: `scripts/ci.ps1` y, si está en verde, merge (salvo #65, que revisa Darío).

## Vídeo en inglés (fuera del repo, en `Documents\Sift - vídeo de presentación\lanzamiento\`)

**Hecho:**
- `video-en/`: copia del proyecto de HyperFrames con el mismo montaje y la misma música. Cambian solo los datos:
  - las composiciones (`desorden`, `espacio`, `ventanas`) con nombres de archivo, ventanas, VS Code y el contrato en inglés (nada de «Calculadora»);
  - `assets/img/intro-*.jpg`: la introducción con los datos en inglés;
  - `assets/rec/`: las cuatro escenas grabadas de nuevo con la rc.4 y los datos en inglés, con su último fotograma (`*-final.jpg`) y `4-tienda.jpg`. Las duraciones coinciden con las de español (2,7 / 9,0 / 5,3 / 6,9 s).
  - En `assets/textos.js` se quita la nota «“duplicate information”» bajo la caja, porque ahora se escribe en inglés.
- La búsqueda por significado del vídeo en inglés es **«avoid repeating data»** → `unit-3-normalization.md` («Trata de lo mismo»). Con «duplicate information» salía primero el `.sql`.
- `grabar.mjs --en` graba con los datos en inglés en `assets/rec-en/` (se guarda una copia del original en `grabar.antes-en.mjs.bak`).
- La interfaz de Sift sale en español, como en el vídeo original: la app no está traducida.

**Falta:**
1. `cd video-en`, `npx --yes hyperframes@0.9.70 check` (se paró a medias) y revisar en `npx hyperframes preview` que los punch-in caen en el sitio (las grabaciones duran lo mismo, pero el resultado de «avoid repeating data» y las filas pueden estar un poco movidos).
2. Render: `npx --yes hyperframes@0.9.70 render -q delivery --variables '{"lang":"en"}' -o out/sift-lanzamiento-en.mp4`. Después, copiarlo a `video/out/` en lugar del antiguo.
3. GIF en inglés, con el comando de `video/SPEC.md` sobre el vídeo nuevo, y llevarlo a `docs/media/sift-en.gif` del PR #65.
4. GIF en español con la rc.4: `node grabar.mjs significado` (graba en `video/assets/rec/`), render en español y su GIF a `docs/media/sift-es.gif`. Opcional: el vídeo en español es de la rc.2.
5. Actualizar `LEEME.md` de la carpeta del vídeo.

## Lo que tiene que hacer Darío

- Revisar el PR #65 (README y licencia).
- Borrar la rama remota ya fusionada `feat/instalador` (decisión 14; los permisos del proyecto no dejan a Claude): `git push origin --delete feat/instalador`.
- El día de publicar: [docs/publicacion/pasos.md](publicacion/pasos.md).

## Decisiones

No queda ninguna pendiente: la 14, la 15 y la 16 están decididas y aplicadas (`docs/proceso/pendiente-decidir.md`).

## Scratchpad

Nada de lo que hay en el scratchpad de Claude hace falta: lo que valía está en GitHub (las ramas de arriba) o en la carpeta del vídeo. Los resultados en bruto de las pruebas de aguante, que respaldan las cifras de `docs/auditoria-1.0.md`, se copian a `docs/proceso/aguante-resultados/` con las rutas de usuario cambiadas por `<scratchpad>` y `<usuario>`.

## Comprobado al parar

- No queda abierta ninguna Sift de desarrollo, ni ventanas de grabación, ni procesos de HyperFrames.
- La carpeta de datos de desarrollo (`%APPDATA%\dev.sift.app-dev`) está como estaba antes de las grabaciones: no existía y no existe. No queda ninguna copia apartada (`.antes-capturas`, `.antes-aguante`).
- La Sift instalada y sus datos no se han tocado.
