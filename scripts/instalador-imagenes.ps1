param([string]$Out = (Join-Path $PSScriptRoot "..ppsdesktopsrc-tauriinstaller"), [int]$Scale = 1)
# Regenera las imágenes del instalador (NSIS) en apps/desktop/src-tauri/installer/:
#   powershell -ExecutionPolicy Bypass -File scripts/instalador-imagenes.ps1
# Imágenes del instalador (NSIS): lateral 164 x 314 y cabecera 150 x 57, BMP de 24 bits.
# Estilo Vidrio: el fondo oscuro del icono, el tamiz con su última barra en el acento.
Add-Type -AssemblyName System.Drawing

function New-Canvas([int]$w, [int]$h, [string]$bg) {
  $bmp = New-Object Drawing.Bitmap ($w * $Scale), ($h * $Scale), ([Drawing.Imaging.PixelFormat]::Format24bppRgb)
  $g = [Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = 'AntiAlias'
  $g.TextRenderingHint = 'AntiAliasGridFit'
  $g.ScaleTransform($Scale, $Scale)
  $g.Clear([Drawing.ColorTranslator]::FromHtml($bg))
  return @($bmp, $g)
}

function Pill($g, [float]$x, [float]$y, [float]$w, [float]$h, [string]$color, [int]$alpha = 255) {
  $c = [Drawing.ColorTranslator]::FromHtml($color)
  $brush = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb($alpha, $c))
  $path = New-Object Drawing.Drawing2D.GraphicsPath
  $r = $h
  $path.AddArc($x, $y, $r, $h, 90, 180)
  $path.AddArc($x + $w - $r, $y, $r, $h, 270, 180)
  $path.CloseFigure()
  $g.FillPath($brush, $path)
}

# El tamiz centrado en (cx, top), con el ancho de la barra de arriba = $w (proporciones del icono).
function Sieve($g, [float]$cx, [float]$top, [float]$w, [string]$fg, [string]$accent) {
  $k = $w / 152
  $h = 26 * $k
  Pill $g ($cx - 76 * $k) $top (152 * $k) $h $fg
  Pill $g ($cx - 56 * $k) ($top + 43 * $k) (112 * $k) $h $fg 191
  Pill $g ($cx - 32 * $k) ($top + 86 * $k) (64 * $k) $h $accent
}

$font = 'Segoe UI'
$dark = '#202024'
$light = '#F3F4F6'
$accent = '#60CDFF'

# --- Lateral (bienvenida y final) ---
$bmp, $g = New-Canvas 164 314 $dark
Sieve $g 82 92 76 $light $accent
$title = New-Object Drawing.Font $font, 20, ([Drawing.FontStyle]::Bold), ([Drawing.GraphicsUnit]::Pixel)
$small = New-Object Drawing.Font $font, 11, ([Drawing.FontStyle]::Regular), ([Drawing.GraphicsUnit]::Pixel)
$fmt = New-Object Drawing.StringFormat
$fmt.Alignment = 'Center'
$g.DrawString('Sift', $title, (New-Object Drawing.SolidBrush ([Drawing.ColorTranslator]::FromHtml($light))), (New-Object Drawing.RectangleF 0, 150, 164, 30), $fmt)
$muted = New-Object Drawing.SolidBrush ([Drawing.ColorTranslator]::FromHtml('#A1A1AA'))
$g.DrawString("Busca y abre lo tuyo", $small, $muted, (New-Object Drawing.RectangleF 0, 182, 164, 18), $fmt)
$g.DrawString("Todo se queda en tu PC", $small, $muted, (New-Object Drawing.RectangleF 0, 282, 164, 18), $fmt)
$bmp.Save((Join-Path $Out "sidebar.bmp"), [Drawing.Imaging.ImageFormat]::Bmp)
$g.Dispose(); $bmp.Dispose()

# --- Cabecera (páginas interiores; va a la derecha, sobre blanco) ---
$bmp, $g = New-Canvas 150 57 '#FFFFFF'
Sieve $g 118 14 40 $dark '#0067C0'
$bmp.Save((Join-Path $Out "header.bmp"), [Drawing.Imaging.ImageFormat]::Bmp)
$g.Dispose(); $bmp.Dispose()
"ok"
