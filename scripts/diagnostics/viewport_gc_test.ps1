# Test: does a child-viewport pass (settings window hover) destroy the root window?
# 1. Show panel via tray click, record hwnd.
# 2. Click Settings button, record settings hwnd.
# 3. Hover over the settings window to force child-only passes (hover glow).
# 4. Re-check both hwnds with IsWindow.
Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KMT {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KMT]::SetProcessDPIAware() | Out-Null

function Get-KimiWindows {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    $res = @{}
    for ($i = 0; $i -lt $all.Count; $i++) {
        $el = $all.Item($i)
        if ($el.Current.ProcessId -eq 17128 -and $el.Current.Name) {
            $res[$el.Current.Name] = $el.Current.NativeWindowHandle
        }
    }
    return $res
}

function Click($x, $y) {
    [KMT]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 40
    [KMT]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
    [KMT]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
}

# 1. show panel
Click 2050 1746
Start-Sleep -Milliseconds 1200
$w1 = Get-KimiWindows
Write-Output 'after panel show:'
$w1.GetEnumerator() | ForEach-Object { Write-Output ("  '{0}' hwnd={1}" -f $_.Key, $_.Value) }
$panelHwnd = [IntPtr]$w1['Kimi Planbar Tray Main Panel']
Write-Output ("panel hwnd = {0}" -f $panelHwnd.ToInt64())

# 2. click Settings button (panel physical rect (1949,545)-(2903,1715); settings btn ~(2518,1565))
Click 2518 1565
Start-Sleep -Milliseconds 1200
$w2 = Get-KimiWindows
Write-Output 'after settings click:'
$w2.GetEnumerator() | ForEach-Object { Write-Output ("  '{0}' hwnd={1}" -f $_.Key, $_.Value) }
$stgHwnd = [IntPtr]$w2['Kimi Planbar Tray Settings']
Write-Output ("panel IsWindow = {0}" -f [KMT]::IsWindow($panelHwnd))

# 3. hover over settings window to force child-only passes (hover glow animation)
for ($k = 0; $k -lt 20; $k++) {
    [KMT]::SetCursorPos((1300 + $k * 10), (700 + ($k % 3) * 40)) | Out-Null
    Start-Sleep -Milliseconds 60
}
Start-Sleep -Milliseconds 800
Write-Output ("after settings hover: panel IsWindow = {0}, settings IsWindow = {1}" -f [KMT]::IsWindow($panelHwnd), [KMT]::IsWindow($stgHwnd))
$w3 = Get-KimiWindows
Write-Output 'final visible windows:'
$w3.GetEnumerator() | ForEach-Object { Write-Output ("  '{0}' hwnd={1}" -f $_.Key, $_.Value) }
