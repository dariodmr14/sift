# CI local de Sift: ejecuta lo mismo que .github/workflows/ci.yml, en el mismo orden.
#
# Uso (desde cualquier carpeta):   powershell -ExecutionPolicy Bypass -File scripts/ci.ps1
#
# Para al primer paso que falle y, al final, muestra un resumen con lo que pasó y cuánto
# tardó cada paso. Sale con código 0 si todo está en verde y 1 si algo falló.
#
# Opciones (solo para ir más rápido mientras se trabaja; antes de un merge, sin ninguna):
#   -SkipInstall   no ejecuta `npm ci` (usa node_modules tal como esté)
#   -SkipBuild     no compila la app en release (el paso más lento)
#   -E2E           añade la prueba de humo de extremo a extremo (scripts/e2e.mjs): arranca la
#                  versión de desarrollo y ABRE VENTANAS en la pantalla, así que no va por
#                  defecto (pruebas en pantalla, con permiso de Darío).

param(
    [switch]$SkipInstall,
    [switch]$SkipBuild,
    [switch]$E2E
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Desktop = Join-Path $Root "apps\desktop"
$env:CARGO_TERM_COLOR = "always"

# Cada paso: nombre, carpeta y comando (programa y argumentos separados, sin shell).
$Steps = @(
    @{ Name = "Rust: formato";           Dir = $Root;    Cmd = "cargo"; Args = @("fmt", "--all", "--check") },
    @{ Name = "PDFium (version fijada)"; Dir = $Root;    Cmd = "powershell"; Args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $Root "scripts\fetch-pdfium.ps1")) },
    @{ Name = "Rust: clippy";            Dir = $Root;    Cmd = "cargo"; Args = @("clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings") },
    @{ Name = "Rust: tests";             Dir = $Root;    Cmd = "cargo"; Args = @("test", "--workspace", "--locked") },
    # Fallos de seguridad conocidos en las dependencias (base de RustSec). Falla si hay
    # alguna vulnerabilidad; los avisos de «sin mantener» se enseñan, pero no paran.
    @{ Name = "Rust: dependencias";      Dir = $Root;    Cmd = "cargo"; Args = @("audit") },
    @{ Name = "UI: npm ci";              Dir = $Desktop; Cmd = "npm.cmd"; Args = @("ci"); Skip = $SkipInstall },
    @{ Name = "UI: dependencias";        Dir = $Desktop; Cmd = "npm.cmd"; Args = @("audit", "--omit=dev", "--audit-level=moderate") },
    @{ Name = "UI: prettier";            Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "format:check") },
    @{ Name = "UI: eslint";              Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "lint") },
    @{ Name = "UI: tsc";                 Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "typecheck") },
    @{ Name = "UI: vitest";              Dir = $Desktop; Cmd = "npm.cmd"; Args = @("test") },
    @{ Name = "Build de la app";         Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "tauri", "build", "--", "--no-bundle"); Skip = $SkipBuild },
    @{ Name = "E2E (abre ventanas)";     Dir = $Root;    Cmd = "node"; Args = @((Join-Path $Root "scripts\e2e.mjs")); Skip = -not $E2E }
)

$Results = @()
$Failed = $false
$Started = Get-Date

# `cargo audit` no viene con Rust: si falta, se dice cómo instalarlo en vez de fallar raro.
& cargo audit --version *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Falta cargo-audit (revisa fallos de seguridad en las dependencias). Instálalo una vez con:" -ForegroundColor Red
    Write-Host "  cargo install cargo-audit --locked" -ForegroundColor Red
    exit 1
}

# `npm ci` borra node_modules entero y lo vuelve a instalar. Si algo lo tiene abierto
# (`npm run tauri dev`, que deja Vite en marcha), Windows no deja borrar algunos archivos
# y npm se para A MEDIAS: node_modules queda roto también para lo que está en marcha. Mejor
# avisar antes y no tocar nada.
if (-not $SkipInstall) {
    $modules = Join-Path $Desktop "node_modules"
    $using = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -and $_.CommandLine.Contains($modules) }
    if ($using) {
        Write-Host ""
        Write-Host "Hay procesos usando $modules (¿npm run tauri dev abierto?):" -ForegroundColor Red
        $using | ForEach-Object { Write-Host "  PID $($_.ProcessId)" -ForegroundColor Red }
        Write-Host "Ciérralos y vuelve a ejecutar la CI: npm ci no puede reinstalar mientras tanto." -ForegroundColor Red
        exit 1
    }
}

foreach ($Step in $Steps) {
    if ($Failed) {
        $Results += [pscustomobject]@{ Paso = $Step.Name; Estado = "sin ejecutar"; Tiempo = "" }
        continue
    }
    if ($Step.Skip) {
        $Results += [pscustomobject]@{ Paso = $Step.Name; Estado = "saltado"; Tiempo = "" }
        continue
    }
    Write-Host ""
    Write-Host "==> $($Step.Name)" -ForegroundColor Cyan
    $t = Get-Date
    Push-Location $Step.Dir
    try {
        & $Step.Cmd @($Step.Args)
        $ok = ($LASTEXITCODE -eq 0)
    } catch {
        Write-Host $_ -ForegroundColor Red
        $ok = $false
    } finally {
        Pop-Location
    }
    $secs = [math]::Round(((Get-Date) - $t).TotalSeconds)
    if ($ok) {
        $Results += [pscustomobject]@{ Paso = $Step.Name; Estado = "OK"; Tiempo = "$secs s" }
    } else {
        $Results += [pscustomobject]@{ Paso = $Step.Name; Estado = "FALLA"; Tiempo = "$secs s" }
        $Failed = $true
    }
}

$total = [math]::Round(((Get-Date) - $Started).TotalMinutes, 1)
Write-Host ""
Write-Host "==== Resumen de la CI local ($total min) ====" -ForegroundColor Cyan
foreach ($r in $Results) {
    $color = switch ($r.Estado) { "OK" { "Green" } "FALLA" { "Red" } default { "DarkGray" } }
    Write-Host ("  {0,-26} {1,-13} {2}" -f $r.Paso, $r.Estado, $r.Tiempo) -ForegroundColor $color
}
if ($Failed) {
    Write-Host "CI local en ROJO: arregla el paso que falla y vuelve a ejecutarla." -ForegroundColor Red
    exit 1
}
if ($SkipInstall -or $SkipBuild) {
    Write-Host "CI local en verde, pero con pasos saltados: antes de un merge, ejecútala completa." -ForegroundColor Yellow
} else {
    Write-Host "CI local en VERDE." -ForegroundColor Green
}
exit 0
