[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$AssetDir = Join-Path $RepoRoot 'assets'
$PngPath = Join-Path $AssetDir 'SubscriptionLens.png'
$IcoPath = Join-Path $AssetDir 'SubscriptionLens.ico'
$size = 256

$visual = [System.Windows.Media.DrawingVisual]::new()
$context = $visual.RenderOpen()
try {
    $context.DrawRoundedRectangle(
        [System.Windows.Media.Brushes]::Black,
        $null,
        [System.Windows.Rect]::new(10, 10, 236, 236),
        56,
        56)
    $lensPen = [System.Windows.Media.Pen]::new(
        [System.Windows.Media.SolidColorBrush]::new([System.Windows.Media.Color]::FromRgb(255, 250, 240)),
        17)
    $context.DrawEllipse($null, $lensPen, [System.Windows.Point]::new(112, 108), 54, 54)
    $handlePen = [System.Windows.Media.Pen]::new(
        [System.Windows.Media.SolidColorBrush]::new([System.Windows.Media.Color]::FromRgb(201, 100, 66)),
        19)
    $handlePen.StartLineCap = [System.Windows.Media.PenLineCap]::Round
    $handlePen.EndLineCap = [System.Windows.Media.PenLineCap]::Round
    $context.DrawLine($handlePen, [System.Windows.Point]::new(151, 148), [System.Windows.Point]::new(199, 196))
    $context.DrawEllipse(
        [System.Windows.Media.SolidColorBrush]::new([System.Windows.Media.Color]::FromRgb(217, 189, 117)),
        $null,
        [System.Windows.Point]::new(91, 88),
        9,
        9)
}
finally {
    $context.Close()
}

$bitmap = [System.Windows.Media.Imaging.RenderTargetBitmap]::new($size, $size, 96, 96, [System.Windows.Media.PixelFormats]::Pbgra32)
$bitmap.Render($visual)
$encoder = [System.Windows.Media.Imaging.PngBitmapEncoder]::new()
$encoder.Frames.Add([System.Windows.Media.Imaging.BitmapFrame]::Create($bitmap))
$pngStream = [System.IO.MemoryStream]::new()
try {
    $encoder.Save($pngStream)
    $pngBytes = $pngStream.ToArray()
    [System.IO.File]::WriteAllBytes($PngPath, $pngBytes)

    $iconStream = [System.IO.MemoryStream]::new()
    $writer = [System.IO.BinaryWriter]::new($iconStream)
    try {
        $writer.Write([uint16]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]1)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]32)
        $writer.Write([uint32]$pngBytes.Length)
        $writer.Write([uint32]22)
        $writer.Write($pngBytes)
        $writer.Flush()
        [System.IO.File]::WriteAllBytes($IcoPath, $iconStream.ToArray())
    }
    finally {
        $writer.Dispose()
        $iconStream.Dispose()
    }
}
finally {
    $pngStream.Dispose()
}

Get-Item -LiteralPath $PngPath, $IcoPath | Select-Object FullName, Length
