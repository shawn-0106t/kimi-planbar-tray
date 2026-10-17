# Check whether the panel actually gains foreground after a tray click.
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KF {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out int pid);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KF]::SetProcessDPIAware() | Out-Null
function Get-WinHwnd($namePart) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    for ($i = 0; $i -lt $all.Count; $i++) {
        $el = $all.Item($i)
        if ($el.Current.ProcessId -eq $AppPid -and $el.Current.Name -match $namePart) { return $el.Current.NativeWindowHandle }
    }
    return 0
}
function Click($x, $y) {
    [KF]::SetCursorPos([int]$x, [int]$y) | Out-Null
    Start-Sleep -Milliseconds 120
    [KF]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 70
    [KF]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}

# hide panel if open
if (Get-WinHwnd 'Main Panel') { Click 600 1400; Start-Sleep -Milliseconds 1200 }
Click 2050 1746
Start-Sleep -Milliseconds 800
$fg1 = [KF]::GetForegroundWindow()
$pid1 = 0; [void][KF]::GetWindowThreadProcessId($fg1, [ref]$pid1)
$panel1 = Get-WinHwnd 'Main Panel'
Write-Output ("t+800ms: panel hwnd={0} foreground={1} fg_pid={2} fg_is_panel={3}" -f $panel1, $fg1.ToInt64(), $pid1, ($fg1.ToInt64() -eq $panel1))
Start-Sleep -Milliseconds 1500
$fg2 = [KF]::GetForegroundWindow()
$pid2 = 0; [void][KF]::GetWindowThreadProcessId($fg2, [ref]$pid2)
Write-Output ("t+2300ms: foreground={0} fg_pid={1} fg_is_panel={2}" -f $fg2.ToInt64(), $pid2, ($fg2.ToInt64() -eq $panel1))
