# Click the tray icon (with retries) until the app's window with HWND -Hwnd
# reports IsWindowVisible=true. Ground truth is IsWindowVisible, not UIA rects
# (UIA BoundingRectangle stays stale for hidden singleton windows).
# Usage: ensure_window.ps1 -Hwnd 0x150208 [-Attempts 5] [-Toggle] [-TrayX 1987] [-TrayY 1746]
# Tray coordinates drift as icons come and go - re-probe with find_tray_icon.ps1.
param([long]$Hwnd, [int]$Attempts = 5, [switch]$Toggle, [int]$TrayX = 2050, [int]$TrayY = 1746)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KEW {
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KEW]::SetProcessDPIAware() | Out-Null
$h = [IntPtr]$Hwnd
for ($a = 1; $a -le $Attempts; $a++) {
    if ([KEW]::IsWindowVisible($h)) { Write-Output ("VISIBLE attempt {0}" -f $a); exit 0 }
    if (-not $Toggle) { Write-Output ("HIDDEN attempt {0} (no toggle requested)" -f $a); exit 1 }
    [KEW]::SetCursorPos($TrayX, $TrayY) | Out-Null
    Start-Sleep -Milliseconds 150
    [KEW]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [KEW]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 1800
    if ([KEW]::IsWindowVisible($h)) { Write-Output ("VISIBLE attempt {0} (after click)" -f $a); exit 0 }
    Write-Output ("attempt {0}: still hidden after tray click" -f $a)
}
Write-Output 'FAIL'
exit 1
