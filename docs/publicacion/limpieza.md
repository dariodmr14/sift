# Limpieza antes de hacer público el repositorio

> Revisión del 2026-11-21 sobre **todo** lo que se verá al hacerlo público: los 325 commits de todas las ramas, los archivos actuales, las 51 imágenes, las ramas y etiquetas de GitHub y las descripciones de los 61 PR. Sigue a la [revisión del 2026-11-19](../proceso/publicar-revision.md), que se aprobó y se aplicó (rutas de los tests, «Uniovi» fuera de `eval/`, documentos de trabajo en `docs/proceso/`).
>
> **Solo es un análisis: no se ha cambiado nada ni se ha reescrito el historial.**

## Resumen

| # | Qué | Dónde | Gravedad | Propuesta |
|---|---|---|---|---|
| 1 | Secretos y tokens | — | Ninguna | No hay. Activar el escaneo de secretos de GitHub al publicar |
| 2 | Tu correo | — | Ninguna | No aparece. Nada que hacer |
| 3 | Tu ruta de usuario (`C:\Users\Darío`) | 1 documento actual; historial (8 commits, 6 capturas borradas, 1 mensaje, 1 PR) | Baja | Cambiar el documento actual; el historial, **dejarlo** |
| 4 | Capturas actuales | `docs/capturas/` | Ninguna (datos inventados) | Dejarlas. Rehacer las `final/`, que son anteriores a la rc.3 |
| 5 | Datos de tu carrera y de tus apps | `eval/`, ADR, `FEEDBACK.md`, PR #13 | Muy baja | Dejarlos |
| 6 | Archivos internos | `CLAUDE.md`, `.claude/`, `FEEDBACK.md`, `docs/proceso/`… | Ninguna | Dejarlos; borrar dos que sobran |
| 7 | Restos en GitHub | rama `feat/instalador` | Ninguna | Borrar la rama (ya fusionada) |
| 8 | Tu nombre | `LICENSE`, `Cargo.toml`, documentos | Ninguna | Dejarlo: es tu firma |
| 9 | Reescribir el historial | — | — | **No hace falta** |

## 1. Secretos y tokens: limpio

**Cómo se ha buscado.** En el diff de los 325 commits (`git log --all -p`), en los archivos actuales y en las descripciones de los 61 PR, con patrones de:
- tokens de GitHub (`ghp_`, `github_pat_`, `gho_`);
- claves de AWS (`AKIA…`), claves privadas (`BEGIN … PRIVATE KEY`), tokens de Slack y claves `sk-…` y `sk-ant-…`;
- asignaciones de `api_key`, `password`, `secret` o `token` con un valor.

**Qué se ha encontrado.** Ningún secreto. Tampoco se ha subido nunca un archivo con nombre sospechoso (`.env`, `*.key`, `*.pem`, `*.pfx`, `settings.local.json`, bases de datos).

**Lo que no sale del PC:**
- el token de Claude, que está en el gestor de `gh`;
- el *credential helper* de `.git/config`, que no se sube;
- la futura clave de firma de las actualizaciones, que va en `%USERPROFILE%\.tauri\`, fuera del repo (ver [pasos.md](pasos.md)).

**Propuesta:**
- Al hacerlo público, activa en GitHub → Settings → Code security el **Secret scanning** y el **Push protection**. Son gratis en repos públicos y vigilan cada push a partir de ese día.
- Pasar **gitleaks** sigue siendo opcional. Habría que instalarlo, y eso necesita tu permiso.

## 2. Correo: limpio

- Los 325 commits usan `329515735+dariodmr14@users.noreply.github.com` (el correo privado de GitHub) o `noreply@github.com` (los *merge* hechos en GitHub).
- Tu correo real no aparece en el historial, ni en los archivos, ni en los PR.

## 3. Tu ruta de usuario

**En los archivos actuales:** solo en `docs/proceso/publicar-revision.md` (líneas 13, 34 y 49–51), que cita la ruta como ejemplo de lo que se limpió. Los tests y el código ya usan `C:\Users\Núria`.

**En el historial:**

| Dónde | Qué |
|---|---|
| 8 commits (del 10 al 19-11) | La ruta en dos tests de la interfaz, dos comentarios de `crates/search/src/vectors.rs`, `docs/aprendido.md` y la revisión del 19-11. Ya están cambiados en el árbol actual |
| 6 capturas borradas | `docs/capturas/{0-antes,1-spacing,2-color,3-icons,4-copy,5-type}-ajustes-espacio.jpg`: el editor de Espacios con `C:\Users\Darío\Documents\GitHub\Sift` |
| 1 mensaje de commit | El arreglo de usearch con rutas con tildes («…C:\Users\Darío failed…») |
| PR #45 | Su descripción cuenta el cambio `C:\Users\Darío` → `C:\Users\Núria` |

Qué revela: tu nombre de pila (que ya firma el proyecto) y que el repo está en `Documents\GitHub`. Nada más.

**Propuesta:**
- **Árbol actual:** cambiar esas líneas por `C:\Users\<tu usuario>`. Es un PR de cinco líneas; lo hago cuando digas.
- **Historial: dejarlo** (ver §9).

## 4. Capturas: datos inventados

He mirado las 35 capturas de `docs/capturas/` (las 16 restantes son iconos de la app):

- **`final/` e `introduccion/`:** salen de los datos de demostración (`demo/`, `C:\Users\Public\Documents\Sift (demostración)`). No hay rutas ni archivos tuyos. Pero `final/` es del 19-11, anterior a la rc.3 (no tiene el indicador del tamiz ni los últimos cambios). **Propuesta:** rehacerlas con la rc.4 y los mismos datos de demostración. Esta noche lo hago para el README (tarea 2).
- **Antes y después de unslop (`0-antes-…` a `6-details-…`):** la barra buscando en los documentos del propio repo (`design-system.md`, los ADR…) y Ajustes → General. No hay nada personal. **Propuesta:** dejarlas; son la historia del diseño que enlaza `docs/proceso/unslop-direccion.md`.

## 5. Datos de tu carrera y de las apps que usas

| Dónde | Qué |
|---|---|
| `eval/queries.json`, `eval/README.md` | 34 consultas reales sobre «la carpeta de una carrera de informática», con nombres de apuntes (`apuntes_bd_completos.pdf`, `6 Backtracking.pdf`…) |
| ADR 0007, 0009, 0010, 0012, `PLAN.md`, `FEEDBACK.md`, `CLAUDE.md` | «La carpeta de la uni», con sus cifras (86 509 archivos, 543 PDF…) y asignaturas como ejemplo |
| `FEEDBACK.md` (líneas 140–150) | Que usas Eclipse y Opera (cómo se colocan sus ventanas) |
| PR #13 | Textos de ejemplo descartados: «el campus virtual», «apuntes de la universidad» |

Nada de eso identifica a nadie: no hay universidad, nombres de personas ni rutas.

**Propuesta: dejarlo.** Las consultas de `eval/` son la prueba de que la búsqueda se midió con datos reales.

## 6. Archivos internos

| Archivo | Qué es | Propuesta |
|---|---|---|
| `CLAUDE.md`, `.claude/agents/*`, `.claude/settings.json` | Reglas para Claude Code, los dos subagentes y los permisos de comandos. Sin rutas ni secretos | **Dejar.** El README cuenta que Sift se hizo con Claude Code, y estos archivos enseñan cómo |
| `PLAN.md`, `FEEDBACK.md`, `docs/aprendido.md` | El plan, lo que fuiste notando y lo aprendido | **Dejar** (buen material para entrevistas). El README enlaza `aprendido.md` |
| `docs/proceso/*` | Auditorías, decisiones pendientes, la dirección de unslop y la revisión anterior | **Dejar.** Están apartados y se entienden como proceso |
| `docs/auditoria-1.0.md` | La auditoría de la 1.0, con cifras | **Dejar.** El README la enlaza |
| `docs/borradores/README-publico.md` | El borrador del README público | **Borrar** cuando el README nuevo esté fusionado (va en ese PR) |
| `docs/tareas-pendientes.md` | La lista de esta noche | **Mover** a `docs/proceso/` al acabar, o borrarla |
| `docs/lanzamiento.md` | Releases y actualizaciones | **Sustituido** por `docs/publicacion/pasos.md` (tarea 4) |
| `PLAN.md`, Fase 8 | Cita la carpeta del vídeo fuera del repo (`Documents\Sift - vídeo de presentación`) | Dejar: no es personal |
| `.vscode/` | Ajustes del editor | Dejar |

## 7. Restos en GitHub

| Qué | Estado | Propuesta |
|---|---|---|
| Rama `feat/instalador` | Ya fusionada (PR #55); no tiene nada que no esté en `main` | **Borrarla** (`git push origin --delete feat/instalador`; se puede recrear) |
| Etiquetas y Releases | Ninguna | La primera será `v1.0.0` ([pasos.md](pasos.md)) |
| Descripciones de PR | 61 | Ver §3 y §5: nada grave |
| `stash` local de GitHub Desktop | Solo en tu PC, nunca se ha subido. Es el único sitio con un `pdfium.dll` | No afecta a la publicación |

## 8. Tu nombre: dejarlo

«Darío» sale en:
- `LICENSE` («Copyright (c) 2026 Darío»);
- `Cargo.toml` (`authors`);
- los documentos de proceso;
- 27 archivos en total, sobre todo ADR y `CLAUDE.md`.

Firmar tu trabajo es lo que quieres para LinkedIn. La alternativa sería `dariodmr14`.

## 9. Reescribir el historial: no hace falta

No hay secretos ni correo. Lo único que queda en el historial es la ruta `C:\Users\Darío` (§3) y nombres de apuntes.

**Por qué no reescribirlo.** Reescribirlo (`git filter-repo`):
- obligaría a un **force push a `main`**;
- cambiaría todos los hashes;
- rompería los enlaces de los 61 PR;
- no borraría las copias que GitHub guarda de los PR, que siguen mostrando los diffs antiguos.

El coste es alto para algo de gravedad baja. **Recomiendo no hacerlo.** Si aun así lo quieres, preparo los comandos y no los ejecuto sin tu permiso expreso.

## Decidido (2026-11-21)

1. Cambiar la ruta en `docs/proceso/publicar-revision.md` (§3): **sí, hecho** (`C:\Users\<tu usuario>`).
2. Borrar la rama remota `feat/instalador` (§7): **sí**. Queda para ti, porque los permisos del proyecto no dejan a Claude borrar ramas remotas: `git push origin --delete feat/instalador`.
3. gitleaks (§1): **no**.
4. Reescribir el historial (§9): **no**.
