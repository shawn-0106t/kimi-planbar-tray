<#
.SYNOPSIS
    Fault-injection helper: terminate the WebView2 processes of a given app
    instance so the WebView2 recovery path (SPEC 22) can be exercised.

.DESCRIPTION
    Filters msedgewebview2.exe by a user-data-dir substring in its command
    line, so WebView2 processes belonging to other applications are never
    touched. The browser process is the one without a --type= switch; renderers
    carry --type=renderer and the GPU process --type=gpu-process. Every matching
    process of that kind is stopped (a browser-process kill takes the whole
    tree down with it).

.PARAMETER TargetPid
    PID of the app main process (kimi-planbar-tray.exe). It is verified to be
    alive and printed for traceability; process ownership is matched by the
    user-data-dir substring, not by this PID.

.PARAMETER Kind
    browser | renderer | gpu. Default: renderer.

.PARAMETER DataDir
    User-data-dir substring. Defaults to the production WebView2 profile
    folder `com.shawnqi.kimiplanbartray` (under %LOCALAPPDATA%); pass
    `gui_test`-style names for a KPT_APP_DATA_DIR test instance or
    `kimi-planbar-tray-testui` for a --test-ui instance.

.EXAMPLE
    ./kill_webview2_process.ps1 -TargetPid 1234 -Kind renderer
#>
param(
    [Parameter(Mandatory = $true)][int]$TargetPid,
    [ValidateSet('browser', 'renderer', 'gpu')][string]$Kind = 'renderer',
    [string]$DataDir = 'com.shawnqi.kimiplanbartray'
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Process -Id $TargetPid -ErrorAction SilentlyContinue)) {
    throw "target process $TargetPid is not running"
}

$all = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
    Where-Object { $_.CommandLine -and $_.CommandLine -like "*$DataDir*" }

$selected = switch ($Kind) {
    'browser' { $all | Where-Object { $_.CommandLine -notmatch '--type=' } }
    'renderer' { $all | Where-Object { $_.CommandLine -match '--type=renderer' } }
    'gpu' { $all | Where-Object { $_.CommandLine -match '--type=gpu-process' } }
}

if (-not $selected) {
    Write-Output "no $Kind WebView2 process found for pid=$TargetPid data-dir=$DataDir"
    exit 1
}

foreach ($proc in $selected) {
    Write-Output ("kill kind={0} webview2-pid={1} app-pid={2} data-dir={3}" -f `
            $Kind, $proc.ProcessId, $TargetPid, $DataDir)
    Stop-Process -Id $proc.ProcessId -Force
}
