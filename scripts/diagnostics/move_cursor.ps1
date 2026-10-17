# Move the physical cursor over a screen rect and report; pairs with reading
# the app trace (root panel pointer field) to detect cross-window mouse routing.
param([int]$X, [int]$Y)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class MV {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[MV]::SetProcessDPIAware() | Out-Null
Write-Output ("moving cursor to ({0},{1}) and wiggling" -f $X, $Y)
[MV]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 400
[MV]::SetCursorPos($X + 20, $Y + 10) | Out-Null
Start-Sleep -Milliseconds 400
[MV]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 600
Write-Output 'done'
