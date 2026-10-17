# Report rect + styles of the four app windows (hidden included), by title match.
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class KptWin2 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder sb, int max);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextLengthW(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out int pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int idx);
    public struct RECT { public int Left, Top, Right, Bottom; }
}
'@
$selfPid = [int]$args[0]
$cb = [KptWin2+EnumProc]{
    param($h, $l)
    try {
        $procId = 0
        [void][KptWin2]::GetWindowThreadProcessId($h, [ref]$procId)
        if ($procId -eq $selfPid) {
            $len = [KptWin2]::GetWindowTextLengthW($h)
            $sb = New-Object System.Text.StringBuilder ($len + 1)
            [void][KptWin2]::GetWindowTextW($h, $sb, $sb.Capacity)
            $r = New-Object KptWin2+RECT
            [void][KptWin2]::GetWindowRect($h, [ref]$r)
            $vis = [KptWin2]::IsWindowVisible($h)
            $st = [KptWin2]::GetWindowLongPtrW($h, -16)
            $ex = [KptWin2]::GetWindowLongPtrW($h, -20)
            Write-Output ("hwnd={0} '{1}' visible={2} rect=({3},{4})-({5},{6}) style=0x{7:x8} exstyle=0x{8:x8}" -f $h.ToInt64(), $sb.ToString(), $vis, $r.Left, $r.Top, $r.Right, $r.Bottom, $st, $ex)
        }
    } catch {
        Write-Output ("ERR: " + $_.Exception.Message)
    }
    return $true
}
[void][KptWin2]::EnumWindows($cb, [IntPtr]::Zero)
