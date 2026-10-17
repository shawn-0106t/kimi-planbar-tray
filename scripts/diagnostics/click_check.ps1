# Click arbitrary point then report app windows via UIA.
param([int]$AppPid, [int]$X, [int]$Y)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KR3 {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KR3]::SetProcessDPIAware() | Out-Null
[KR3]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 150
[KR3]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[KR3]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 1200
$root = [System.Windows.Automation.AutomationElement]::RootElement
$all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
for ($i = 0; $i -lt $all.Count; $i++) {
    $el = $all.Item($i)
    if ($el.Current.ProcessId -eq $AppPid) {
        $r = $el.Current.BoundingRectangle
        Write-Output ("VISIBLE: '{0}' rect=({1},{2},{3},{4})" -f $el.Current.Name, $r.X, $r.Y, $r.Width, $r.Height)
    }
}
Write-Output 'done'
