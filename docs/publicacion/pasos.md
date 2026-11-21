# Lanzamiento: Releases y actualizaciones

Todo lo de esta página está **preparado e inactivo**. Nada se publica hasta que Darío lo decida.

| Pieza | Dónde | Estado |
|---|---|---|
| Workflow de Releases | `.github/workflows/release.yml` | Solo se lanza a mano y deja la Release **en borrador** |
| Actualizador | `apps/desktop/src-tauri/src/updates.rs` ([ADR 0018](adr/0018-actualizaciones.md)) | Solo funciona en compilaciones con `tauri.updater.conf.json`. Falta la clave pública |
| Configuración del actualizador | `apps/desktop/src-tauri/tauri.updater.conf.json` | Con `PEGA_AQUI_LA_CLAVE_PUBLICA` |

## 1. Generar la clave de firma (Darío, una vez)

Las actualizaciones van firmadas: Sift solo instala una versión nueva si la firma encaja con la clave pública que lleva dentro. **La clave privada no la ve nadie más, tampoco Claude.**

En PowerShell, desde `apps/desktop`:

```powershell
npm run tauri signer generate -- -w "$HOME\.tauri\sift.key"
```

1. Te pide una contraseña dos veces. Usa una larga y guárdala en tu gestor de contraseñas.
2. Crea dos archivos: `%USERPROFILE%\.tauri\sift.key` (la **privada**) y `%USERPROFILE%\.tauri\sift.key.pub` (la pública).
3. Guarda también `sift.key` en tu gestor de contraseñas (como archivo adjunto o nota segura). **Si se pierde, no se pueden publicar actualizaciones para quien ya tenga Sift instalado**: tendrían que reinstalarlo a mano.
4. No la subas nunca al repositorio. `.tauri\` está fuera del repo, en tu carpeta de usuario.

## 2. Guardarla en GitHub (Darío)

Con `gh` autenticado con tu cuenta (no con el token de Claude, que solo tiene acceso al código):

```powershell
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo dariodmr14/Sift < "$HOME\.tauri\sift.key"
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo dariodmr14/Sift
```

El segundo comando te pide la contraseña sin enseñarla. También se puede hacer en GitHub → Settings → Secrets and variables → Actions.

## 3. Poner la clave pública (puede hacerlo Claude)

Copia el contenido de `sift.key.pub` (es pública, se puede compartir) en `pubkey` de `apps/desktop/src-tauri/tauri.updater.conf.json`, en lugar de `PEGA_AQUI_LA_CLAVE_PUBLICA…`. Va en un PR, como cualquier cambio.

## 4. Publicar una versión

1. Sube la versión en `Cargo.toml` (workspace), `apps/desktop/package.json` y `apps/desktop/src-tauri/tauri.conf.json`.
2. GitHub → Actions → **Release** → *Run workflow* en `main`.
3. Revisa la Release en borrador que aparece: el instalador `Sift_<versión>_x64-setup.exe`, su `.sig` y `latest.json`. Las versiones con guion (`1.0.0-rc.2`) se marcan como *pre-release*.
4. Pulsa **Publish release**.

Las actualizaciones automáticas leen `releases/latest/download/latest.json`, es decir, la última Release **publicada y no pre-release**. Las versiones candidatas no se ofrecen a quien tiene una estable.

Para lanzarlo solo al crear una etiqueta `v*`, está explicado al principio de `release.yml`.

## 5. Requisitos

- **GitHub Actions:** con el repo privado, el presupuesto es 0 $ y no hay minutos. **Con el repo público, los runners de Windows son gratis.** Hasta entonces, el instalador se genera en local (`scripts/instalar.ps1`).
- **Firma de código (SmartScreen):** no está. Windows avisará al instalar («Más información» → «Ejecutar de todas formas»). Está en el backlog: cuesta dinero.
