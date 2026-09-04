# Locate the tray icon via Shell_TrayWnd / NotifyIconOverflowWindow HWNDs
# (UIA root-descendant search misses them), invoke it, screenshot the panel.
param([string]$Out = "$PSScriptRoot\csp-check.png")

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class TrayHwnd {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string win);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string win);
}
"@

$btn = $null
$roots = @()
$shellTray = [TrayHwnd]::FindWindow("Shell_TrayWnd", $null)
if ($shellTray -ne [IntPtr]::Zero) { $roots += $shellTray }
$overflow = [TrayHwnd]::FindWindow("NotifyIconOverflowWindow", $null)
if ($overflow -ne [IntPtr]::Zero) { $roots += $overflow }
Write-Output ("roots: " + ($roots.Count))

foreach ($h in $roots) {
    $el = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    if ($null -eq $el) { continue }
    $btnCond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button)
    foreach ($b in $el.FindAll([System.Windows.Automation.TreeScope]::Descendants, $btnCond)) {
        $n = $b.Current.Name
        if ($n -match 'Kimi') { Write-Output ("candidate: [" + $n + "]") }
        if ($n -match 'Kimi Planbar Tray') { $btn = $b; break }
    }
    if ($null -ne $btn) { break }
}
if ($null -eq $btn) { Write-Output "TRAY-BUTTON-NOT-FOUND"; exit 1 }

$btn.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
Start-Sleep -Milliseconds 1500

$wa = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$w = 560; $h = 640
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($wa.Width - $w, $wa.Height - $h, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved: $Out"
