# ADR 0021 — El instalador lleva el instalador de WebView2 y comprueba que queda instalado

- **Estado:** aceptada
- **Fecha:** 2026-11-25

## Contexto

Al probar el instalador 1.0.0 en Windows Sandbox, ya con el runtime de Visual C++ (ADR 0020), Sift falló al abrir con «Could not find the WebView2 Runtime». WebView2 es el motor de Microsoft que dibuja la interfaz de las apps de Tauri. Viene con Windows 11. En Windows 10 suele llegar con Edge, pero no siempre: el Sandbox no lo trae, y algunos Windows 10 reales tampoco.

`tauri.conf.json` no tenía `webviewInstallMode`, así que se usaba el modo por defecto, `downloadBootstrapper`. La plantilla NSIS de Tauri (2.12) hace esto:
- Si el registro no tiene la versión de WebView2 (`pv` en `EdgeUpdate\Clients\{F3017226-…}`), descarga el bootstrapper de Microsoft y lo ejecuta.
- **Solo mira que `pv` no esté vacío.** Microsoft recomienda tratar también `0.0.0.0` como «no instalado», y la plantilla no comprueba que los archivos existan. Con una entrada así se salta la instalación sin decir nada, y Sift falla al abrir.
- Si la instalación falla, para con «No se ha podido instalar WebView2. Intente reiniciar el instalador.», sin enlace. Ese texto no se puede cambiar sin una plantilla propia: nuestro `hooks.nsh` se incluye antes del archivo de idioma de Tauri (ADR 0019).

## Opciones de Tauri

| Modo | Tamaño del instalador | Internet al instalar | Sin administrador |
|---|---|---|---|
| `downloadBootstrapper` (el que había) | +0 | Sí: el instalador descarga el bootstrapper y este, el runtime | Sí: sin permisos de administrador, Microsoft lo instala solo para el usuario |
| **`embedBootstrapper`** | **+1,6 MiB** (medido: 15,69 → 17,33 MiB) | Sí: el bootstrapper descarga el runtime | Sí, igual |
| `offlineInstaller` | +~127 MB (según Tauri), unas 8 veces el instalador de hoy | No | Sí, igual |
| `skip` | +0 | — | — (Sift no abriría sin WebView2) |

## Decisión

**`embedBootstrapper`**, la preferencia de Darío:
- Cuesta 1,6 MiB.
- Quita un paso que puede fallar: la descarga del bootstrapper desde el propio instalador, con el plugin `NSISdl`.
- Funciona por usuario, como el resto del instalador.
- Pide internet solo cuando falta WebView2, que es lo raro.
- `offlineInstaller` multiplicaría por 8 la descarga de todo el mundo para cubrir un caso que se resuelve con el aviso.

Además, en `installer/hooks.nsh`:
1. **Comprobación de verdad antes de copiar Sift** (`NSIS_HOOK_PREINSTALL`). WebView2 está si `pv` no está vacío ni es `0.0.0.0` y `msedgewebview2.exe` existe en su carpeta. La carpeta sale del valor `location` del registro o, si no lo hay, de las de siempre. Se mira primero la instalación para todo el equipo y después la del usuario.
2. **Si falta,** se ejecuta el mismo bootstrapper que va dentro, en silencio, y se vuelve a comprobar. NSIS guarda el archivo una sola vez aunque se use en dos sitios.
3. **Si sigue faltando**, un aviso en español: qué es WebView2, que no se ha podido instalar (¿falta internet?), el enlace oficial de Microsoft (`https://go.microsoft.com/fwlink/p/?LinkId=2124703`) y «¿Abrir la página ahora?». Después **no se instala Sift**: mejor un aviso claro que un programa que no abre.
4. **`.onInstFailed`**: si la sección de Tauri falla y para, sale el mismo aviso con el enlace, una sola vez.
5. **Al actualizarse desde Sift** (`/UPDATE`) no se comprueba: WebView2 ya estaba, porque Sift se estaba ejecutando.

`scripts/comprobar-instalador.ps1` (ADR 0020) comprueba también que el instalador lleva `MicrosoftEdgeWebview2Setup.exe` con una firma de Microsoft válida. Tauri lo descarga al compilar desde el enlace de Microsoft, que cambia de versión, así que no se puede fijar un hash. Se comprueba la firma en su lugar.

## Consecuencias

- Instalador de 17,33 MiB (+1,6 MiB).
- Sin WebView2 y sin internet, Sift no se instala, y el aviso dice qué hacer. Se prueba con `scripts/sandbox.ps1 -SinRed`.
- Si Tauri cambia el modo o el nombre de `WEBVIEW2BOOTSTRAPPERPATH`, el hook para la compilación con un `!error` que lo explica.
- Si una versión nueva de Tauri arregla su comprobación, la nuestra no estorba: ve WebView2 instalado y no hace nada.
- Sin comprobar en un Windows real sin WebView2: que el bootstrapper lo instale sin administrador. Microsoft lo documenta así, y es la prueba A1–A3 en el Sandbox.
