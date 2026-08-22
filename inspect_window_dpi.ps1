# One-shot: launch the release exe's --test-ui, then dump every window's
# rect (physical px) + GetDpiForWindow for exact size/DPI evidence.
$exe = "C:\Users\rexxa\Documents\trae_projects\github\kimi-planbar-tray\rust\src-tauri\target\release\kimi-planbar-tray.exe"
$log = "$env:TEMP\win-dpi-dump.txt"
Remove-Item $log -ErrorAction SilentlyContinue

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinApi {
    public static uint TargetPid;
    public static string LogPath;
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
            System.IO.File.AppendAllText(LogPath, string.Format(
                "hwnd={0} visible={1} dpi={2} (x{3:F2}) rect={4}x{5} at ({6},{7}) title='{8}'\n",
                h, IsWindowVisible(h), dpi, dpi/96.0, r.Right-r.Left, r.Bottom-r.Top, r.Left, r.Top, sb));
        }
        return true;
    }
}
"@

$proc = Start-Process -FilePath $exe -ArgumentList "--test-ui" -PassThru
Start-Sleep -Milliseconds 1800

[WinApi]::TargetPid = $proc.Id
[WinApi]::LogPath = $log
[WinApi]::EnumWindows([WinApi+EnumWindowsProc]{ param($h,$l) [WinApi]::Callback($h,$l) }, [IntPtr]::Zero) | Out-Null

Get-Content $log
Start-Sleep -Seconds 3  # let the test-ui instance self-exit
