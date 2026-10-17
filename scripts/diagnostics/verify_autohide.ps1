# Verify focus-loss auto-hide still works with WM_NCACTIVATE swallowed:
# settings closed -> click desktop -> panel must auto-hide within ~2s.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KH {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KH]::SetProcessDPIAware() | Out-Null

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
    [KH]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 120
    [KH]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 70
    [KH]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}

$sr = Get-Win 'Settings'
if ($sr) {
    Write-Output 'closing settings...'
    Click ($sr.X + $sr.Width - 45) ($sr.Y + 122)
    Start-Sleep -Milliseconds 900
}
$pr = Get-Win 'Main Panel'
Write-Output ("panel visible before desktop click: {0}" -f ($null -ne $pr))
if ($pr) {
    Click 600 1400
    Start-Sleep -Milliseconds 2000
    $pr2 = Get-Win 'Main Panel'
    Write-Output ("panel visible after desktop click: {0}  (expected False = auto-hide works)" -f ($null -ne $pr2))
}
