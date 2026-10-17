# Post WM_LBUTTONDOWN/UP directly to the panel HWND (bypasses OS hit-testing).
# Requires the panel to be visible. Button client coords given in PHYSICAL pixels.
param([int]$AppPid, [int]$Cx, [int]$Cy)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KP {
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KP]::SetProcessDPIAware() | Out-Null
$root = [System.Windows.Automation.AutomationElement]::RootElement
$all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
$hwnd = [IntPtr]::Zero
for ($i = 0; $i -lt $all.Count; $i++) {
    $el = $all.Item($i)
    if ($el.Current.ProcessId -eq $AppPid -and $el.Current.Name -eq 'Kimi Planbar Tray Main Panel') {
        $hwnd = [IntPtr]$el.Current.NativeWindowHandle
    }
}
if ($hwnd -eq [IntPtr]::Zero) { Write-Output 'FAIL: panel not visible'; exit 1 }
Write-Output ("panel hwnd=0x{0:X}, posting click at client ({1},{2})" -f $hwnd.ToInt64(), $Cx, $Cy)
$lp = [IntPtr](($Cy -band 0xFFFF) * 65536 + ($Cx -band 0xFFFF))
[KP]::PostMessageW($hwnd, 0x0200, [IntPtr]::Zero, $lp) | Out-Null   # WM_MOUSEMOVE
Start-Sleep -Milliseconds 100
[KP]::PostMessageW($hwnd, 0x0201, [IntPtr]1, $lp) | Out-Null        # WM_LBUTTONDOWN
Start-Sleep -Milliseconds 80
[KP]::PostMessageW($hwnd, 0x0202, [IntPtr]::Zero, $lp) | Out-Null   # WM_LBUTTONUP
Start-Sleep -Milliseconds 1200
$all2 = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
$settingsOpen = $false
for ($i = 0; $i -lt $all2.Count; $i++) {
    $el = $all2.Item($i)
    if ($el.Current.ProcessId -eq $AppPid -and $el.Current.Name -match 'Settings') { $settingsOpen = $true }
}
if ($settingsOpen) { Write-Output 'RESULT: settings opened via PostMessage (app-side input path OK)' } else { Write-Output 'RESULT: no response to PostMessage (app-side input DEAD)' }
