param([Parameter(Mandatory=$true)][int]$Season,[Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
if([int][math]::Floor($Season/10000)+1 -ne $Season%10000){throw 'Invalid season'}
if(Test-Path -LiteralPath $OutputDirectory){throw 'Use a new capture directory'}
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
$encoding=New-Object System.Text.UTF8Encoding($false)
$manifest=@()
foreach($entry in @(@('skaters','skater/summary'),@('faceoffs','skater/faceoffwins'),@('goalies','goalie/summary'),@('saves','goalie/savesByStrength'))){
 $kind=$entry[0];$route=$entry[1];$raw=Join-Path $OutputDirectory "raw/$kind";New-Item -ItemType Directory -Path $raw -Force | Out-Null
 $rows=@();$total=$null;$urls=@()
 for($offset=0;$null -eq $total -or $offset -lt $total;$offset+=100){
  $sort=[uri]::EscapeDataString('[{"property":"playerId","direction":"ASC"}]');$filter=[uri]::EscapeDataString("seasonId=$Season and gameTypeId=2")
  $url="https://api.nhle.com/stats/rest/en/${route}?isAggregate=false&isGame=false&start=$offset&limit=100&sort=$sort&cayenneExp=$filter";$path=Join-Path $raw "$offset.json"
  Invoke-WebRequest $url -UseBasicParsing -OutFile $path -TimeoutSec 30
  $page=Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
  if($null -eq $total){$total=$page.total};if($page.total -ne $total -or $page.data.Count -eq 0){throw 'Inconsistent pagination'}
  $rows+=$page.data;$urls+=$url;$manifest+=@{kind=$kind;url=$url;path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower();accessed_utc=[DateTime]::UtcNow.ToString('o')}
 }
 if($rows.Count -ne $total -or @($rows.playerId | Sort-Object -Unique).Count -ne $total -or @($rows | Where-Object {$_.seasonId -ne $Season}).Count -ne 0){throw 'Incomplete or mixed actor/year cohort'}
 [System.IO.File]::WriteAllText((Join-Path $OutputDirectory "$kind.json"),(@{season=$Season;route=$route;source_urls=$urls;total=$total;data=$rows}|ConvertTo-Json -Depth 14 -Compress),$encoding)
 Write-Output "$kind rows=$total"
}
[System.IO.File]::WriteAllText((Join-Path $OutputDirectory 'capture.json'),(@{season=$Season;pages=$manifest}|ConvertTo-Json -Depth 10),$encoding)
