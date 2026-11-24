# Comprueba que el instalador lleva todo lo que Sift necesita para arrancar en un Windows
# recién instalado, sin Visual Studio ni el «Visual C++ Redistributable».
#
# Abre el instalador con 7-Zip (sin instalar nada) y, con dumpbin, sigue las DLL que usa
# sift-desktop.exe y las que usan esas DLL. Falla si:
#   - falta una DLL del runtime de Visual C++ (msvcp140, vcruntime140…) junto al ejecutable;
#   - una de las que lleva es más antigua de lo necesario (anterior a la 14.40 o sin alguna
#     de las funciones que se usan);
#   - se usa una DLL que no lleva el instalador ni trae Windows;
#   - falta pdfium\pdfium.dll;
#   - falta el instalador de WebView2 (MicrosoftEdgeWebview2Setup.exe) o no lleva una firma
#     válida de Microsoft (ADR 0021).
# Ver docs/adr/0020-runtime-de-visual-cpp.md.
#
# Uso (desde la raíz del repo):   pwsh scripts/comprobar-instalador.ps1 [-Installer <ruta>]
# Sin -Installer, usa target\release\bundle\nsis\Sift_<versión de tauri.conf.json>_x64-setup.exe.

param([string]$Installer)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "visual-studio.ps1")
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if (-not $Installer) {
    $conf = Join-Path $Root "apps\desktop\src-tauri\tauri.conf.json"
    $version = (Get-Content $conf -Raw | ConvertFrom-Json).version
    $Installer = Join-Path $Root "target\release\bundle\nsis\Sift_${version}_x64-setup.exe"
}
if (-not (Test-Path $Installer)) { throw "No existe el instalador $Installer (¿se compiló con «tauri build»?)." }
$Installer = (Resolve-Path $Installer).Path

$SevenZip = (Get-Command 7z.exe -ErrorAction SilentlyContinue).Source
if (-not $SevenZip) { $SevenZip = Join-Path $env:ProgramFiles "7-Zip\7z.exe" }
if (-not (Test-Path $SevenZip)) { throw "Hace falta 7-Zip (https://www.7-zip.org) para abrir el instalador sin instalarlo." }
$Dumpbin = Get-DumpbinPath

# El runtime de Visual C++: no viene con Windows, así que el instalador tiene que llevarlo.
$VcRuntime = '^(msvcp|vcruntime|concrt|vccorlib|vcomp|vcamp)\d'
# Vienen con Windows 10 (1903 o posterior) y 11, aunque alguna no esté en el runner de GitHub.
# DirectML la pide ONNX Runtime al cargar, pero Sift no la usa (todo va por la CPU).
$Inbox = @("directml.dll", "d3d12.dll", "dxgi.dll", "dxcore.dll")
$System32 = Join-Path $env:SystemRoot "System32"
# Lo compilado con Visual Studio 2022 17.10 (MSVC 14.40) o posterior (puede ser el caso de
# ONNX Runtime) puede cerrarse al usar std::mutex con un msvcp140.dll anterior. No se nota en las funciones
# que exporta (son las mismas), así que se mira la versión.
$MinVcRuntime = [version]"14.40.0"

# Lo que importa un binario: @{ "msvcp140.dll" = @("?foo@std@@…", "#12", …) } (en minúsculas).
function Get-Imports([string]$file) {
    $out = & $Dumpbin /nologo /imports $file
    if ($LASTEXITCODE -ne 0) { throw "dumpbin no pudo leer $file" }
    $imports = @{}
    $current = $null
    foreach ($line in $out) {
        if ($line -match '^\s+Summary\s*$') { break }
        if ($line -match '^    (\S+\.dll)\s*$') {
            $current = $Matches[1].ToLowerInvariant()
            if (-not $imports.ContainsKey($current)) { $imports[$current] = [System.Collections.Generic.List[string]]::new() }
        } elseif ($current -and $line -match '^\s+Ordinal\s+(\d+)\s*$') {
            $imports[$current].Add("#$($Matches[1])")
        } elseif ($current -and $line -match '^\s+[0-9A-Fa-f]+\s+(\S+)\s*$' -and $Matches[1] -ne "Characteristics") {
            $imports[$current].Add($Matches[1])
        }
    }
    return $imports
}

# Lo que exporta una DLL: nombres y ordinales («#12»).
function Get-Exports([string]$file) {
    $out = & $Dumpbin /nologo /exports $file
    if ($LASTEXITCODE -ne 0) { throw "dumpbin no pudo leer $file" }
    $exports = [System.Collections.Generic.HashSet[string]]::new()
    $inTable = $false
    foreach ($line in $out) {
        if ($line -match '^\s+ordinal\s+hint\s+RVA\s+name') { $inTable = $true; continue }
        if (-not $inTable) { continue }
        if ($line -match '^\s+Summary\s*$') { break }
        if ($line -match '^\s+(\d+)\s+[0-9A-Fa-f]+\s+(?:[0-9A-Fa-f]{8}\s+)?(\S+)') {
            [void]$exports.Add("#$($Matches[1])")
            [void]$exports.Add($Matches[2])
        } elseif ($line -match '^\s+(\d+)\s+[0-9A-Fa-f]{8}\s+\[NONAME\]') {
            [void]$exports.Add("#$($Matches[1])")
        }
    }
    return $exports
}

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("sift-instalador-" + [guid]::NewGuid().ToString("N"))
$errors = @()
$found = @()
try {
    Write-Host "Abriendo $Installer"
    & $SevenZip x -y "-o$Tmp" $Installer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "7-Zip no pudo abrir $Installer" }

    # Lo que queda en la raíz es la carpeta de instalación de Sift.
    $exe = Join-Path $Tmp "sift-desktop.exe"
    if (-not (Test-Path $exe)) { throw "El instalador no lleva sift-desktop.exe en su raíz." }
    $pdfium = Join-Path $Tmp "pdfium\pdfium.dll"
    if (-not (Test-Path $pdfium)) { $errors += "Falta pdfium\pdfium.dll (scripts/fetch-pdfium.ps1)." }

    # El bootstrapper de WebView2 (webviewInstallMode: embedBootstrapper). NSIS lo guarda con
    # destino $TEMP: solo se usa si WebView2 falta.
    $webview2 = Join-Path $Tmp '$TEMP\MicrosoftEdgeWebview2Setup.exe'
    if (-not (Test-Path -LiteralPath $webview2)) {
        $errors += "Falta el instalador de WebView2: en un Windows sin WebView2 Sift no se abriría (webviewInstallMode tiene que ser embedBootstrapper, ADR 0021)."
    } else {
        $sig = Get-AuthenticodeSignature -LiteralPath $webview2
        if ($sig.Status -ne "Valid" -or $sig.SignerCertificate.Subject -notmatch '^CN=Microsoft Corporation,') {
            $errors += "El instalador de WebView2 no lleva una firma válida de Microsoft ($($sig.Status), $($sig.SignerCertificate.Subject))."
        }
    }

    # Recorre las dependencias: el ejecutable, PDFium y las DLL que lleve el instalador.
    $queue = [System.Collections.Generic.Queue[string]]::new()
    $queue.Enqueue($exe)
    if (Test-Path $pdfium) { $queue.Enqueue($pdfium) }
    $seen = @{}
    while ($queue.Count -gt 0) {
        $file = $queue.Dequeue()
        $name = Split-Path $file -Leaf
        $imports = Get-Imports $file
        foreach ($dll in $imports.Keys) {
            if ($dll -like "api-ms-win-*" -or $dll -like "ext-ms-*") { continue } # UCRT y API sets: Windows
            # Windows busca las DLL primero en la carpeta del ejecutable (la de instalación).
            $bundled = Join-Path $Tmp $dll
            if (Test-Path $bundled) {
                $exports = Get-Exports $bundled
                $missing = @($imports[$dll] | Where-Object { -not $exports.Contains($_) })
                if ($missing) {
                    $errors += "$dll es más antigua de lo que necesita ${name}: le faltan $($missing.Count) funciones (p. ej. $($missing[0])). Copia un runtime más nuevo (scripts/vc-runtime.ps1)."
                }
                if (-not $seen.ContainsKey($dll)) {
                    $seen[$dll] = $true
                    $fileVersion = (Get-Item $bundled).VersionInfo
                    $v = [version]::new($fileVersion.FileMajorPart, $fileVersion.FileMinorPart, $fileVersion.FileBuildPart)
                    if ($dll -match $VcRuntime -and $v -lt $MinVcRuntime) {
                        $errors += "$dll es la versión $v; hace falta $MinVcRuntime o posterior. Compila con un Visual Studio más nuevo (scripts/vc-runtime.ps1)."
                    }
                    $found += "$dll $v"
                    $queue.Enqueue($bundled)
                }
            } elseif ($dll -match $VcRuntime) {
                $errors += "$name necesita $dll y el instalador no la lleva: en un Windows sin Visual C++ Sift no arrancaría («no se encontró $dll»)."
            } elseif ($Inbox -notcontains $dll -and -not (Test-Path (Join-Path $System32 $dll))) {
                $errors += "$name necesita $dll, que ni lleva el instalador ni trae Windows."
            }
        }
    }
} finally {
    if (Test-Path $Tmp) { Remove-Item $Tmp -Recurse -Force }
}

if ($errors) {
    Write-Host ""
    Write-Host "El instalador NO vale para un Windows limpio:" -ForegroundColor Red
    $errors | Select-Object -Unique | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}
Write-Host "Instalador correcto: lleva pdfium.dll, el instalador de WebView2 (firmado por Microsoft) y el runtime de Visual C++ ($($found -join ', '))." -ForegroundColor Green
exit 0
