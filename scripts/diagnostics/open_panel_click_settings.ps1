# Open panel via tray, click Settings, report.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KC {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KC]::SetProcessDPIAware() | Out-Null
function Get-Win($namePart) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    for ($i = 0; $i -lt $all.Count; $i++) {
        $el = $all.Item($i)
        if ($el.Current.ProcessId -eq $AppPid -and $el.Current.Name -match $namePart) {
            return $el.Current.BoundingRectangle
        }
    }
    return $null
}
function Click($x, $y) {
    [KC]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 150
    [KC]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [KC]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}
$p = Get-Win 'Main Panel'
if (-not $p) {
    Click 2050 1746
    Start-Sleep -Milliseconds 1500
    $p = Get-Win 'Main Panel'
}
if (-not $p) { Write-Output 'FAIL: panel did not open'; exit 1 }
Write-Output ("panel at ({0},{1},{2},{3})" -f $p.X, $p.Y, $p.Width, $p.Height)
$bx = [int]($p.X + $p.Width * 0.603)
$by = [int]($p.Y + $p.Height * 0.868)
Click $bx $by
Start-Sleep -Milliseconds 1300
$s = Get-Win 'Settings'
if ($s) { Write-Output 'RESULT: settings opened (input PASS)' } else { Write-Output 'RESULT: settings did NOT open (input FAIL)' }
