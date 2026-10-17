# Burst-capture a screen region around a synthetic mouse click.
# Usage: burst_capture.ps1 -Left X -Top Y -Width W -Height H -DurationMs N -OutDir DIR
#        [-ClickX x -ClickY y -ClickAtMs t] [-Button right] [-Prefix name]
param(
    [int]$Left, [int]$Top, [int]$Width, [int]$Height,
    [int]$DurationMs = 1500,
    [string]$OutDir,
    [int]$ClickX = -1, [int]$ClickY = -1, [int]$ClickAtMs = 300,
    [string]$Button = 'left',
    [string]$Prefix = 'f'
)
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KptMouse {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    public const uint LEFTDOWN = 0x02, LEFTUP = 0x04, RIGHTDOWN = 0x08, RIGHTUP = 0x10;
}
'@
[KptMouse]::SetProcessDPIAware() | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$frames = New-Object System.Collections.ArrayList
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$clicked = $false
$bmp = New-Object System.Drawing.Bitmap($Width, $Height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)

while ($sw.ElapsedMilliseconds -lt $DurationMs) {
    if (-not $clicked -and $ClickX -ge 0 -and $sw.ElapsedMilliseconds -ge $ClickAtMs) {
        [KptMouse]::SetCursorPos($ClickX, $ClickY) | Out-Null
        Start-Sleep -Milliseconds 100
        if ($Button -eq 'right') {
            [KptMouse]::mouse_event([KptMouse]::RIGHTDOWN, 0, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 60
            [KptMouse]::mouse_event([KptMouse]::RIGHTUP, 0, 0, 0, [UIntPtr]::Zero)
        } else {
            [KptMouse]::mouse_event([KptMouse]::LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 60
            [KptMouse]::mouse_event([KptMouse]::LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
        }
        $clicked = $true
    }
    $t = $sw.ElapsedMilliseconds
    $g.CopyFromScreen($Left, $Top, 0, 0, (New-Object System.Drawing.Size($Width, $Height)))
    $copy = $bmp.Clone()
    [void]$frames.Add(@($t, $copy))
}
$g.Dispose(); $bmp.Dispose()

$i = 0
foreach ($f in $frames) {
    $path = Join-Path $OutDir ("{0}_{1:d4}_{2}ms.png" -f $Prefix, $i, $f[0])
    $f[1].Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $f[1].Dispose()
    $i++
}
Write-Output ("captured {0} frames over {1} ms -> {2}" -f $frames.Count, $sw.ElapsedMilliseconds, $OutDir)
