# Temp diagnostic: dump style/exstyle/rect/DWM frame bounds for egui tray windows.
param([int]$TargetPid)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class StyleDump {
    public static uint TargetPid;
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int i);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public static bool Callback(IntPtr h, IntPtr l) {
        uint pid;
        GetWindowThreadProcessId(h, out pid);
        if (pid == TargetPid) {
            RECT r; GetWindowRect(h, out r);
            RECT ef; int hr = DwmGetWindowAttribute(h, 9, out ef, 16); // DWMWA_EXTENDED_FRAME_BOUNDS
            var sb = new System.Text.StringBuilder(256);
            GetWindowText(h, sb, 256);
            long style = GetWindowLongPtrW(h, -16).ToInt64();  // GWL_STYLE
            long exstyle = GetWindowLongPtrW(h, -20).ToInt64(); // GWL_EXSTYLE
            Console.WriteLine(string.Format(
                "hwnd={0} vis={1} style=0x{2:X8} exstyle=0x{3:X8} rect=({4},{5})-({6},{7}) {8}x{9} dwm=({10},{11})-({12},{13}) hr={14} title='{15}'",
                h, IsWindowVisible(h), (ulong)style & 0xFFFFFFFF, (ulong)exstyle & 0xFFFFFFFF,
                r.Left, r.Top, r.Right, r.Bottom, r.Right-r.Left, r.Bottom-r.Top,
                hr==0?ef.Left:0, hr==0?ef.Top:0, hr==0?ef.Right:0, hr==0?ef.Bottom:0, hr, sb));
        }
        return true;
    }
}
"@
[StyleDump]::TargetPid = [uint32]$TargetPid
[StyleDump]::EnumWindows([StyleDump+EnumWindowsProc]{ param($h,$l) [StyleDump]::Callback($h,$l) }, [IntPtr]::Zero) | Out-Null
