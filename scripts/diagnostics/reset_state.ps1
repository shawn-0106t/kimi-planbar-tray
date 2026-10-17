# Reset to a clean state: close settings/skills/menu, hide panel.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KRZ {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KRZ]::SetProcessDPIAware() | Out-Null
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
    [KRZ]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 150
    [KRZ]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [KRZ]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}
$sr = Get-Win 'Settings'
if ($sr) { Click ([int]($sr.X + $sr.Width - 134)) ([int]($sr.Y + 122)); Start-Sleep -Milliseconds 1000 }
$sk = Get-Win 'Skills'
if ($sk) { Click ([int]($sk.X + $sk.Width - 134)) ([int]($sk.Y + 122)); Start-Sleep -Milliseconds 1000 }
if (Get-Win 'Main Panel') { Click 600 1400; Start-Sleep -Milliseconds 1300 }
if (Get-Win 'Menu') { Click 600 1400; Start-Sleep -Milliseconds 1000 }
Write-Output ("clean: settings={0} skills={1} panel={2} menu={3}" -f ($null -ne (Get-Win 'Settings')), ($null -ne (Get-Win 'Skills')), ($null -ne (Get-Win 'Main Panel')), ($null -ne (Get-Win 'Menu')))
