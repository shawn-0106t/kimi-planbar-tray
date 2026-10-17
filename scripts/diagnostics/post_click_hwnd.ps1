# Post WM_LBUTTONDOWN/UP to an arbitrary HWND of the app (bypasses OS hit-testing).
# Usage: -Hwnd <int> -Cx <client-x> -Cy <client-y> (physical client pixels)
param([long]$Hwnd, [int]$Cx, [int]$Cy)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KP2 {
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KP2]::SetProcessDPIAware() | Out-Null
$h = [IntPtr]$Hwnd
Write-Output ("posting click to hwnd=0x{0:X} at client ({1},{2})" -f $Hwnd, $Cx, $Cy)
$lp = [IntPtr](($Cy -band 0xFFFF) * 65536 + ($Cx -band 0xFFFF))
[KP2]::PostMessageW($h, 0x0200, [IntPtr]::Zero, $lp) | Out-Null   # WM_MOUSEMOVE
Start-Sleep -Milliseconds 100
[KP2]::PostMessageW($h, 0x0201, [IntPtr]1, $lp) | Out-Null        # WM_LBUTTONDOWN
Start-Sleep -Milliseconds 80
[KP2]::PostMessageW($h, 0x0202, [IntPtr]::Zero, $lp) | Out-Null   # WM_LBUTTONUP
Start-Sleep -Milliseconds 1000
Write-Output 'posted'
