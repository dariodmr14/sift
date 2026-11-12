# Compila la versión estable de Sift desde `main` y abre su instalador.
#
# Uso (desde cualquier carpeta):   powershell -ExecutionPolicy Bypass -File scripts/instalar.ps1
#
# Qué hace:
#   1. Comprueba que estás en `main` sin cambios a medias y trae lo último (`git pull`).
#   2. Baja PDFium (versión y hash fijados) e instala las dependencias de la UI.
#   3. Compila en modo release y genera el instalador NSIS
#      (target\release\bundle\nsis\Sift_<versión>_x64-setup.exe).
#   4. Abre el instalador. Si Sift ya está instalado, lo actualiza en el sitio: tus datos
#      (%APPDATA%\dev.sift.app) no se tocan.
#
# Opciones:
#   -SoloCompilar   genera el instalador pero no lo abre.

param([switch]$SoloCompilar)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Desktop = Join-Path $Root "apps\desktop"

function Step($text) { Write-Host ""; Write-Host "==> $text" -ForegroundColor Cyan }
function Run($exe, [string[]]$arguments) {
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "falló: $exe $($arguments -join ' ')" }
}

Push-Location $Root
try {
    Step "Comprobando que estás en main y al día"
    $branch = (git rev-parse --abbrev-ref HEAD).Trim()
    if ($branch -ne "main") {
        throw "estás en «$branch»: la versión estable se instala desde main (git checkout main)."
    }
    if (git status --porcelain) {
        throw "hay cambios sin guardar en el repositorio: guárdalos o descártalos antes."
    }
    Run git @("pull", "--ff-only")

    Step "PDFium (para leer los PDF)"
    Run powershell @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $Root "scripts\fetch-pdfium.ps1"))

    Step "Dependencias de la interfaz"
    Push-Location $Desktop
    try { Run npm.cmd @("ci") } finally { Pop-Location }

    Step "Compilando la versión estable (tarda unos minutos)"
    Push-Location $Desktop
    try { Run npm.cmd @("run", "tauri", "build") } finally { Pop-Location }

    $setup = Get-ChildItem (Join-Path $Root "target\release\bundle\nsis") -Filter "*-setup.exe" |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $setup) { throw "no se encontró el instalador en target\release\bundle\nsis" }
    Write-Host ""
    Write-Host "Instalador listo: $($setup.FullName)" -ForegroundColor Green

    if (-not $SoloCompilar) {
        Step "Abriendo el instalador"
        Write-Host "Windows puede avisar de que el programa no está firmado (SmartScreen):"
        Write-Host "«Más información» → «Ejecutar de todas formas»."
        Start-Process $setup.FullName
    }
} finally {
    Pop-Location
}
