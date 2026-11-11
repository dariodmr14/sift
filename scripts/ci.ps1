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

param(
    [switch]$SkipInstall,
    [switch]$SkipBuild
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
    @{ Name = "UI: npm ci";              Dir = $Desktop; Cmd = "npm.cmd"; Args = @("ci"); Skip = $SkipInstall },
    @{ Name = "UI: prettier";            Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "format:check") },
    @{ Name = "UI: eslint";              Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "lint") },
    @{ Name = "UI: tsc";                 Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "typecheck") },
    @{ Name = "UI: vitest";              Dir = $Desktop; Cmd = "npm.cmd"; Args = @("test") },
    @{ Name = "Build de la app";         Dir = $Desktop; Cmd = "npm.cmd"; Args = @("run", "tauri", "build", "--", "--no-bundle"); Skip = $SkipBuild }
)

$Results = @()
$Failed = $false
$Started = Get-Date

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
