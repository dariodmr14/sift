# pdfium/

Aquí va `pdfium.dll`, el motor de PDF de Chrome, que Sift usa para leer el texto de los PDF.

No está en el repositorio (es un binario de varios MB). Se descarga, en una versión fijada y con su SHA-256 comprobado, con:

```powershell
pwsh scripts/fetch-pdfium.ps1
```

Esta carpeta entera se empaqueta con la app como recurso de Tauri (`bundle.resources` en `tauri.conf.json`), así que el instalador lleva la DLL dentro y **Sift nunca descarga nada al ejecutarse**.

Sin la DLL, la app funciona igual, pero los PDF se encuentran solo por su nombre (Ajustes → Carpetas lo avisa).

Por qué así: [ADR 0008](../../../../docs/adr/0008-distribucion-de-pdfium.md).
