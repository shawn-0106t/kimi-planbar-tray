# Burst-capture the panel+settings region while clicking the panel's Settings button.
# Prereq: panel visible, settings closed. Captures pre-click frames too.
param(
    [int]$DurationMs = 2200,
    [string]$OutDir,
    [int]$ClickX = 2518, [int]$ClickY = 1565, [int]$ClickAtMs = 500
)
& "$PSScriptRoot\burst_capture.ps1" -Left 900 -Top 300 -Width 1980 -Height 1392 `
    -DurationMs $DurationMs -OutDir $OutDir -ClickX $ClickX -ClickY $ClickY -ClickAtMs $ClickAtMs -Prefix stg
