# Copia el runtime de Visual C++ que necesita Sift a apps/desktop/src-tauri/vcruntime/.
#
# ONNX Runtime (la búsqueda por significado) va dentro de sift-desktop.exe, pero está
# compilado con el runtime de C++ en DLL: sin msvcp140.dll y compañía, en un Windows sin
# Visual Studio ni el «Visual C++ Redistributable» Sift no llega a arrancar («no se encontró
# MSVCP140.dll»). El instalador las lleva junto al ejecutable (despliegue local, como permite
# Microsoft), sin pedir permisos de administrador. Ver docs/adr/0020-runtime-de-visual-cpp.md.
#
# No descarga nada: las copia de la instalación de Visual Studio (o sus Build Tools) que ya
# hace falta para compilar. scripts/comprobar-instalador.ps1 comprueba después que el
# instalador las lleva todas y que son lo bastante nuevas.
#
# Uso (desde la raíz del repo):   pwsh scripts/vc-runtime.ps1

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "visual-studio.ps1")

# msvcp140 y msvcp140_1 las importa sift-desktop.exe; vcruntime140 y vcruntime140_1, a su vez,
# msvcp140. Si una versión nueva de ONNX Runtime pide otra, la comprobación del instalador lo
# dirá y bastará con añadirla aquí y en `bundle.resources` (tauri.conf.json).
$Dlls = @("msvcp140.dll", "msvcp140_1.dll", "vcruntime140.dll", "vcruntime140_1.dll")

$Source = Get-VcRedistCrtDir
$Target = Join-Path $PSScriptRoot "..\apps\desktop\src-tauri\vcruntime"
New-Item -ItemType Directory -Force $Target | Out-Null

foreach ($dll in $Dlls) {
    $from = Join-Path $Source $dll
    if (-not (Test-Path $from)) { throw "No se encuentra $from." }
    Copy-Item $from (Join-Path $Target $dll) -Force
}

$version = (Get-Item (Join-Path $Target "msvcp140.dll")).VersionInfo.FileVersion
Set-Content (Join-Path $Target "vcruntime.version") $version -Encoding ascii
Write-Host "Listo: runtime de Visual C++ $version en $((Resolve-Path $Target).Path)"
Write-Host "  (copiado de $Source)"
