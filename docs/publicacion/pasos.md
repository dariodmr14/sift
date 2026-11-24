# El día de publicar: pasos exactos

Todo lo de publicar está **preparado e inactivo**. Nada sale hasta que hagas estos pasos.

**Estado (2026-11-24):** hechos los pasos 1 a 4 (versión 1.0.0, clave de firma, secretos y clave pública). Quedan el 0, el 5 y del 6 en adelante. Los marcados **(tú)** solo los puedes hacer tú, porque tocan claves, secretos o la visibilidad del repo. Los marcados **(Claude)** los puede preparar Claude en un PR, como siempre.

| Pieza | Dónde | Estado |
|---|---|---|
| Workflow de Releases | `.github/workflows/release.yml` | Listo. Se lanza al subir una etiqueta `v*`. Antes de compilar comprueba que están la clave, los secretos y las notas, y si falta algo para y lo dice |
| Actualizador | `apps/desktop/src-tauri/src/updates.rs` ([ADR 0018](../adr/0018-actualizaciones.md)) | Solo va en las compilaciones de las Releases. Con la clave pública |
| Configuración del actualizador | `apps/desktop/src-tauri/tauri.updater.conf.json` | Con la clave pública (`1AE49F71D00F5175`) |
| Notas de la versión | `docs/publicacion/notas/v1.0.0.md` | Escritas; revísalas |

## 0. Antes (días antes)

- [ ] Mira si queda algo en [`pendiente-decidir.md`](../proceso/pendiente-decidir.md).
- [ ] Borra la rama remota ya fusionada `feat/instalador` (decisión 14): `git push origin --delete feat/instalador`.
- [ ] Revisa y fusiona el PR del README.
- [ ] Haz la prueba en Windows Sandbox y la prueba con una persona ([prueba-instalacion.md](../prueba-instalacion.md)) con la última versión candidata.

## 1. Subir la versión a 1.0.0 (Claude) — hecho

En una rama y un PR, como siempre: `1.0.0-rc.N` → `1.0.0` en:
- `Cargo.toml` (workspace) y `Cargo.lock`;
- `apps/desktop/package.json` y `package-lock.json`;
- `apps/desktop/src-tauri/tauri.conf.json`.

La etiqueta del paso 7 tiene que ser `v` + esa versión; si no coinciden, el workflow para. Sin guion (`1.0.0`, no `1.0.0-rc.5`) la Release sale como versión estable y el actualizador la ofrece.

## 2. Generar la clave de firma (tú, una vez en la vida del proyecto) — hecho

Las actualizaciones van firmadas: Sift solo instala una versión nueva si la firma encaja con la clave pública que lleva dentro. **La clave privada no la ve nadie más, tampoco Claude.**

En PowerShell:

```powershell
cd "$HOME\Documents\GitHub\Sift\apps\desktop"
npx tauri signer generate -w "$HOME\.tauri\sift.key"
```

1. Te pide una contraseña dos veces. Usa una larga y nueva.
2. Crea dos archivos:
   - `%USERPROFILE%\.tauri\sift.key`: la clave **privada**;
   - `%USERPROFILE%\.tauri\sift.key.pub`: la pública.

Dónde va cada parte:

| Qué | Dónde | Por qué |
|---|---|---|
| `sift.key` (privada) | Tu gestor de contraseñas (adjunto o nota segura) **y** el secreto del repo (paso 3). Puedes dejar la copia de `%USERPROFILE%\.tauri\` o borrarla después | Si se pierde, **nadie que tenga Sift instalado recibirá más actualizaciones**: tendrían que reinstalar a mano |
| La contraseña | Tu gestor de contraseñas **y** el secreto del repo (paso 3) | Sin ella, la clave no sirve |
| `sift.key.pub` (pública) | En el repo (paso 4). Se puede enseñar | Sift la lleva dentro para comprobar las firmas |

**Nunca** subas `sift.key` al repo, ni la pegues en un chat, un issue o un PR. `.tauri\` está en tu carpeta de usuario, fuera del repo.

## 3. Guardar la clave en GitHub (tú) — hecho

En GitHub: **Sift → Settings → Secrets and variables → Actions → New repository secret**. Crea dos:

| Name | Secret |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | El contenido entero de `sift.key`. Para copiarlo: `Get-Content "$HOME\.tauri\sift.key" -Raw \| Set-Clipboard` y pégalo |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | La contraseña del paso 2 |

Hazlo en la web, con tu cuenta: el token de Claude solo tiene acceso al código y no puede leer ni escribir secretos (está bien que sea así). Después, vacía el portapapeles: `Set-Clipboard -Value $null`.

## 4. Poner la clave pública (Claude) — hecho

Pásale a Claude el contenido de `sift.key.pub` (es pública) o pégalo tú en `pubkey` de `apps/desktop/src-tauri/tauri.updater.conf.json`, en lugar de `PEGA_AQUI_LA_CLAVE_PUBLICA…`. Va en un PR, con su CI.

## 5. Revisar las notas (tú)

Lee `docs/publicacion/notas/v1.0.0.md`. Es lo que se verá en la página de la Release. Si cambias algo, va en un PR.

## 6. Hacer público el repositorio (tú)

1. **Settings → General → Danger Zone → Change repository visibility → Make public.** GitHub pide escribir el nombre del repo.
2. **Settings → Code security:** activa **Secret scanning** y **Push protection** (gratis en repos públicos).
3. **Settings → Actions → General:** que esté en «Allow all actions and reusable workflows». Con el repo público, los minutos de Actions en Windows son gratis.

Desde ese momento se ve todo el historial y los PR. Lo que contiene está revisado en [limpieza.md](limpieza.md).

## 7. Crear la etiqueta `v1.0.0` (tú, o Claude si se lo pides expresamente)

Desde `main` actualizado, con el PR del paso 1 ya fusionado:

```powershell
cd "$HOME\Documents\GitHub\Sift"
git checkout main
git pull
git tag -a v1.0.0 -m "Sift 1.0.0"
git push origin v1.0.0
```

Subir la etiqueta lanza el workflow **Release**.

## 8. Seguir el workflow (tú)

GitHub → **Actions → Release**. Tarda unos 20–30 minutos.

- Si para en **«Comprobar que se puede publicar»**, el error dice qué falta: un secreto, la clave pública, las notas o que la etiqueta no coincide con la versión. Arréglalo, borra la etiqueta (paso 10) y vuelve al paso 7.
- Si para en **«Comprobar el instalador»**, al instalador le falta algo para arrancar en un Windows limpio (el runtime de Visual C++ o PDFium; [ADR 0020](../adr/0020-runtime-de-visual-cpp.md)). La Release se queda en borrador a medias: bórrala (paso 10) y pásale el registro a Claude.
- Si falla más adelante, pásale el registro a Claude.

## 9. Comprobar la Release y publicarla (tú)

En **Releases** aparece **Sift 1.0.0** en borrador (*Draft*). Comprueba:

- [ ] Tiene `Sift_1.0.0_x64-setup.exe`, `Sift_1.0.0_x64-setup.exe.sig`, `latest.json` y `SHA256SUMS.txt`.
- [ ] Las notas son las de `notas/v1.0.0.md`.
- [ ] **No** está marcada como *pre-release*.
- [ ] Descarga el instalador y comprueba su huella: `Get-FileHash .\Sift_1.0.0_x64-setup.exe` tiene que dar lo mismo que `SHA256SUMS.txt`.
- [ ] Instálalo en la Windows Sandbox (`scripts/sandbox.ps1`): abre, busca y en Ajustes → General sale «Buscar actualizaciones al arrancar», **marcado**.

Si todo está bien: **Edit → Publish release**.

Después:
- [ ] `https://github.com/dariodmr14/Sift/releases/latest/download/latest.json` se abre y dice `"version": "1.0.0"`.
- [ ] El enlace de descarga del README lleva a la Release.
- [ ] Las actualizaciones se podrán comprobar de verdad con la siguiente versión (1.0.1): una 1.0.0 instalada tiene que preguntar al arrancar si quieres instalarla.

## 10. Si algo sale mal

Mientras la Release esté en borrador no la ve nadie. Para empezar de nuevo:

```powershell
git push origin --delete v1.0.0
git tag -d v1.0.0
```

y borra la Release en borrador desde GitHub (Releases → Sift 1.0.0 → Delete).

Si ya está publicada, no la borres: publica una 1.0.1 con el arreglo, que es lo que recibirá quien tenga las actualizaciones activadas.

## Las versiones siguientes

Los pasos 1, 5, 7, 8 y 9, con la versión nueva y sus notas en `docs/publicacion/notas/v<versión>.md`. Una versión con guion (`1.1.0-rc.1`) sale como *pre-release* y el actualizador de las estables no la ofrece.
