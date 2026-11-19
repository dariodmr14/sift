# Junta las capturas de la introducción (docs/capturas/introduccion/) en una sola lámina.
# Uso, desde la raíz del repo:
#   powershell -File scripts/lamina-introduccion.ps1 -Dir docs/capturas/introduccion -Out docs/capturas/introduccion/recorrido.jpg
param([string]$Dir, [string]$Out)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
$names = "1-atajo.jpg","1-atajo-hecho.jpg","2-carpetas.jpg","3-buscar.jpg","4-inteligente.jpg","5-espacio.jpg","5-espacio-hecho.jpg","6-final.jpg"
$imgs = $names | ForEach-Object { [System.Drawing.Image]::FromFile((Join-Path $Dir $_)) }
$cols = 4; $scale = 0.5; $gap = 16
$w = [int]($imgs[0].Width * $scale); $h = [int]($imgs[0].Height * $scale)
$rows = [math]::Ceiling($imgs.Count / $cols)
$sheet = New-Object System.Drawing.Bitmap (($w + $gap) * $cols + $gap), (($h + $gap) * $rows + $gap)
$g = [System.Drawing.Graphics]::FromImage($sheet)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.Clear([System.Drawing.Color]::FromArgb(32, 32, 36))
for ($i = 0; $i -lt $imgs.Count; $i++) {
  $x = $gap + ($i % $cols) * ($w + $gap); $y = $gap + [math]::Floor($i / $cols) * ($h + $gap)
  $g.DrawImage($imgs[$i], $x, $y, $w, $h)
}
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq "image/jpeg" }
$params = New-Object System.Drawing.Imaging.EncoderParameters 1
$params.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 82L
$sheet.Save($Out, $codec, $params)
$g.Dispose(); $sheet.Dispose(); $imgs | ForEach-Object { $_.Dispose() }
