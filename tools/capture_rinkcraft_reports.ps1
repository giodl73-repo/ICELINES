param(
 [Parameter(Mandatory=$true)][int[]]$Seasons,
 [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference='Stop'
$encoding=New-Object System.Text.UTF8Encoding($false)
if(Test-Path -LiteralPath $OutputDirectory) {throw 'Use a new output directory; retained captures cannot be overwritten'}
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
foreach($season in $Seasons) {
 if([int][math]::Floor($season/10000)+1 -ne $season%10000) {throw 'Invalid season'}
 $destination=Join-Path $OutputDirectory "$season"
 New-Item -ItemType Directory -Path $destination | Out-Null
 $allRecords=@()
 $reports=@{}
 foreach($entry in @(@('skaters','skater/summary'),@('realtime','skater/realtime'),@('penalties','skater/penalties'),@('goalies','goalie/summary'),@('timeonice','skater/timeonice'))) {
  $kind=$entry[0];$route=$entry[1];$records=@();$rows=@();$total=$null
  $raw=Join-Path $destination "raw/$kind"
  New-Item -ItemType Directory -Path $raw -Force | Out-Null
  for($offset=0;$null -eq $total -or $offset -lt $total;$offset+=100) {
   $sort=[uri]::EscapeDataString('[{"property":"playerId","direction":"ASC"}]')
   $filter=[uri]::EscapeDataString("seasonId=$season and gameTypeId=2")
   $url="https://api.nhle.com/stats/rest/en/${route}?isAggregate=false&isGame=false&start=$offset&limit=100&sort=$sort&cayenneExp=$filter"
   $path=Join-Path $raw "$offset.json"
   Invoke-WebRequest $url -UseBasicParsing -OutFile $path -TimeoutSec 30
   $page=Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
   if($null -eq $total) {$total=$page.total}
   if($total -ne $page.total -or $page.data.Count -eq 0) {throw 'Inconsistent or empty pagination'}
   $rows+=$page.data
   $records+=@{url=$url;path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower();accessed_utc=[DateTime]::UtcNow.ToString('o');report=$kind}
  }
  if($rows.Count -ne $total -or @($rows.playerId | Sort-Object -Unique).Count -ne $total) {throw 'Partial/duplicate actor cohort'}
  if(@($rows | Where-Object {$_.seasonId -ne $season}).Count -ne 0) {throw 'Mixed seasons'}
  $reports[$kind]=@{total=$total;data=$rows}
  $allRecords+=$records
  Write-Output "$season $kind rows=$total pages=$($records.Count)"
 }
 $assembly=@{season=$season;source_urls=@($allRecords | Where-Object {$_.report -ne 'timeonice'} | ForEach-Object {$_.url})}
 foreach($key in @('skaters','realtime','penalties','goalies')) {$assembly[$key]=$reports[$key]}
 [System.IO.File]::WriteAllText((Join-Path $destination 'reports.json'),($assembly | ConvertTo-Json -Depth 14 -Compress),$encoding)
 [System.IO.File]::WriteAllText((Join-Path $destination 'timeonice.json'),($reports['timeonice'] | ConvertTo-Json -Depth 12 -Compress),$encoding)
 [System.IO.File]::WriteAllText((Join-Path $destination 'capture.json'),(@{season=$season;pages=$allRecords} | ConvertTo-Json -Depth 9),$encoding)
}
