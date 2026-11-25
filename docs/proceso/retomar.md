# Retomar Sift

Aparcado el 2026-11-25, tras publicar la 1.0.0. Esta página basta para volver; lo demás, solo si hace falta.

## Estado

1. **Sift 1.0.0 publicada** en GitHub Releases (repo público, licencia «todos los derechos reservados»). Instalador NSIS con el runtime de Visual C++ y WebView2, probado en Windows Sandbox.
2. **Funciona:** la barra (`Alt+Espacio`) busca por nombre, contenido y significado (e5-small, local), con vista previa (Tab), filtros, PDF en su página y frecencia.
3. **Espacios:** abren carpetas, archivos, webs, apps y comandos (con permiso nativo de Windows), esperan a servicios locales y colocan ventanas en zonas.
4. **Actualizaciones firmadas:** al arrancar, Sift mira `latest.json` de la última Release y pregunta antes de instalar. La 1.0.1 será la primera prueba real.
5. **Fases 0 a 8 cerradas** ([PLAN.md](../../PLAN.md)); sin PR abiertos ni nada pendiente en `FEEDBACK.md`.

## La 1.1, por prioridad

1. **Las notas en `latest.json`** (ideal como 1.0.1, para estrenar el actualizador): hoy dice «Notas en camino.». En `release.yml`, pasar `docs/publicacion/notas/v<versión>.md` como `releaseBody` de `tauri-action`.
2. **«Security and quality» de GitHub:** es la pestaña de seguridad del repo. Desde fuera marca 0 alertas y avisa de que **no hay política de seguridad**. Añadir `SECURITY.md` (cómo avisar de un fallo en privado) y activar las alertas de Dependabot. Las alertas solo las ves tú (el token de Claude no puede leerlas): `npm audit` y `cargo audit` estaban limpios el 2026-11-25.
3. **Versión en inglés:** textos de la interfaz, datos de demostración y vídeo (`demo-en/` ya existe).
4. **Reranker local** para los primeros resultados, solo si mejora recall@10 y MRR en `eval/` y cabe en 50 ms p95 (ADR 0010).
5. **Zonas propias para las ventanas**, dibujadas por pantalla (el modelo ya lo admite, ADR 0014).
6. **Conectores:** Notion y Google Calendar (rompen el «todo local»: pide un ADR antes).
7. **Lo que dejó abierto la 1.0** ([auditoría §3](../auditoria-1.0.md#3-para-la-11) y [pendiente-decidir.md](pendiente-decidir.md)): vectorizar más rápido; memoria de WebView2 (~340 MB) y del modelo (~600 MB); `.lnk` que siguen a su destino (decisión 11); «y N copias más» (decisión 6); volver a buscar al acabar el indexado; grabar el atajo pulsándolo; avisar de carpetas sin permiso.

`FEEDBACK.md` no tiene nada abierto: lo nuevo saldrá de volver a usarlo.

## Arrancar el entorno

1. **Herramientas:** Visual Studio Build Tools con «Desarrollo de escritorio con C++», Rust con rustup (la versión la fija `rust-toolchain.toml` y se instala sola), Node 22+, Git y `gh` (`gh auth login` con el token *fine-grained* de este repo). Para la CI: `cargo install cargo-audit` y 7-Zip.
2. **Una vez:** `pwsh scripts/fetch-pdfium.ps1` y `pwsh scripts/vc-runtime.ps1`.
3. **Desarrollo:** `cd apps/desktop`, `npm install`, `npm run tauri dev` (datos propios en `%APPDATA%\dev.sift.app-dev`, atajo `Ctrl+Alt+Espacio`; no toca la Sift instalada).
4. **CI local** (antes de cada merge): `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1`.
5. **Publicar una versión:** subir la versión en `Cargo.toml`, `package.json` y `tauri.conf.json` (y sus lock), escribir `docs/publicacion/notas/v<versión>.md`, fusionar, subir la etiqueta `v<versión>` y revisar el borrador de la Release. Detalle en [publicacion/pasos.md](../publicacion/pasos.md) (pasos 1, 5, 7, 8 y 9).

Más detalle: [desarrollo.md](../desarrollo.md). Reglas de trabajo: [CLAUDE.md](../../CLAUDE.md).

## Fuera del repo

- **Clave de firma de las actualizaciones** (`sift.key` y su contraseña): en tu gestor de contraseñas, y como secretos `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` del repo. **Si se pierde, nadie recibe más actualizaciones.** La pública está en `tauri.updater.conf.json` (`1AE49F71D00F5175`).
- **Vídeo de presentación:** `Documents\Sift - vídeo de presentación\`, con el proyecto de HyperFrames, las versiones en español e inglés y su `LEEME.md`.
- **Tu Sift de uso diario:** datos en `%APPDATA%\dev.sift.app`. No se tocan al desarrollar.
