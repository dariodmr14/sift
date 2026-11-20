# Abre Windows Sandbox con el instalador de Sift a mano, para probarlo en un Windows limpio
# (sin nada instalado y sin datos). Ver docs/prueba-instalacion.md.
#
# Uso, desde la raíz del repo, después de generar el instalador (scripts/instalar.ps1 o
# `npm run tauri build` en apps/desktop):
#   powershell -ExecutionPolicy Bypass -File scripts/sandbox.ps1
#
# Genera la configuración (.wsb) con la ruta de ESTE equipo al vuelo, en la carpeta temporal:
# así el repo no guarda ninguna ruta personal. La carpeta del instalador entra en el Sandbox
# en solo lectura, y al arrancar se abre en el Explorador. Al cerrar el Sandbox se borra todo
# lo que se hizo dentro.

$ErrorActionPreference = "Stop"

$sandbox = Join-Path $env:windir "System32\WindowsSandbox.exe"
if (-not (Test-Path $sandbox)) {
    Write-Host "Windows Sandbox no está activado."
    Write-Host "Actívalo en «Activar o desactivar las características de Windows» → «Espacio aislado de Windows» y reinicia."
    exit 1
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$bundle = Join-Path $root "target\release\bundle\nsis"
$setup = Get-ChildItem $bundle -Filter "Sift_*_x64-setup.exe" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) {
    Write-Host "No hay instalador en $bundle. Genéralo antes (ver docs/instalar.md)."
    exit 1
}
Write-Host "Instalador: $($setup.Name)"

$inside = "C:\Users\WDAGUtilityAccount\Desktop\Instalador de Sift"
# Los datos de demostración (inventados), para tener algo que buscar: en el Escritorio, para
# añadirlos con «Elegir otra carpeta…». También en solo lectura.
$demo = Join-Path $root "demo"
$demoInside = "C:\Users\WDAGUtilityAccount\Desktop\Datos de prueba"
# Dos Espacios para importar en la prueba de seguridad (ver docs/prueba-instalacion.md).
$checks = Join-Path $root "scripts\sandbox-pruebas"
$checksInside = "C:\Users\WDAGUtilityAccount\Desktop\Pruebas de seguridad"
$wsb = @"
<Configuration>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$bundle</HostFolder>
      <SandboxFolder>$inside</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
    <MappedFolder>
      <HostFolder>$demo</HostFolder>
      <SandboxFolder>$demoInside</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
    <MappedFolder>
      <HostFolder>$checks</HostFolder>
      <SandboxFolder>$checksInside</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>explorer.exe "$inside"</Command>
  </LogonCommand>
</Configuration>
"@
$file = Join-Path $env:TEMP "sift-prueba.wsb"
Set-Content -Path $file -Value $wsb -Encoding UTF8
Start-Process $file
Write-Host "Abriendo Windows Sandbox (tarda unos segundos)…"
