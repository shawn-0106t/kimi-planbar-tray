# Analyze burst frames: per-frame bright-pixel stats to spot white flashes.
param([string]$Dir, [int]$Threshold = 200)
Add-Type -AssemblyName System.Drawing
$files = Get-ChildItem $Dir -Filter *.png | Sort-Object Name
foreach ($f in $files) {
    $bmp = New-Object System.Drawing.Bitmap($f.FullName)
    $w = $bmp.Width; $h = $bmp.Height
    $rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $stride = $data.Stride
    $bytes = New-Object byte[] ($stride * $h)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
    $bmp.UnlockBits($data)
    $bright = 0; $total = 0; $sum = 0L
    for ($y = 0; $y -lt $h; $y += 2) {
        $row = $y * $stride
        for ($x = 0; $x -lt $w; $x += 2) {
            $o = $row + $x * 3
            $lum = ($bytes[$o] + $bytes[$o+1] + $bytes[$o+2]) / 3
            $sum += $lum
            if ($lum -ge $Threshold) { $bright++ }
            $total++
        }
    }
    $bmp.Dispose()
    Write-Output ("{0}  mean={1,5:n1}  bright%={2,6:n2}" -f $f.Name, ($sum / $total), (100.0 * $bright / $total))
}
