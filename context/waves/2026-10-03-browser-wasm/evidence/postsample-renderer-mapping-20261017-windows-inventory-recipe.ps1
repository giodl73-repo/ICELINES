param([Parameter(Mandatory = $true)][int]$RootProcessId)
$ErrorActionPreference = 'Stop'
$records = @(Get-CimInstance Win32_Process)
$rootRecord = @($records | Where-Object { [int]$_.ProcessId -eq $RootProcessId })
if ($rootRecord.Count -ne 1) { throw 'Owned browser root is missing or ambiguous.' }
$owned = @{}
$owned[$RootProcessId] = $rootRecord[0]
do {
    $added = $false
    foreach ($record in $records) {
        $recordId = [int]$record.ProcessId
        $parentId = [int]$record.ParentProcessId
        if (!$owned.ContainsKey($recordId) -and $owned.ContainsKey($parentId) -and $record.CreationDate -ge $owned[$parentId].CreationDate) {
            $owned[$recordId] = $record
            $added = $true
        }
    }
} while ($added)
$items = @()
foreach ($record in $owned.Values) {
    $commandText = [string]$record.CommandLine
    $kind = 'unclassified'
    $subtype = $null
    if ([int]$record.ProcessId -eq $RootProcessId) { $kind = 'browser' }
    elseif ($commandText -match '--type=([^\s"]+)') { $kind = $Matches[1] }
    if ($commandText -match '--utility-sub-type=([^\s"]+)') { $subtype = $Matches[1] }
    $items += [PSCustomObject]@{
        id = [int]$record.ProcessId
        parent_id = [int]$record.ParentProcessId
        created_at_utc = $record.CreationDate.ToUniversalTime().ToString('o')
        kind = $kind
        utility_subtype = $subtype
    }
}
[PSCustomObject]@{
    scope = 'Owned root and currently live descendants only. Full command lines inspected locally but never emitted. Roles do not identify app versus browser-UI renderer.'
    observed_at_utc = [DateTime]::UtcNow.ToString('o')
    root_id = $RootProcessId
    root_created_at_utc = $rootRecord[0].CreationDate.ToUniversalTime().ToString('o')
    processes = $items
} | ConvertTo-Json -Depth 5 -Compress
