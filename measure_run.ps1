param(
    [Parameter(Mandatory=$true)][string]$ExePath,
    [Parameter(Mandatory=$true)][string]$RootName
)

# Sequential benchmark of a tray app: startup proxy, steady-state memory of the
# whole process tree (WebView2 children included), and idle CPU over 8s.
# The tree is rooted at the PID this script launched - never matched by process
# name - so a same-name instance that is already running neither pollutes the
# measurement nor gets killed at the end.
$ErrorActionPreference = "SilentlyContinue"

function Get-Tree {
    param([int]$RootId)
    $all = Get-CimInstance Win32_Process
    $ids = @{}
    $queue = @($RootId)
    while ($queue.Count -gt 0) {
        $p = $queue[0]
        if ($queue.Count -gt 1) { $queue = @($queue[1..($queue.Count - 1)]) } else { $queue = @() }
        if ($ids.ContainsKey([int]$p)) { continue }
        $ids[[int]$p] = $true
        $all | Where-Object { $_.ParentProcessId -eq $p } | ForEach-Object { $queue += $_.ProcessId }
    }
    return @(Get-Process | Where-Object { $ids.ContainsKey($_.Id) })
}

# --- launch, measure time until the process tree stops growing (startup proxy)
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $launched = Start-Process -FilePath $ExePath -PassThru
    Write-Output "launched pid: $($launched.Id)"
} catch {
    Write-Output "START FAILED: $($_.Exception.Message)"
    exit 1
}
$tree = @()
$lastCount = -1
$stableRounds = 0
while ($sw.Elapsed.TotalSeconds -lt 30) {
    Start-Sleep -Milliseconds 300
    $tree = Get-Tree -RootId $launched.Id
    if ($launched.HasExited -and $tree.Count -eq 0) {
        Write-Output "NOTE: launched process exited immediately (single-instance mutex? another $RootName already running). Nothing to measure."
        exit 1
    }
    if ($tree.Count -gt 0 -and $tree.Count -eq $lastCount) {
        $stableRounds++
        if ($stableRounds -ge 3) { break }
    } else { $stableRounds = 0 }
    $lastCount = $tree.Count
}
$startupMs = $sw.ElapsedMilliseconds

# --- let it settle through the initial quota fetch (fires at ~2s)
Start-Sleep -Seconds 15

$tree = Get-Tree -RootId $launched.Id
$cpu0 = @{}
$tree | ForEach-Object { $cpu0[$_.Id] = $_.CPU }
Start-Sleep -Seconds 8
$tree = Get-Tree -RootId $launched.Id
$cpuDelta = 0.0
$tree | ForEach-Object { if ($cpu0.ContainsKey($_.Id)) { $cpuDelta += ($_.CPU - $cpu0[$_.Id]) } }
$cpuPct = [math]::Round($cpuDelta / 8.0 / [Environment]::ProcessorCount * 100, 2)

$wsMB = [math]::Round(($tree | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 1)
$privMB = [math]::Round(($tree | Measure-Object PagedMemorySize64 -Sum).Sum / 1MB, 1)

$exeSizeMB = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)

Write-Output "== $RootName =="
Write-Output "exe size:        $exeSizeMB MB"
Write-Output "startup proxy:   $startupMs ms (process tree stable)"
Write-Output "processes:       $($tree.Count)"
$tree | Sort-Object WorkingSet64 -Descending | ForEach-Object {
    Write-Output ("  {0,-28} pid={1,-7} ws={2,8:N1} MB" -f $_.ProcessName, $_.Id, ($_.WorkingSet64 / 1MB))
}
Write-Output "TOTAL workset:   $wsMB MB"
Write-Output "TOTAL private:   $privMB MB"
Write-Output "idle CPU (8s):   $cpuPct %"

# Stop only the tree this script launched, children before the root, by PID.
Get-Tree -RootId $launched.Id | Sort-Object { $_.Id -eq $launched.Id } | Stop-Process -Force
Start-Sleep -Seconds 2
