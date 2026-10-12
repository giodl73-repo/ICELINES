param(
    [Parameter(Mandatory = $true)][int]$RootProcessId,
    [Parameter(Mandatory = $true)][string]$StopFile
)
$ErrorActionPreference = 'Stop'
$rootProcess = Get-Process -Id $RootProcessId
$rootStarted = $rootProcess.StartTime.ToUniversalTime()
$clock = [Diagnostics.Stopwatch]::StartNew()
$tracked = @{}
$lastDiscovery = -1000
while ($clock.ElapsedMilliseconds -lt 180000 -and !(Test-Path -LiteralPath $StopFile)) {
    $rootProcess.Refresh()
    if ($rootProcess.HasExited) { break }
    if ($clock.ElapsedMilliseconds - $lastDiscovery -ge 500) {
        $records = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, CreationDate)
        $owned = [Collections.Generic.HashSet[int]]::new()
        [void]$owned.Add($RootProcessId)
        do {
            $added = $false
            foreach ($record in $records) {
                if ($owned.Contains([int]$record.ParentProcessId) -and $record.CreationDate.ToUniversalTime() -ge $rootStarted) {
                    if ($owned.Add([int]$record.ProcessId)) { $added = $true }
                }
            }
        } while ($added)
        $tracked = @{}
        foreach ($processId in $owned) {
            try { $tracked[$processId] = Get-Process -Id $processId -ErrorAction Stop } catch { }
        }
        $lastDiscovery = $clock.ElapsedMilliseconds
    }
    $processes = @()
    foreach ($process in $tracked.Values) {
        try {
            $process.Refresh()
            if (!$process.HasExited -and $process.StartTime.ToUniversalTime() -ge $rootStarted) {
                $processes += @{ id = $process.Id; private_bytes = $process.PrivateMemorySize64; working_set_bytes = $process.WorkingSet64; lifetime_peak_working_set_bytes = $process.PeakWorkingSet64 }
            }
        } catch { }
    }
    if ($processes.Count -eq 0) { throw 'No owned browser processes observed.' }
    @{ elapsed_ms = $clock.ElapsedMilliseconds; processes = $processes } | ConvertTo-Json -Depth 4 -Compress
    Start-Sleep -Milliseconds 100
}
if ($clock.ElapsedMilliseconds -ge 180000) { throw 'Memory sampler exceeded its 180-second limit.' }
