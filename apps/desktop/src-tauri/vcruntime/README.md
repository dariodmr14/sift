# vcruntime/

Aquí van las DLL del runtime de Visual C++ que necesita Sift para arrancar: `msvcp140.dll`, `msvcp140_1.dll`, `vcruntime140.dll` y `vcruntime140_1.dll`.

Las pide ONNX Runtime (la búsqueda por significado), que va dentro de `sift-desktop.exe` pero está compilado con el runtime de C++ en DLL. Un Windows recién instalado no las trae: sin ellas, Sift no se abre («no se encontró MSVCP140.dll»).

No están en el repositorio. Se copian de la instalación de Visual Studio (o sus Build Tools), que ya hace falta para compilar, con:

```powershell
pwsh scripts/vc-runtime.ps1
```

El instalador las deja junto al ejecutable (`bundle.resources` en `tauri.conf.json`), sin pedir permisos de administrador. `scripts/comprobar-instalador.ps1` comprueba que las lleva todas y que son lo bastante nuevas.

Por qué así: [ADR 0020](../../../../docs/adr/0020-runtime-de-visual-cpp.md).
