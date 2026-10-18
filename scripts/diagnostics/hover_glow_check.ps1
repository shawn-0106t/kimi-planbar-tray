# One-off: run wpf --test-ui, hover the Refresh action button, grab the panel
# region mid-hover to verify the SPEC 15.3 glow renders (accent ring + halo).
param([string]$OutPng = "$env:TEMP\kpt-wpf-hover.png")
$exe = Join-Path $PSScriptRoot "..\..\wpf\bin\Release\net8.0-windows\KimiPlanbarTray.exe"
$p = Start-Process -FilePath $exe -ArgumentList "--test-ui" -PassThru
Start-Sleep -Milliseconds 3800
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Native -Name Dpi -MemberDefinition '
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);'
[Native.Dpi]::SetProcessDPIAware() | Out-Null
# panel window rect from the previous run: (1949,545)-(2903,1715); the action
# row sits at the bottom — Refresh button center ~ x=L+358, y=B-145
[Native.Dpi]::SetCursorPos(2307, 1570) | Out-Null
Start-Sleep -Milliseconds 500   # 140ms animation + margin
$bmp = New-Object System.Drawing.Bitmap(954, 1170)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen(1949, 545, 0, 0, $bmp.Size)
$g.Dispose()
$bmp.Save($OutPng, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
[Native.Dpi]::SetCursorPos(100, 100) | Out-Null
Write-Output "saved: $OutPng"
Wait-Process -Id $p.Id -ErrorAction SilentlyContinue
