# Timed reproduction of the input-death sequence with wall-clock markers.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KT {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KT]::SetProcessDPIAware() | Out-Null
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
    [KT]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 150
    [KT]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [KT]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}
function Wiggle($x, $y) {
    [KT]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 120
    [KT]::SetCursorPos([int]($x + 30), [int]$y) | Out-Null
    Start-Sleep -Milliseconds 120
    [KT]::SetCursorPos([int]$x, [int]$y) | Out-Null
}
function Mark($msg) { Write-Output ("{0:HH:mm:ss.fff} {1}" -f (Get-Date), $msg) }

Mark 'T1: tray click -> panel'
Click 2050 1746
Start-Sleep -Milliseconds 1600
$pr = Get-Win 'Main Panel'
if (-not $pr) { Mark 'FAIL: panel did not open'; exit 1 }
Mark ("panel at ({0},{1},{2},{3})" -f $pr.X, $pr.Y, $pr.Width, $pr.Height)

Mark 'T2: wiggle over panel + click Settings'
Wiggle ($pr.X + 700) ($pr.Y + 500)
Start-Sleep -Milliseconds 400
Click ([int]($pr.X + $pr.Width * 0.603)) ([int]($pr.Y + $pr.Height * 0.868))
Start-Sleep -Milliseconds 1400
$sr = Get-Win 'Settings'
Mark ("settings open: {0}" -f ($null -ne $sr))
if (-not $sr) { Mark 'FAIL: settings did not open at T2'; exit 1 }

Mark 'T3: close settings via X'
Click ([int]($sr.X + $sr.Width - 134)) ([int]($sr.Y + 122))
Start-Sleep -Milliseconds 1600
Mark ("settings visible: {0}, panel visible: {1}" -f ($null -ne (Get-Win 'Settings')), ($null -ne (Get-Win 'Main Panel')))

Mark 'T4: click desktop (force panel focus loss)'
Click 600 1400
Start-Sleep -Milliseconds 1600
Mark ("panel visible: {0}" -f ($null -ne (Get-Win 'Main Panel')))

Mark 'T5: tray click -> panel again'
Click 2050 1746
Start-Sleep -Milliseconds 1600
$pr5 = Get-Win 'Main Panel'
Mark ("panel visible: {0}" -f ($null -ne $pr5))
if (-not $pr5) { Mark 'FAIL: panel did not reopen at T5'; exit 1 }

Mark 'T6: wiggle + click Settings again'
Wiggle ($pr5.X + 700) ($pr5.Y + 500)
Start-Sleep -Milliseconds 400
Click ([int]($pr5.X + $pr5.Width * 0.603)) ([int]($pr5.Y + $pr5.Height * 0.868))
Start-Sleep -Milliseconds 1400
$sr6 = Get-Win 'Settings'
if ($sr6) { Mark 'RESULT: settings opened at T6 (input PASS)' } else { Mark 'RESULT: settings did NOT open at T6 (input FAIL)' }
