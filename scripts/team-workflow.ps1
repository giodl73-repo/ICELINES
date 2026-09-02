param(
    [Parameter(Mandatory = $true)]
    [string]$Player,
    [Parameter(Mandatory = $true)]
    [string]$Team,
    [switch]$UseInstalled,
    [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"

function Invoke-Icelines {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Args,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    if ($UseInstalled) {
        $cmd = "icelines"
        $fullArgs = @("--no-setup", "--no-live") + $Args
    } else {
        $cmd = "cargo"
        $fullArgs = @(
            "run", "-q", "-p", "icelines-cli", "--bin", "icelines",
            "--", "--no-setup", "--no-live"
        ) + $Args
    }

    Write-Host "== $Name =="
    $output = & $cmd @fullArgs 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | Out-String)
    if ($exitCode -ne 0) {
        Write-Host $text
        throw "$Name failed with exit code $exitCode"
    }
    if ($OutDir -ne "") {
        New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
        $safeName = ($Name -replace "[^A-Za-z0-9_.-]", "-").ToLowerInvariant()
        $text | Set-Content -Encoding UTF8 -Path (Join-Path $OutDir "$safeName.txt")
    }
    return $text
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Text,
        [Parameter(Mandatory = $true)]
        [string]$Needle,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    if (-not $Text.Contains($Needle)) {
        throw "$Name did not contain expected text: $Needle"
    }
}

$teamDepth = Invoke-Icelines -Name "team-depth" -Args @("team", $Team, "--no-color")
Assert-Contains $teamDepth $Team "team-depth"
Assert-Contains $teamDepth "FORWARDS" "team-depth"
Assert-Contains $teamDepth "GOALIES" "team-depth"

$leaders = Invoke-Icelines -Name "leaders" -Args @("query", "leaders", "--team", $Team, "--top", "5")
Assert-Contains $leaders "Context:" "leaders"
Assert-Contains $leaders "source roster complete" "leaders"
Assert-Contains $leaders "active_filters -" "leaders"
Assert-Contains $leaders $Team "leaders"

$goalies = Invoke-Icelines -Name "goalies" -Args @("query", "goalies", "--team", $Team, "--top", "5")
Assert-Contains $goalies $Team "goalies"
Assert-Contains $goalies "QS%" "goalies"
Assert-Contains $goalies "SA/60" "goalies"

$signals = Invoke-Icelines -Name "signals" -Args @("signals", $Player)
Assert-Contains $signals "SIGNALS" "signals"
Assert-Contains $signals $Player "signals"
Assert-Contains $signals "Note: Signals are descriptive derived metrics" "signals"
Assert-Contains $signals "Disclaimer: Not a prediction" "signals"
Assert-Contains $signals "Unavailable Signals" "signals"

$signalsRoster = Invoke-Icelines -Name "signals-roster" -Args @("signals-roster", "--team", $Team)
Assert-Contains $signalsRoster "SIGNALS ROSTER" "signals-roster"
Assert-Contains $signalsRoster $Team "signals-roster"
Assert-Contains $signalsRoster "Team-scoped Signals discovery matrix" "signals-roster"
Assert-Contains $signalsRoster "Not a Signal leaderboard" "signals-roster"
Assert-Contains $signalsRoster $Player "signals-roster"

$teamExport = Invoke-Icelines -Name "export-team" -Args @("export", "md", "team", "--team", $Team, "--out", "-")
Assert-Contains $teamExport "## Disclosure" "export-team"
Assert-Contains $teamExport "not era-adjusted, predictive" "export-team"
Assert-Contains $teamExport "## Team roster Pts/82 SVG" "export-team"

$signalsExport = Invoke-Icelines -Name "export-signals" -Args @("export", "md", "signals", "--player", $Player, "--out", "-")
Assert-Contains $signalsExport "## Signals Scope" "export-signals"
Assert-Contains $signalsExport "Not a prediction, betting edge, injury signal" "export-signals"
Assert-Contains $signalsExport 'Signals remain outside `StatId`' "export-signals"

Write-Host "Rangers workflow proof passed for $Team / $Player."
