# One-off: run wpf --test-ui, grab the real screen (DWM-composited) while the
# four windows are visible, and dump their exact rects for cropping.
param([string]$OutPng = "$env:TEMP\kpt-wpf-testui.png",
      [string]$OutTxt = "$env:TEMP\kpt-wpf-testui-rects.txt")
$exe = Join-Path $PSScriptRoot "..\..\wpf\bin\Release\net8.0-windows\KimiPlanbarTray.exe"
$p = Start-Process -FilePath $exe -ArgumentList "--test-ui" -PassThru
Start-Sleep -Milliseconds 3800  # windows are up ~6s; grab mid-window
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Native -Name Dpi -MemberDefinition '
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lp);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
[DllImport("user32.dll")] public static extern int GetWindowTextW(IntPtr hWnd, [MarshalAs(UnmanagedType.LPWStr)] System.Text.StringBuilder sb, int max);
public struct RECT { public int L; public int T; public int R; public int B; }'
[Native.Dpi]::SetProcessDPIAware() | Out-Null
$vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap($vs.Width, $vs.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($vs.X, $vs.Y, 0, 0, $bmp.Size)
$g.Dispose()
$bmp.Save($OutPng, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
# dump windows of the test process
$sb = New-Object System.Text.StringBuilder 256
$lines = @()
$cb = {
    param($h, $lp)
    $pid2 = 0
    [Native.Dpi]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
    if ($pid2 -eq $p.Id) {
        [Native.Dpi]::GetWindowTextW($h, $sb, 256) | Out-Null
        $r = New-Object Native.Dpi+RECT
        [Native.Dpi]::GetWindowRect($h, [ref]$r) | Out-Null
        if ($r.R - $r.L -gt 0) {
            $script:lines += "hwnd=$h title='$($sb)' rect=($($r.L),$($r.T))-($($r.R),$($r.B))"
        }
    }
    return $true
}
[Native.Dpi]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
$lines += "virtualscreen=($($vs.X),$($vs.Y)) $($vs.Width)x$($vs.Height)"
$lines | Set-Content -Path $OutTxt -Encoding UTF8
Write-Output "saved: $OutPng + $OutTxt"
Wait-Process -Id $p.Id -ErrorAction SilentlyContinue
Write-Output "test-ui exited"
