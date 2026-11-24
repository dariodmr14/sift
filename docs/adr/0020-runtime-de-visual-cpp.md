# ADR 0020 — El runtime de Visual C++ va dentro del instalador

- **Estado:** aceptada
- **Fecha:** 2026-11-24

## Contexto

Al probar el instalador 1.0.0 en Windows Sandbox (un Windows limpio), Sift no arrancaba: «La ejecución de código no puede continuar porque no se encontró MSVCP140.dll». En el PC de Darío no pasa porque Visual Studio instala el runtime de Visual C++ en `System32`.

Con `dumpbin /dependents` sobre el ejecutable de la versión 1.0.0:

| Binario | Qué pide que no traiga un Windows limpio | Por qué |
|---|---|---|
| `sift-desktop.exe` | `MSVCP140.dll`, `MSVCP140_1.dll` | ONNX Runtime (ADR 0009) va enlazado dentro del ejecutable, pero está compilado con la biblioteca estándar de C++ en DLL (`/MD`) |
| `msvcp140.dll` | `VCRUNTIME140.dll`, `VCRUNTIME140_1.dll` | Las necesita a su vez |
| `msvcp140_1.dll` | `MSVCP140.dll`, `VCRUNTIME140.dll` | Ídem |
| `pdfium.dll` | Nada | Solo usa DLL del sistema |

Lo demás viene con Windows:
- Los `api-ms-win-crt-*` son el CRT universal (UCRT), de serie desde Windows 10.
- El ejecutable no pide `vcruntime140.dll` directamente porque Tauri enlaza esa parte en estático.
- `DirectML.dll`, `d3d12.dll` y `dxgi.dll`: los binarios de ONNX Runtime para Windows siempre vienen con DirectML (la aceleración por GPU). Estas DLL vienen con Windows 10 desde la 1903 (mayo de 2019), y la `DirectML.dll` del sistema exporta lo que se importa (`DMLCreateDevice1`). Sift no las usa: todo va por la CPU.
- **WebView2.** Viene con Windows 11. En Windows 10 suele llegar con las actualizaciones de Edge, pero puede faltar (el Sandbox no lo trae). Se resuelve aparte, en el [ADR 0021](0021-webview2.md).

## Decisión

Las cuatro DLL (`msvcp140.dll`, `msvcp140_1.dll`, `vcruntime140.dll` y `vcruntime140_1.dll`) **se copian junto a `sift-desktop.exe`** dentro del instalador. Es el *despliegue local* que permite Microsoft: son «código redistribuible» de Visual Studio. Windows busca primero en la carpeta del ejecutable, así que se usan esas aunque el sistema no tenga otras.

- `scripts/vc-runtime.ps1` las copia de la instalación de Visual Studio (o sus Build Tools), que ya hace falta para compilar, a `apps/desktop/src-tauri/vcruntime/`. No descarga nada, y no se suben al repo.
- `bundle.resources` (`tauri.conf.json`) las deja en la raíz de la carpeta de instalación. El desinstalador las borra con el resto.
- `scripts/comprobar-instalador.ps1` abre el instalador con 7-Zip, sin instalarlo, y sigue con `dumpbin` las DLL que usan el ejecutable, PDFium y las DLL que lleva. Falla si:
  - falta una del runtime de Visual C++;
  - una de las que lleva es más antigua de lo necesario: anterior a la 14.40 (lo compilado con Visual Studio 2022 17.10 o posterior puede cerrarse al usar `std::mutex` con un runtime anterior, y eso no se ve en las funciones que exporta) o sin alguna de las funciones que se importan;
  - se pide una DLL que no lleva el instalador ni trae Windows.

  Se ejecuta en `scripts/ci.ps1` (que ahora genera el instalador), en el workflow de CI y en el de Release, antes de crear la Release.

| Opción | A favor | En contra |
|---|---|---|
| **Despliegue local (copiar las DLL)** | Sin permisos de administrador, sin internet y sin pasos para el usuario; +0,9 MB (menos en el instalador, que va comprimido); lo comprueba la CI | Esas copias no las actualiza Windows Update: se renuevan con cada versión de Sift, que las copia del Visual Studio con que se compila |
| Instalar el Visual C++ Redistributable (`vc_redist.x64.exe`) desde el instalador | Las actualiza Windows Update | Pide permisos de administrador, y el instalador de Sift es por usuario. +25 MB o una descarga más |
| CRT estático (`+crt-static`) | Sin DLL que copiar | ONNX Runtime no se publica así: los binarios de `ort` solo existen con el CRT en DLL, y habría que compilar ONNX Runtime a mano (una hora de compilación y mantenerlo en cada versión). Además, cambiaría cómo se compilan el resto de dependencias en C y C++ (tokenizadores, usearch) |
| Pedir al usuario que instale el Redistributable | Nada que mantener | Un primer arranque roto con un error críptico: justo lo que la prueba en Sandbox ha encontrado |

## Consecuencias

- Hace falta ejecutar `scripts/vc-runtime.ps1` una vez antes de compilar, como `fetch-pdfium.ps1`: `tauri-build` falla si falta un recurso. Lo hacen `ci.ps1`, `instalar.ps1` y los dos workflows.
- El runtime que se copia es el de la versión de Visual Studio con que se compila (14.51 en el PC de Darío; en GitHub, el del runner). Si es más antiguo que el que usó ONNX Runtime, la comprobación del instalador lo detecta.
- Si una versión nueva de ONNX Runtime pide otra DLL (p. ej. `msvcp140_2.dll`), la comprobación falla y dice cuál: se añade a `vc-runtime.ps1` y a `bundle.resources`.
- Windows 10 anterior a la 1903 no tiene `DirectML.dll` y Sift no arrancaría. Todas esas versiones están fuera de soporte desde 2020-2021.
- La prueba en Windows Sandbox (`docs/prueba-instalacion.md`, A3) es la comprobación de verdad: un Windows sin Visual Studio.
