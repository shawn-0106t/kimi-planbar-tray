# Minimal auto-hide check: show panel via tray, click desktop, panel must hide.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KH2 {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KH2]::SetProcessDPIAware() | Out-Null
function Get-Win($namePart) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    for ($i = 0; $i -lt $all.Count; $i++) {
        $el = $all.Item($i)
        if ($el.Current.ProcessId -eq $AppPid -and $el.Current.Name -match $namePart) { return $el.Current.BoundingRectangle }
    }
    return $null
}
function Click($x, $y) {
    [KH2]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 120
    [KH2]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 70
    [KH2]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}

# ensure panel hidden first
if (Get-Win 'Main Panel') { Click 600 1400; Start-Sleep -Milliseconds 1200 }
Click 2050 1746
Start-Sleep -Milliseconds 1500
$shown = $null -ne (Get-Win 'Main Panel')
Write-Output ("panel shown: {0}" -f $shown)
Click 600 1400
Start-Sleep -Milliseconds 2000
$stillThere = $null -ne (Get-Win 'Main Panel')
Write-Output ("panel visible after desktop click: {0}  (expected False)" -f $stillThere)
