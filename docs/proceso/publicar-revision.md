# Revisión antes de hacer público el repositorio

> Revisión del 2026-11-19, hecha sobre los 269 commits del historial y los archivos actuales.
>
> **Aprobada por Darío el 2026-11-19 y aplicada** (`chore/limpieza-publico`): rutas y webs del campus cambiadas en los tests, «Uniovi» fuera de `eval/`, documentos de trabajo en `docs/proceso/` y las 6 capturas antiguas de `ajustes-espacio` quitadas del árbol. Las capturas `final/` y las de la introducción ya están rehechas con los datos de demostración (`demo/`, `SIFT_DEV_DEMO=1`): no sale ninguna ruta personal. **No se ha instalado gitleaks ni se ha reescrito el historial**: las dos cosas siguen esperando un sí expreso de Darío.

## Resumen

| # | Qué | Gravedad | Propuesta |
|---|---|---|---|
| 1 | Secretos (tokens, claves, contraseñas) | — | No hay ninguno. Nada que hacer |
| 2 | Tu correo | — | No aparece. Nada que hacer |
| 3 | Tu ruta de usuario (`C:\Users\Darío\…`) en capturas | Media | Rehacer las capturas con datos de demostración |
| 4 | Tu ruta de usuario en tests y comentarios | Baja | Cambiarla por una ruta inventada |
| 5 | Datos de tu carrera (Uniovi, asignaturas, nombres de apuntes) | Baja | Quitar «Uniovi» y las webs del campus; dejar las consultas de `eval/` |
| 6 | Archivos internos del proceso | Ninguna | Dejarlos y ordenarlos |
| 7 | Tu nombre como autor | Ninguna | Dejarlo |
| 8 | Reescribir el historial | — | No hace falta |

## 1. Secretos — limpio

He buscado en el **diff de todos los commits** patrones de tokens de GitHub (`ghp_`, `github_pat_`, `gho_`), claves de AWS, claves privadas (`BEGIN … PRIVATE KEY`), tokens de Slack, claves `sk-…` y asignaciones de `api_key` o `password` con valor. **No hay ninguno.** Tampoco hay ningún `.env` ni `settings.local.json` en el repo.

El push usa `gh auth git-credential` desde `.git/config`, que no se sube.

**Propuesta:** antes de publicar, pasar además **gitleaks**, que es más completo que mi búsqueda. Habría que instalarlo, con tu permiso.

## 2. Correo — limpio

Los 269 commits usan `329515735+dariodmr14@users.noreply.github.com`, el correo privado de GitHub. Tu correo real no aparece **ni en el historial ni en los archivos**.

## 3. Tu ruta de usuario en capturas — rehacerlas

La ruta `C:\Users\Darío\Documents\GitHub\Sift` se ve en el editor de Espacios porque el Espacio de prueba de la versión de desarrollo abre la carpeta del repo:

- `docs/capturas/final/ajustes-espacio.jpg`, que es la que usa el borrador del README público.
- `docs/capturas/0-antes-ajustes-espacio.jpg`, `1-spacing-…`, `2-color-…`, `3-icons-…`, `4-copy-…` y `5-type-ajustes-espacio.jpg` (el antes y después de unslop).

Las capturas de la barra, Carpetas e introducción solo enseñan `C:\…\GitHub\Sift` y nombres de archivos de `docs/`. Eso no es personal. Además, **las capturas `final/` están desfasadas**: todavía dicen «Alias», tienen la rejilla de zonas antigua y el Estado como lista.

**Propuesta:**
- Crear un **conjunto de datos de demostración** en el repo (`demo/`, con apuntes, notas y un par de PDF inventados) y una opción de desarrollo para usarlo. Con él se rehacen las capturas `final/` y se graba el vídeo del bloque 4: una sola pieza para las dos cosas.
- Las capturas de antes y después de unslop se pueden **dejar** (son historia del diseño) o **quitar del árbol**. **Recomiendo quitar** las 6 de `ajustes-espacio`: siguen en el historial, pero solo las encuentra quien lo busque a propósito.

## 4. Tu ruta en tests y comentarios — cambiarla

| Archivo | Qué |
|---|---|
| `apps/desktop/src/launcher/files.test.ts:11` | `C:\Users\Darío\Documents\Uni\BD\Teoría\tema4.pdf` |
| `apps/desktop/src/settings/pathSuggest.test.ts:12` | `C:\Users\Darío\tema4.pdf` |
| `crates/search/src/vectors.rs:67, 197` | `C:\Users\Darío` como ejemplo de carpeta con tilde |
| `docs/aprendido.md:39` | Lo mismo |

**Propuesta:** usar `C:\Users\Núria\…`. Sigue llevando tilde, que es justo lo que prueban esos tests (el fallo de usearch con rutas con tildes).

## 5. Datos de tu carrera — quitar lo identificable

- `eval/queries.json` y `eval/README.md`: «Consultas reales sobre la carpeta de la carrera (Ingeniería Informática del Software, Uniovi)», con nombres de apuntes (`apuntes_bd_completos.pdf`, `6 Backtracking.pdf`…).
- `crates/spaces/src/run.rs:928` y `validate.rs:340, 474`: `campus.uniovi.es` y `campusvirtual.uniovi.es` como webs de ejemplo en los tests.
- `FEEDBACK.md`, los ADR 0007, 0009 y 0010 y `PLAN.md` hablan de «la carpeta de la uni», con sus números (86.509 archivos, 543 PDF…).

**Propuesta:**
- Tests: cambiar las webs del campus por `https://campus.example.edu`.
- `eval/`: **dejar las consultas**. Son la prueba de que la búsqueda se midió con datos reales, y los nombres de apuntes no dicen nada personal. Quitaría solo «Uniovi» de la descripción («la carpeta de una carrera de informática»).
- Los números de la carpeta de la uni en ADR y FEEDBACK: dejarlos. Son mediciones y no identifican a nadie.

## 6. Archivos internos — dejarlos y ordenarlos

| Archivo | Qué es | Propuesta |
|---|---|---|
| `CLAUDE.md`, `.claude/agents/*`, `.claude/settings.json` | Reglas e instrucciones para Claude Code; permisos | **Dejar.** El README ya cuenta que Sift se hizo con Claude Code, y enseñan cómo se trabajó. `settings.json` solo tiene permisos de comandos, sin rutas ni secretos |
| `FEEDBACK.md`, `PLAN.md`, `docs/aprendido.md` | El proceso: lo que fuiste notando, el plan y lo aprendido | **Dejar** (buen material para entrevistas). Si quieres el repo más limpio, moverlos a `docs/proceso/` |
| `docs/pendiente-decidir.md`, `docs/auditoria-fase-7.md`, `docs/propuesta-introduccion.md`, `docs/unslop-direccion.md` | Decisiones y auditorías de trabajo | Mover a `docs/proceso/` |
| `docs/borradores/README-publico.md` | Borrador del README | Borrarlo cuando pase a ser el README de verdad |
| `.vscode/` | Ajustes del editor | Dejar: es habitual |

En todos ellos sales como «Darío»: es tu nombre de autor (punto 7).

## 7. Tu nombre — dejarlo

«Darío» sale en `LICENSE` («Copyright (c) 2026 Darío»), en `Cargo.toml` (`authors`) y en los documentos de proceso. Si prefieres no usar tu nombre, la alternativa es tu usuario de GitHub (`dariodmr14`). **Recomiendo dejarlo:** firmar tu trabajo es lo que quieres para LinkedIn.

## 8. Reescribir el historial — no hace falta

No hay secretos ni tu correo. Lo único que queda en el historial son las capturas antiguas con tu ruta de usuario y los nombres de apuntes. Reescribirlo (con `git filter-repo`) obligaría a un **force push a `main`**, cambiaría todos los hashes y rompería los enlaces de los 40 PR ya fusionados. **Recomiendo no reescribirlo.** Si aun así quieres borrar esas capturas del historial, lo preparo y no lo ejecuto sin tu permiso expreso.

## Licencia — hecho

`LICENSE` pasa a **«Todos los derechos reservados»**, en español y en inglés. Se puede ver y estudiar, e instalar las versiones publicadas; no se puede copiar, modificar ni redistribuir sin permiso. Los componentes de terceros conservan sus licencias. Ese es el sentido de los cambios en el README y su borrador, y en `Cargo.toml` (`LicenseRef-AllRightsReserved`) y `package.json` (`UNLICENSED`).

Ten en cuenta que, en un repositorio público, las Condiciones de GitHub dejan a cualquiera **verlo y hacer un fork dentro de GitHub**. La licencia lo dice, pero no lo puede impedir.
