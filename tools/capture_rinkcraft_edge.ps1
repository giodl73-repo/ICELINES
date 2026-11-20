param(
 [Parameter(Mandatory=$true)][string]$Catalog,
 [Parameter(Mandatory=$true)][string]$OutputDirectory,
 [ValidateRange(1,6)][int]$Concurrency=4
)
$ErrorActionPreference='Stop'
if(Test-Path -LiteralPath $OutputDirectory){throw 'Use a new directory; retained EDGE captures cannot be overwritten'}
$edgeCatalogPath=(Resolve-Path -LiteralPath $Catalog).Path
$edgeCatalog=Get-Content -LiteralPath $edgeCatalogPath -Raw -Encoding UTF8 | ConvertFrom-Json
if($edgeCatalog.schema -ne 'rinkcraft.public-preview.v1'){throw 'Expected an explicit public player/season catalog'}
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
$edgeCaptureRoot=(Resolve-Path -LiteralPath $OutputDirectory).Path
$edgeRequests=@(foreach($window in $edgeCatalog.windows){foreach($player in $window.players){
 if($player.id -le 0 -or $window.season -lt 20212022){throw 'Unsupported actor or EDGE season'}
 [pscustomobject]@{player_id=$player.id;season=$window.season;group=$player.group;expected_games=$player.games;route=if($player.group -eq 'Goalies'){'goalie-detail'}else{'skater-detail'}}
}})
if($edgeRequests.Count -eq 0 -or @($edgeRequests | ForEach-Object {"$($_.season)-$($_.player_id)"} | Sort-Object -Unique).Count -ne $edgeRequests.Count){throw 'Empty or duplicate actor/season request cohort'}
$edgeStarted=[DateTime]::UtcNow.ToString('o')
$edgeReceipts=@($edgeRequests | ForEach-Object -Parallel {
 $edgeRequest=$_
 $edgeDestination=Join-Path $using:edgeCaptureRoot "$($edgeRequest.season)"
 [System.IO.Directory]::CreateDirectory($edgeDestination) | Out-Null
 $edgeFile=Join-Path $edgeDestination "$($edgeRequest.player_id).json"
 $edgeUrl="https://api-web.nhle.com/v1/edge/$($edgeRequest.route)/$($edgeRequest.player_id)/$($edgeRequest.season)/2"
 $edgeReceipt=@{player_id=$edgeRequest.player_id;season=$edgeRequest.season;group=$edgeRequest.group;expected_games=$edgeRequest.expected_games;url=$edgeUrl;path=$edgeFile}
 try {
  $edgeResponse=Invoke-WebRequest -Uri $edgeUrl -OutFile $edgeFile -PassThru -TimeoutSec 20 -MaximumRedirection 0
  $edgeReceipt.status='captured';$edgeReceipt.http_status=[int]$edgeResponse.StatusCode
  $edgeReceipt.sha256=(Get-FileHash -LiteralPath $edgeFile -Algorithm SHA256).Hash.ToLower()
  $edgeReceipt.bytes=(Get-Item -LiteralPath $edgeFile).Length
 } catch {
  $edgeReceipt.status='unavailable';$edgeReceipt.error=$_.Exception.Message
 }
 $edgeReceipt.accessed_utc=[DateTime]::UtcNow.ToString('o')
 $edgeReceipt
 Start-Sleep -Milliseconds 40
 } -ThrottleLimit $Concurrency)
$edgeFinished=[DateTime]::UtcNow.ToString('o')
$edgeManifest=@{schema='icelines.edge-cohort-capture.v1';started_utc=$edgeStarted;finished_utc=$edgeFinished;catalog_sha256=(Get-FileHash -LiteralPath $edgeCatalogPath -Algorithm SHA256).Hash.ToLower();source_catalog=$edgeCatalogPath;concurrency=$Concurrency;records=@($edgeReceipts | Sort-Object season,player_id);scope='Requested NHL regular-season EDGE detail, all strengths; unavailable is not zero.'}
[System.IO.File]::WriteAllText((Join-Path $edgeCaptureRoot 'capture.json'),($edgeManifest | ConvertTo-Json -Depth 12),[System.Text.UTF8Encoding]::new($false))
Write-Output ($edgeReceipts | Group-Object status | Select-Object Name,Count | ConvertTo-Json -Compress)
