# Pulsa el atajo de la versión de desarrollo (Ctrl+Alt+Space) para mostrar su barra. Lo
# usa scripts/capturas.mjs cuando la barra se ha ocultado (se oculta al perder el foco).
$ErrorActionPreference = "Stop"
Add-Type -Namespace Sift -Name Keys -MemberDefinition '[DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, System.UIntPtr extra);'
$up = 2
foreach ($vk in 0x11, 0x12, 0x20) { [Sift.Keys]::keybd_event($vk, 0, 0, [System.UIntPtr]::Zero) }
foreach ($vk in 0x20, 0x12, 0x11) { [Sift.Keys]::keybd_event($vk, 0, $up, [System.UIntPtr]::Zero) }
