# Repro helper: locate the tray icon and report screen/DPI metrics.
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing, System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KptDpi {
    [DllImport("user32.dll")] public static extern int GetDpiForSystem();
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[KptDpi]::SetProcessDPIAware() | Out-Null

$screen = [System.Windows.Forms.SystemInformation]::PrimaryMonitorSize
$wa = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
Write-Output ("PrimaryMonitorSize: {0}x{1}" -f $screen.Width, $screen.Height)
Write-Output ("WorkingArea: X={0} Y={1} W={2} H={3}" -f $wa.X, $wa.Y, $wa.Width, $wa.Height)
Write-Output ("GetDpiForSystem: {0}" -f [KptDpi]::GetDpiForSystem())

$auto = [System.Windows.Automation.AutomationElement]
$root = $auto::RootElement

function Find-InTray($parent, $label) {
    $all = $parent.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    for ($i = 0; $i -lt $all.Count; $i++) {
        $el = $all.Item($i)
        $name = $el.Current.Name
        if ($name -and $name -match 'Kimi') {
            $r = $el.Current.BoundingRectangle
            Write-Output ("[{0}] HIT name='{1}' class='{2}' rect=({3},{4},{5},{6})" -f $label, $name, $el.Current.ClassName, $r.X, $r.Y, $r.Width, $r.Height)
        }
    }
}

# Visible tray area
$tray = $root.FindFirst([System.Windows.Automation.TreeScope]::Children,
    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'Shell_TrayWnd')))
if ($tray) { Find-InTray $tray 'Shell_TrayWnd' } else { Write-Output 'Shell_TrayWnd not found' }

# Overflow flyout (may need to be open)
$ov = $root.FindFirst([System.Windows.Automation.TreeScope]::Children,
    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'NotifyIconOverflowWindow')))
if ($ov) { Find-InTray $ov 'Overflow' } else { Write-Output 'NotifyIconOverflowWindow not found' }

# Also list our app windows if visible
$all2 = $root.FindAll([System.Windows.Automation.TreeScope]::Children,
    [System.Windows.Automation.Condition]::TrueCondition)
for ($i = 0; $i -lt $all2.Count; $i++) {
    $el = $all2.Item($i)
    $name = $el.Current.Name
    if ($name -and $name -match 'Kimi') {
        $r = $el.Current.BoundingRectangle
        Write-Output ("[top-level] name='{0}' class='{1}' rect=({2},{3},{4},{5}) offscreen={6}" -f $name, $el.Current.ClassName, $r.X, $r.Y, $r.Width, $r.Height, $el.Current.IsOffscreen)
    }
}
