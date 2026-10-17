# Poll UIA until one of the app's top-level windows matching -NamePart is
# visible (BoundingRectangle non-empty), or -TimeoutS elapses.
# Usage: wait_window.ps1 -AppPid <pid> -NamePart 'Main Panel' [-TimeoutS 10]
param([int]$AppPid, [string]$NamePart, [int]$TimeoutS = 10)
Add-Type -AssemblyName UIAutomationClient
$sw = [System.Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $TimeoutS) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    for ($i = 0; $i -lt $all.Count; $i++) {
        $el = $all.Item($i)
        if ($el.Current.ProcessId -eq $AppPid -and $el.Current.Name -match $NamePart) {
            $r = $el.Current.BoundingRectangle
            if ($r.Width -gt 0) {
                Write-Output ("VISIBLE after {0:0.0}s: '{1}' rect=({2},{3},{4},{5})" -f $sw.Elapsed.TotalSeconds, $el.Current.Name, [int]$r.X, [int]$r.Y, [int]$r.Width, [int]$r.Height)
                exit 0
            }
        }
    }
    Start-Sleep -Milliseconds 250
}
Write-Output ("TIMEOUT after {0}s waiting for '{1}'" -f $TimeoutS, $NamePart)
exit 1
