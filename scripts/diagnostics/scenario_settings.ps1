# Clean, state-verified scenario runner:
#   close settings if open -> hide panel if open -> show panel -> burst capture Settings click.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient, System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KR {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KR]::SetProcessDPIAware() | Out-Null

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
    [KR]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 120
    [KR]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 70
    [KR]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}

# 1. close settings if open (X at top-right corner of the settings window)
$sr = Get-Win 'Settings'
if ($sr) {
    Write-Output ("settings open at ({0},{1},{2},{3}) -> closing" -f $sr.X, $sr.Y, $sr.Width, $sr.Height)
    Click ($sr.X + $sr.Width - 45) ($sr.Y + 122)
    Start-Sleep -Milliseconds 900
    if (Get-Win 'Settings') { Write-Output 'WARN: settings still open after close click' }
}
# 2. hide panel if visible (click desktop)
if (Get-Win 'Main Panel') {
    Write-Output 'panel open -> hiding via desktop click'
    Click 600 1400
    Start-Sleep -Milliseconds 1200
}
# 3. show panel
Click 2050 1746
Start-Sleep -Milliseconds 1400
$pr = Get-Win 'Main Panel'
if (-not $pr) { Write-Output 'FATAL: panel did not open'; exit 1 }
Write-Output ("panel at ({0},{1},{2},{3})" -f $pr.X, $pr.Y, $pr.Width, $pr.Height)
# Settings button: measured offsets from panel top-left (575, 1016) on a 954x1170 window.
$bx = [int]($pr.X + 954 * 0.603)
$by = [int]($pr.Y + 1170 * 0.868)
Write-Output ("settings button at ($bx,$by)")

# 4. burst capture with the click
& "$PSScriptRoot\burst_capture.ps1" -Left 900 -Top 300 -Width 1980 -Height 1392 `
    -DurationMs 2200 -OutDir "$PSScriptRoot\burst_out7" -ClickX $bx -ClickY $by -ClickAtMs 500 -Prefix stg

# 5. final state
Start-Sleep -Milliseconds 400
$sr2 = Get-Win 'Settings'
$pr2 = Get-Win 'Main Panel'
Write-Output ("after: settings visible={0} panel visible={1}" -f ($null -ne $sr2), ($null -ne $pr2))
