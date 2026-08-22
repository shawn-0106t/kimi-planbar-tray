# Dump all top-level windows of the running kimi-planbar-tray process:
# rect, visibility, GetDpiForWindow. Used to verify tray-panel placement.
param([int]$TargetPid = 0)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinDump {
    public static uint TargetPid;
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder sb, int max);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public static bool Callback(IntPtr h, IntPtr l) {
        uint pid;
        GetWindowThreadProcessId(h, out pid);
        if (pid == TargetPid) {
            RECT r; GetWindowRect(h, out r);
            uint dpi = GetDpiForWindow(h);
            var sb = new System.Text.StringBuilder(256);
            GetWindowText(h, sb, 256);
            Console.WriteLine(string.Format(
                "hwnd={0} visible={1} dpi={2} (x{3:F2}) rect={4}x{5} at ({6},{7}) title='{8}'",
                h, IsWindowVisible(h), dpi, dpi/96.0, r.Right-r.Left, r.Bottom-r.Top, r.Left, r.Top, sb));
        }
        return true;
    }
}
"@
if ($TargetPid -eq 0) {
    $TargetPid = (Get-Process kimi-planbar-tray | Select-Object -First 1).Id
}
[WinDump]::TargetPid = [uint32]$TargetPid
[WinDump]::EnumWindows([WinDump+EnumWindowsProc]{ param($h,$l) [WinDump]::Callback($h,$l) }, [IntPtr]::Zero) | Out-Null
