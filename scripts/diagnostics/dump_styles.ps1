# Read GWL_STYLE/GWL_EXSTYLE of every window belonging to the app (via UIA native handles).
param([int]$AppPid)
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KS {
    [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int idx);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
}
'@
$root = [System.Windows.Automation.AutomationElement]::RootElement
$all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
for ($i = 0; $i -lt $all.Count; $i++) {
    $el = $all.Item($i)
    if ($el.Current.ProcessId -eq $AppPid) {
        $h = [IntPtr]$el.Current.NativeWindowHandle
        $style = [KS]::GetWindowLongPtrW($h, -16)
        $exstyle = [KS]::GetWindowLongPtrW($h, -20)
        $en = [KS]::IsWindowEnabled($h)
        Write-Output ("'{0}' hwnd=0x{1:X} style=0x{2:X8} exstyle=0x{3:X8} enabled={4}" -f $el.Current.Name, $h.ToInt64(), $style, $exstyle, $en)
    }
}
Write-Output 'done'
