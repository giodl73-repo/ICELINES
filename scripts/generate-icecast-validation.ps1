param(
    [uint32[]]$Season = @(20212022, 20222023, 20232024, 20242025, 20252026),
    [uint32]$Trials = 1000,
    [string]$IceLinesPath = (Join-Path $PSScriptRoot "..\target\debug\icelines.exe"),
    [string]$OutputDir = (Join-Path $env:USERPROFILE ".icelines\reports\validation"),
    [switch]$NoRetrospectiveOpeningLineups,
    [switch]$ForceReplay,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ($Season.Count -lt 3) {
    throw "At least three seasons are required for IceCast validation."
}
if ($Trials -eq 0) {
    throw "Trials must be greater than zero."
}
for ($index = 0; $index -lt $Season.Count; $index++) {
    $value = $Season[$index]
    $startYear = [int]($value / 10000)
    $endYear = [int]($value % 10000)
    if ($endYear -ne $startYear + 1) {
        throw "Invalid NHL season '$value'; expected adjacent years such as 20242025."
    }
    if ($index -gt 0 -and $value -le $Season[$index - 1]) {
        throw "Seasons must be unique and supplied in strictly increasing order."
    }
}

$resolvedOutput = [IO.Path]::GetFullPath($OutputDir)
$resolvedIceLines = if ($PlanOnly) {
    [IO.Path]::GetFullPath($IceLinesPath)
} else {
    (Resolve-Path -LiteralPath $IceLinesPath).Path
}
if (-not $PlanOnly) {
    New-Item -ItemType Directory -Force -Path $resolvedOutput | Out-Null
}

function Read-ValidatedReplay {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][uint32]$ExpectedSeason
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Replay does not exist: $Path"
    }
    $artifact = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    if ($artifact.schema -ne "team_season_forecast.v1" -or
        [uint32]$artifact.season -ne $ExpectedSeason -or
        [int]$artifact.accuracy.final_games -le 0) {
        throw "Replay validation failed for season ${ExpectedSeason}: $Path"
    }
    $artifact
}

$plans = @()
$forecastPaths = @()
foreach ($value in $Season) {
    $startYear = [int]($value / 10000)
    $statsSeason = [uint32](($startYear - 1) * 10000 + $startYear)
    $forecastPath = Join-Path $resolvedOutput "icecast-$value-rolling-replay.json"
    $arguments = @(
        "icecast", "season",
        "--season", "$value",
        "--stats-season", "$statsSeason",
        "--replay-mode", "rolling"
    )
    if (-not $NoRetrospectiveOpeningLineups) {
        $arguments += "--retrospective-opening-lineups"
    }
    $arguments += @(
        "--all-games",
        "--trials", "$Trials",
        "--seed", "$value",
        "--json",
        "--out", $forecastPath
    )
    $plans += [pscustomobject]@{
        Kind = "Replay"
        Season = $value
        StatsSeason = $statsSeason
        Output = $forecastPath
        Arguments = $arguments
        ReuseValidReplay = -not $ForceReplay
    }
    $forecastPaths += $forecastPath
}

$validationPath = Join-Path $resolvedOutput "icecast-validation-$($Season[0])-$($Season[-1]).json"
$backtestArguments = @("icecast", "backtest")
foreach ($forecastPath in $forecastPaths) {
    $backtestArguments += @("--input", $forecastPath)
}
$backtestArguments += @("--json", "--out", $validationPath)
$plans += [pscustomobject]@{
    Kind = "Backtest"
    Season = $null
    StatsSeason = $null
    Output = $validationPath
    Arguments = $backtestArguments
}

if ($PlanOnly) {
    $plans
    return
}

foreach ($plan in @($plans | Where-Object Kind -eq "Replay")) {
    if ($plan.ReuseValidReplay -and (Test-Path -LiteralPath $plan.Output -PathType Leaf)) {
        try {
            $artifact = Read-ValidatedReplay -Path $plan.Output -ExpectedSeason $plan.Season
            Write-Host "Reusing valid replay for $($plan.Season): $($plan.Output)"
            Write-Host "  graded games: $($artifact.accuracy.final_games); roster authority: $($artifact.opening_roster_authority.status)"
            continue
        } catch {
            Write-Host "Existing replay for $($plan.Season) is invalid and will be regenerated: $($_.Exception.Message)"
        }
    }
    Write-Host "Generating rolling replay for $($plan.Season)..."
    & $resolvedIceLines @($plan.Arguments) | Write-Host
    if ($LASTEXITCODE -ne 0) {
        throw "IceCast replay failed for season $($plan.Season)."
    }
    $artifact = Read-ValidatedReplay -Path $plan.Output -ExpectedSeason $plan.Season
    Write-Host "  graded games: $($artifact.accuracy.final_games); roster authority: $($artifact.opening_roster_authority.status)"
}

$backtest = $plans[-1]
Write-Host "Building chronological validation artifact..."
& $resolvedIceLines @($backtest.Arguments) | Write-Host
if ($LASTEXITCODE -ne 0) {
    throw "IceCast cross-season backtest failed."
}
if (-not (Test-Path -LiteralPath $validationPath -PathType Leaf)) {
    throw "IceCast did not write the expected validation artifact: $validationPath"
}
$validation = Get-Content -LiteralPath $validationPath -Raw | ConvertFrom-Json
if ($validation.schema -ne "team_game_forecast_validation.v1") {
    throw "Unexpected validation schema '$($validation.schema)'."
}

Write-Output $validationPath
