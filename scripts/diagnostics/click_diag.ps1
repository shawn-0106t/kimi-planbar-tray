# Diagnose click delivery: foreground window, panel styles, then click with hover verification.
param([int]$AppPid, [int]$X, [int]$Y)
Add-Type -AssemblyName UIAutomationClient, System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class KD {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int idx);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    public struct POINT { public int X; public int Y; }
}
'@
[KD]::SetProcessDPIAware() | Out-Null

$panel = [KD]::FindWindowW($null, 'Kimi Planbar Tray Main Panel')
Write-Output ("panel hwnd=0x{0:X}" -f $panel.ToInt64())
if ($panel -ne [IntPtr]::Zero) {
    $style = [KD]::GetWindowLongPtrW($panel, -16)   # GWL_STYLE
    $exstyle = [KD]::GetWindowLongPtrW($panel, -20) # GWL_EXSTYLE
    Write-Output ("style=0x{0:X8} exstyle=0x{1:X8}" -f $style, $exstyle)
    $ppid = 0; [KD]::GetWindowThreadProcessId($panel, [ref]$ppid) | Out-Null
    Write-Output "panel owner pid=$ppid"
}
$fg = [KD]::GetForegroundWindow()
$fgpid = 0; [KD]::GetWindowThreadProcessId($fg, [ref]$fgpid) | Out-Null
Write-Output ("foreground hwnd=0x{0:X} pid={1}" -f $fg.ToInt64(), $fgpid)

# Move to button, wait, screenshot to check hover state, then click
[KD]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 600
$p = New-Object KD+POINT; [KD]::GetCursorPos([ref]$p) | Out-Null
Write-Output ("cursor at ({0},{1})" -f $p.X, $p.Y)
$bmp = New-Object System.Drawing.Bitmap(500, 300)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($X - 250, $Y - 150, 0, 0, (New-Object System.Drawing.Size(500, 300)))
$out = "$PSScriptRoot\hover_probe.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
[KD]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[KD]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero)
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
$fg2 = [KD]::GetForegroundWindow()
$fgpid2 = 0; [KD]::GetWindowThreadProcessId($fg2, [ref]$fgpid2) | Out-Null
Write-Output ("after click: foreground pid={0}" -f $fgpid2)
Write-Output 'done'
