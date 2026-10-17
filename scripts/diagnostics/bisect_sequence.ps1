# Bisect sequence: tray->panel, click Settings, close settings (X), wait auto-hide,
# tray->panel, click Settings again. Reports window visibility at each step.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KB {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KB]::SetProcessDPIAware() | Out-Null

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
    [KB]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 150
    [KB]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [KB]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}

Write-Output '--- step 1: tray click -> panel'
Click 2050 1746
Start-Sleep -Milliseconds 1500
$pr = Get-Win 'Main Panel'
if (-not $pr) { Write-Output 'FAIL: panel did not open'; exit 1 }
Write-Output ("panel at ({0},{1},{2},{3})" -f $pr.X, $pr.Y, $pr.Width, $pr.Height)
$bx = [int]($pr.X + $pr.Width * 0.603)
$by = [int]($pr.Y + $pr.Height * 0.868)

Write-Output '--- step 2: click Settings'
Click $bx $by
Start-Sleep -Milliseconds 1300
$sr = Get-Win 'Settings'
Write-Output ("settings open: {0}" -f ($null -ne $sr))
if (-not $sr) { Write-Output 'FAIL: settings did not open (step 2)'; exit 1 }

Write-Output '--- step 3: close settings via X'
# X measured: right - 134, top + 122 (physical, from burst_out8 capture)
Click ([int]($sr.X + $sr.Width - 134)) ([int]($sr.Y + 122))
Start-Sleep -Milliseconds 1500
$sr2 = Get-Win 'Settings'
$pr2 = Get-Win 'Main Panel'
Write-Output ("settings visible: {0}, panel visible: {1}" -f ($null -ne $sr2), ($null -ne $pr2))

Write-Output '--- step 4: tray click -> panel again'
Click 2050 1746
Start-Sleep -Milliseconds 1500
$pr3 = Get-Win 'Main Panel'
Write-Output ("panel visible: {0}" -f ($null -ne $pr3))
if (-not $pr3) { Write-Output 'FAIL: panel did not reopen (step 4)'; exit 1 }
$bx3 = [int]($pr3.X + $pr3.Width * 0.603)
$by3 = [int]($pr3.Y + $pr3.Height * 0.868)

Write-Output '--- step 5: click Settings again'
Click $bx3 $by3
Start-Sleep -Milliseconds 1300
$sr3 = Get-Win 'Settings'
Write-Output ("settings open: {0}" -f ($null -ne $sr3))
if ($sr3) { Write-Output 'RESULT: input alive after reopen (PASS)' } else { Write-Output 'RESULT: input DEAD after reopen (FAIL)' }
