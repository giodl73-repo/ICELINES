$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$runner = Join-Path $PSScriptRoot "generate-icecast-validation.ps1"
$output = Join-Path ([IO.Path]::GetTempPath()) "icelines-validation-plan"
$seasons = @(20212022, 20222023, 20232024, 20242025, 20252026)
$plans = @(& $runner -Season $seasons -Trials 25 -OutputDir $output -IceLinesPath "missing-icelines.exe" -PlanOnly)

if ($plans.Count -ne 6) {
    throw "Expected five replay plans and one backtest plan; got $($plans.Count)."
}
for ($index = 0; $index -lt $seasons.Count; $index++) {
    $plan = $plans[$index]
    $expectedStatsSeason = [uint32](([int]($seasons[$index] / 10000) - 1) * 10000 + [int]($seasons[$index] / 10000))
    if ($plan.Kind -ne "Replay" -or $plan.Season -ne $seasons[$index] -or $plan.StatsSeason -ne $expectedStatsSeason) {
        throw "Unexpected replay plan at index $index."
    }
    if (-not $plan.ReuseValidReplay) {
        throw "Replay plan $($plan.Season) should reuse valid sealed output by default."
    }
    if ($plan.Arguments -notcontains "--retrospective-opening-lineups" -or
        $plan.Arguments -notcontains "--all-games") {
        throw "Replay plan $($plan.Season) lacks required historical flags."
    }
}

$backtest = $plans[-1]
if ($backtest.Kind -ne "Backtest" -or ($backtest.Arguments | Where-Object { $_ -eq "--input" }).Count -ne 5) {
    throw "Backtest plan does not consume all five replay artifacts."
}
if ($backtest.Output -notmatch "icecast-validation-20212022-20252026\.json$") {
    throw "Unexpected validation output path '$($backtest.Output)'."
}

$forcedPlans = @(& $runner -Season @(20232024, 20242025, 20252026) -ForceReplay -NoRetrospectiveOpeningLineups -PlanOnly)
foreach ($plan in @($forcedPlans | Where-Object Kind -eq "Replay")) {
    if ($plan.ReuseValidReplay -or $plan.Arguments -contains "--retrospective-opening-lineups") {
        throw "Forced replay plan did not honor force/no-retrospective switches."
    }
}

$rejected = $false
try {
    & $runner -Season @(20232024, 20222023, 20242025) -PlanOnly | Out-Null
} catch {
    $rejected = $_.Exception.Message -match "strictly increasing"
}
if (-not $rejected) {
    throw "Runner accepted non-chronological seasons."
}

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("icelines-validation-runner-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempRoot | Out-Null
try {
    $fakeScript = Join-Path $tempRoot "fake-icelines.ps1"
    $fakeCommand = Join-Path $tempRoot "fake-icelines.cmd"
    $invocationLog = Join-Path $tempRoot "invocations.txt"
    $fakeSource = @'
$ErrorActionPreference = "Stop"
$tokens = @($args)
function Get-ArgumentValue([string]$Name) {
    $index = [Array]::IndexOf($tokens, $Name)
    if ($index -lt 0 -or $index + 1 -ge $tokens.Count) { throw "Missing $Name" }
    $tokens[$index + 1]
}
$command = $tokens[1]
$out = Get-ArgumentValue "--out"
if ($command -eq "season") {
    $season = [uint32](Get-ArgumentValue "--season")
    $document = [ordered]@{
        schema = "team_season_forecast.v1"
        season = $season
        accuracy = [ordered]@{ final_games = 1312 }
        opening_roster_authority = [ordered]@{ status = "fixture_partial_evaluation" }
    }
} elseif ($command -eq "backtest") {
    $document = [ordered]@{ schema = "team_game_forecast_validation.v1" }
} else {
    throw "Unexpected fake command '$command'."
}
[IO.File]::WriteAllText($out, ($document | ConvertTo-Json -Depth 10), [Text.UTF8Encoding]::new($false))
Add-Content -LiteralPath $env:ICELINES_FAKE_LOG -Value $command
'@
    [IO.File]::WriteAllText($fakeScript, $fakeSource, [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText(
        $fakeCommand,
        "@echo off`r`npowershell.exe -NoProfile -ExecutionPolicy Bypass -File `"%~dp0fake-icelines.ps1`" %*`r`n",
        [Text.ASCIIEncoding]::new()
    )
    $env:ICELINES_FAKE_LOG = $invocationLog
    $fixtureSeasons = @(20232024, 20242025, 20252026)
    $fixtureOutput = Join-Path $tempRoot "output"

    & $runner -Season $fixtureSeasons -Trials 1 -IceLinesPath $fakeCommand -OutputDir $fixtureOutput | Out-Null
    if (@(Get-Content -LiteralPath $invocationLog).Count -ne 4) {
        throw "Initial run did not generate three replays and one backtest."
    }
    & $runner -Season $fixtureSeasons -Trials 1 -IceLinesPath $fakeCommand -OutputDir $fixtureOutput | Out-Null
    if (@(Get-Content -LiteralPath $invocationLog).Count -ne 5) {
        throw "Resume run did not reuse all valid replays."
    }
    $invalidReplay = Join-Path $fixtureOutput "icecast-20242025-rolling-replay.json"
    [IO.File]::WriteAllText($invalidReplay, '{"schema":"broken"}', [Text.UTF8Encoding]::new($false))
    & $runner -Season $fixtureSeasons -Trials 1 -IceLinesPath $fakeCommand -OutputDir $fixtureOutput | Out-Null
    if (@(Get-Content -LiteralPath $invocationLog).Count -ne 7) {
        throw "Resume run did not regenerate only the invalid replay before backtesting."
    }
    & $runner -Season $fixtureSeasons -Trials 1 -IceLinesPath $fakeCommand -OutputDir $fixtureOutput -ForceReplay | Out-Null
    if (@(Get-Content -LiteralPath $invocationLog).Count -ne 11) {
        throw "Force run did not regenerate all replays before backtesting."
    }
} finally {
    Remove-Item Env:ICELINES_FAKE_LOG -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $tempRoot) {
        $resolvedTemp = (Resolve-Path -LiteralPath $tempRoot).Path
        $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if (-not $resolvedTemp.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing fixture cleanup outside the temporary directory: $resolvedTemp"
        }
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}

Write-Output "IceCast validation runner plan and resume checks passed."
