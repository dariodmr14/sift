# Funciones para encontrar las herramientas de Visual Studio (C++). Se cargan con
#   . (Join-Path $PSScriptRoot "visual-studio.ps1")
# desde vc-runtime.ps1 y comprobar-instalador.ps1. Funcionan igual en el PC de desarrollo y
# en el runner de GitHub (windows-latest), que traen Visual Studio o sus Build Tools.

# Carpeta de la instalación de Visual Studio más reciente que tiene el compilador de C++
# (el mismo que usa Rust para enlazar).
function Get-VsInstallPath {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) {
        throw "No se encuentra vswhere.exe: hacen falta Visual Studio o sus Build Tools con «Desarrollo de escritorio con C++» (docs/desarrollo.md)."
    }
    $path = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $path) {
        throw "Visual Studio está instalado, pero sin las herramientas de C++ (x64): añade «Desarrollo de escritorio con C++»."
    }
    return ($path | Select-Object -First 1).Trim()
}

# Carpeta con las DLL del runtime de Visual C++ que Microsoft permite copiar junto a una app
# (VC\Redist\MSVC\<versión>\x64\Microsoft.VC14x.CRT).
function Get-VcRedistCrtDir {
    $vs = Get-VsInstallPath
    $versionFile = Join-Path $vs "VC\Auxiliary\Build\Microsoft.VCRedistVersion.default.txt"
    if (-not (Test-Path $versionFile)) { throw "No se encuentra $versionFile (¿faltan las herramientas de C++?)." }
    $version = (Get-Content $versionFile -Raw).Trim()
    $x64 = Join-Path $vs "VC\Redist\MSVC\$version\x64"
    $crt = Get-ChildItem $x64 -Directory -Filter "Microsoft.VC*.CRT" -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if (-not $crt) { throw "No se encuentra la carpeta Microsoft.VC*.CRT en $x64." }
    return $crt.FullName
}

# dumpbin.exe (lee qué DLL y qué funciones usa un ejecutable).
function Get-DumpbinPath {
    $vs = Get-VsInstallPath
    $versionFile = Join-Path $vs "VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt"
    if (-not (Test-Path $versionFile)) { throw "No se encuentra $versionFile (¿faltan las herramientas de C++?)." }
    $version = (Get-Content $versionFile -Raw).Trim()
    $dumpbin = Join-Path $vs "VC\Tools\MSVC\$version\bin\Hostx64\x64\dumpbin.exe"
    if (-not (Test-Path $dumpbin)) { throw "No se encuentra $dumpbin." }
    return $dumpbin
}
