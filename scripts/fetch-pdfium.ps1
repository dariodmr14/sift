# Descarga pdfium.dll (el motor de PDF de Chrome) a apps/desktop/src-tauri/pdfium/.
#
# Sift lo usa para leer el texto de los PDF. La DLL no está en el repositorio: se baja
# una versión FIJADA de https://github.com/bblanchon/pdfium-binaries y se comprueba su
# SHA-256 antes de usarla. Ver docs/adr/0008-distribucion-de-pdfium.md.
#
# Uso (desde la raíz del repo):   pwsh scripts/fetch-pdfium.ps1
# Para cambiar de versión: cambia $Version y $Sha256 (el hash sale en la página de la
# release, campo "digest" del archivo pdfium-win-x64.tgz) y la feature `pdfium_XXXX` de
# pdfium-render en Cargo.toml, que tiene que ser la misma versión.

$ErrorActionPreference = "Stop"

$Version = "8125"   # = feature `pdfium_latest` de pdfium-render 0.9.5
$Sha256 = "9a964261760cff52570aabf038ab346d3a5034dd8b30d0d5fe8861235d356b8b"
$Url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/$Version/pdfium-win-x64.tgz"

$Target = Join-Path $PSScriptRoot "..\apps\desktop\src-tauri\pdfium"
$Dll = Join-Path $Target "pdfium.dll"
$Stamp = Join-Path $Target "pdfium.version"

if ((Test-Path $Dll) -and (Test-Path $Stamp) -and ((Get-Content $Stamp -Raw).Trim() -eq $Version)) {
    Write-Host "pdfium.dll $Version ya está en $Target"
    exit 0
}

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) "sift-pdfium-$Version"
New-Item -ItemType Directory -Force $Tmp | Out-Null
$Archive = Join-Path $Tmp "pdfium-win-x64.tgz"

Write-Host "Descargando PDFium $Version..."
Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing

$Actual = (Get-FileHash $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($Actual -ne $Sha256) {
    Remove-Item $Archive -Force
    throw "El SHA-256 no coincide: esperado $Sha256, descargado $Actual. No se usa el archivo."
}

# tar viene con Windows 10 (1803+) y entiende .tgz.
tar -xzf $Archive -C $Tmp
New-Item -ItemType Directory -Force $Target | Out-Null
Copy-Item (Join-Path $Tmp "bin\pdfium.dll") $Dll -Force
Copy-Item (Join-Path $Tmp "LICENSE") (Join-Path $Target "LICENSE-pdfium.txt") -Force
Set-Content $Stamp $Version -Encoding ascii
Remove-Item $Tmp -Recurse -Force

Write-Host "Listo: $Dll (PDFium $Version, SHA-256 comprobado)"
