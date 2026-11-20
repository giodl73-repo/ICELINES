param([Parameter(Mandatory=$true)][string]$InputProfiles,[Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
if(Test-Path -LiteralPath $OutputDirectory){throw 'Use a new career capture directory'}
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
$profiles=Get-Content -LiteralPath $InputProfiles -Raw -Encoding UTF8 | ConvertFrom-Json
$encoding=New-Object System.Text.UTF8Encoding($false);$actors=@();$captures=@()
foreach($property in $profiles.PSObject.Properties){
 $profile=$property.Value;$id=$profile.request.player_id;$key=$property.Name
 if($null -eq $id -or $id -lt 1){throw 'Explicit actor ID required'}
 $directory=Join-Path $OutputDirectory "$id";New-Item -ItemType Directory -Path $directory | Out-Null
 $url="https://api-web.nhle.com/v1/player/$id/landing";$path=Join-Path $directory 'landing.json'
 Invoke-WebRequest $url -UseBasicParsing -OutFile $path -TimeoutSec 30
 $landing=Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
 if($landing.playerId -ne $id){throw 'Landing actor mismatch'}
 $captures+=@{player_id=$id;kind='landing';url=$url;path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower();accessed_utc=[DateTime]::UtcNow.ToString('o')}
 $goalie=$landing.position -eq 'G';$reports=@{}
 $routes=if($goalie){@('goalie/summary')}else{@('skater/summary','skater/timeonice','skater/realtime','skater/penalties')}
 foreach($route in $routes){
  $kind=$route.Replace('/','-');$rows=@();$total=$null
  for($offset=0;$null -eq $total -or $offset -lt $total;$offset+=100){
   $sort=[uri]::EscapeDataString('[{"property":"seasonId","direction":"ASC"}]');$filter=[uri]::EscapeDataString("playerId=$id and gameTypeId=2")
   $url="https://api.nhle.com/stats/rest/en/${route}?isAggregate=false&isGame=false&start=$offset&limit=100&sort=$sort&cayenneExp=$filter"
   $path=Join-Path $directory "$kind-$offset.json";Invoke-WebRequest $url -UseBasicParsing -OutFile $path -TimeoutSec 30
   $page=Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
   if($null -eq $total){$total=$page.total};if($page.total -ne $total -or $page.data.Count -eq 0){throw 'Inconsistent career pagination'}
   $rows+=$page.data;$captures+=@{player_id=$id;kind=$kind;url=$url;path=$path;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLower();accessed_utc=[DateTime]::UtcNow.ToString('o')}
  }
  if($rows.Count -ne $total -or @($rows.seasonId|Sort-Object -Unique).Count -ne $total){throw 'Partial or duplicated career season assembly'}
  if(@($rows|Where-Object {$_.playerId -ne $id}).Count -ne 0){throw 'Career report actor mismatch'}
  $reports[$kind]=@{total=$total;data=$rows};Write-Output "$id $route $total seasons"
 }
 $actors+=@{key=$key;player_id=$id;landing=$landing;reports=$reports}
}
[System.IO.File]::WriteAllText((Join-Path $OutputDirectory 'input.json'),(@{schema='rinkcraft.career-capture.v1';players=$actors;captures=$captures}|ConvertTo-Json -Depth 40 -Compress),$encoding)
Write-Output "Captured $($actors.Count) actors in $($captures.Count) raw requests."
