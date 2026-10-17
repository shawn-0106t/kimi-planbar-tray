# Click tray icon center, then report app window visibility via UIA.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KR2 {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KR2]::SetProcessDPIAware() | Out-Null
[KR2]::SetCursorPos(2050, 1746) | Out-Null
Start-Sleep -Milliseconds 150
[KR2]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[KR2]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 1500
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
