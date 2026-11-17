# Captura un trozo de la pantalla tal como se ve (con el vidrio de Windows, que el
# WebView no ve) y lo guarda en JPEG. Lo usa scripts/capturas.mjs.
#
# Uso: powershell -File scripts/captura-region.ps1 -X 100 -Y 100 -W 800 -H 500 -Out a.jpg
# Las coordenadas son píxeles físicos de la pantalla.

param(
    [Parameter(Mandatory)][int]$X,
    [Parameter(Mandatory)][int]$Y,
    [Parameter(Mandatory)][int]$W,
    [Parameter(Mandatory)][int]$H,
    [Parameter(Mandatory)][string]$Out
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
# Sin esto, con la escala de Windows por encima del 100 %, las coordenadas se reescalan.
Add-Type -Namespace Sift -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][Sift.Dpi]::SetProcessDPIAware()

$bmp = New-Object System.Drawing.Bitmap $W, $H
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($X, $Y, 0, 0, $bmp.Size)
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq "image/jpeg" }
$params = New-Object System.Drawing.Imaging.EncoderParameters 1
$params.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 85L
$bmp.Save($Out, $codec, $params)
$g.Dispose()
$bmp.Dispose()
