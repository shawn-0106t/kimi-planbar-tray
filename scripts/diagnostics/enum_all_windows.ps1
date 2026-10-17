# Enumerate ALL top-level windows (visible or not) owned by the app PID.
# Collects (hwnd,pid,title,visible,rect) for every window, filters outside the callback
# (GetWindowThreadProcessId inside an EnumWindows callback misbehaves in PS).
param([int]$AppPid)
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WE {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern int GetWindowTextW(IntPtr h, StringBuilder sb, int n);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int idx);
    public struct RECT { public int L, T, R, B; }
    public static List<string> Rows = new List<string>();
    public static bool CB(IntPtr h, IntPtr l) {
        uint pid; GetWindowThreadProcessId(h, out pid);
        var sb = new StringBuilder(256); GetWindowTextW(h, sb, 256);
        RECT r; GetWindowRect(h, out r);
        long style = GetWindowLongPtrW(h, -16);
        long exstyle = GetWindowLongPtrW(h, -20);
        Rows.Add(string.Format("pid={0} hwnd=0x{1:X} vis={2} style=0x{3:X8} ex=0x{4:X8} rect=({5},{6},{7},{8}) title='{9}'",
            pid, h.ToInt64(), IsWindowVisible(h), style, exstyle, r.L, r.T, r.R - r.L, r.B - r.T, sb.ToString()));
        return true;
    }
    public static void Run() { Rows.Clear(); EnumWindows(CB, IntPtr.Zero); }
}
'@
[WE]::Run()
[WE]::Rows | Where-Object { $_ -like "pid=$AppPid *" }
Write-Output 'done'
