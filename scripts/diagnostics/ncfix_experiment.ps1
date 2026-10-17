# Temp experiment: does DWMWA_NCRENDERING_POLICY=DWMNCRP_DISABLED remove the
# native frame that Win11 26H2 draws around the egui edition's windows?
# Launches --test-ui (shows all 4 windows ~6s), applies the attribute, screenshots.
param([string]$Exe = "egui/target/release/kimi-planbar-tray-egui.exe", [int]$Mode = 1, [int]$NcValue = 2)
$ErrorActionPreference = "Stop"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class NcFix {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder sb, int max);
    [DllImport("dwmapi.dll")] public static extern int DwmSetWindowAttribute(IntPtr h, int a, ref int v, int s);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int i);
    [DllImport("user32.dll")] public static extern IntPtr SetWindowLongPtrW(IntPtr h, int i, IntPtr v);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr ins, int x, int y, int cx, int cy, uint flags);
    [DllImport("comctl32.dll")] public static extern bool SetWindowSubclass(IntPtr h, SubclassProc cb, IntPtr id, IntPtr data);
    public delegate IntPtr SubclassProc(IntPtr h, uint msg, IntPtr wp, IntPtr lp, IntPtr id, IntPtr data);
    [DllImport("comctl32.dll")] public static extern IntPtr DefSubclassProc(IntPtr h, uint msg, IntPtr wp, IntPtr lp);
    public struct RECT { public int Left, Top, Right, Bottom; }
    // Subclass proc: fully collapse the non-client area (undo winit's
    // undecorated-shadow top+=1 hack that leaves a 1px gray strip).
    public static IntPtr NcCollapseProc(IntPtr h, uint msg, IntPtr wp, IntPtr lp, IntPtr id, IntPtr data) {
        const uint WM_NCCALCSIZE = 0x0083;
        if (msg == WM_NCCALCSIZE && wp != IntPtr.Zero) {
            return IntPtr.Zero; // leave rgrc[0] = full window rect, no NC area
        }
        return DefSubclassProc(h, msg, wp, lp);
    }
    public static SubclassProc KeepAlive = new SubclassProc(NcCollapseProc);
    public static uint TargetPid;
    public static int Mode; // 1 = set DWMWA_NCRENDERING_POLICY, 2 = strip WS_CAPTION bits, 3 = both
    public static int NcValue;
    public static bool Callback(IntPtr h, IntPtr l) {
        uint pid; GetWindowThreadProcessId(h, out pid);
        if (pid != TargetPid) return true;
        var sb = new System.Text.StringBuilder(256); GetWindowText(h, sb, 256);
        var t = sb.ToString();
        if (!t.StartsWith("Kimi")) return true;
        if ((Mode & 1) != 0) {
            int v = NcValue;
            int hr = DwmSetWindowAttribute(h, 2, ref v, 4); // DWMWA_NCRENDERING_POLICY
            Console.WriteLine("NCRP=" + v + " hr=" + hr + " on '" + t + "'");
        }
        if ((Mode & 8) != 0) {
            bool ok = SetWindowSubclass(h, KeepAlive, new IntPtr(0x4B50), IntPtr.Zero);
            // force NC recalc
            SetWindowPos(h, IntPtr.Zero, 0, 0, 0, 0, 0x0001|0x0002|0x0004|0x0020);
            Console.WriteLine("subclass ok=" + ok + " on '" + t + "'");
        }
        if ((Mode & 4) != 0) {
            int border = unchecked((int)0xFFFFFFFE); // DWMWA_COLOR_NONE
            int hr = DwmSetWindowAttribute(h, 34, ref border, 4); // DWMWA_BORDER_COLOR
            Console.WriteLine("BORDER_COLOR hr=" + hr + " on '" + t + "'");
        }
        if ((Mode & 2) != 0) {
            long style = GetWindowLongPtrW(h, -16).ToInt64(); // GWL_STYLE
            long cleared = style & ~(0x00C00000L | 0x00080000L | 0x00010000L | 0x00020000L | 0x00040000L); // CAPTION|SYSMENU|MAX|MIN|THICKFRAME
            SetWindowLongPtrW(h, -16, new IntPtr(cleared));
            SetWindowPos(h, IntPtr.Zero, 0, 0, 0, 0, 0x0001|0x0002|0x0004|0x0020); // NOSIZE|NOMOVE|NOZORDER|FRAMECHANGED
            Console.WriteLine("style stripped on '" + t + "'");
        }
        return true;
    }
}
"@
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$exePath = (Resolve-Path $Exe).Path
$p = Start-Process -FilePath $exePath -ArgumentList "--test-ui" -PassThru
Start-Sleep -Milliseconds 2500
[NcFix]::TargetPid = [uint32]$p.Id
[NcFix]::Mode = $Mode
[NcFix]::NcValue = $NcValue
[NcFix]::EnumWindows([NcFix+EnumWindowsProc]{ param($h,$l) [NcFix]::Callback($h,$l) }, [IntPtr]::Zero) | Out-Null
Start-Sleep -Milliseconds 800
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
Add-Type -AssemblyName System.Windows.Forms
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
$out = Join-Path $PSScriptRoot ("ncfix_mode{0}_v{1}.png" -f $Mode, $NcValue)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved $out"
Wait-Process -Id $p.Id -Timeout 10 -ErrorAction SilentlyContinue
