Get-Process | Where-Object { $_.ProcessName -match 'kimi|planbar' } | Select-Object Id, ProcessName, StartTime, Path | Format-List
