param(
    [uint32]$Season = 20242025,
    [uint32]$StatsSeason = 20232024,
    [string[]]$CheckpointDate = @("2025-01-31", "2025-02-28", "2025-03-31"),
    [uint32]$Trials = 1000,
    [uint64]$Seed = 20242025,
    [string]$IceLinesPath = (Join-Path $PSScriptRoot "..\target\debug\icelines.exe"),
    [string]$OutputDir = (Join-Path $PSScriptRoot "..\examples")
)

$ErrorActionPreference = "Stop"

if ($CheckpointDate.Count -lt 2) {
    throw "At least two checkpoint dates are required."
}

$parsedDates = @($CheckpointDate | ForEach-Object {
    $parsed = [DateTime]::MinValue
    if (-not [DateTime]::TryParseExact(
        $_,
        "yyyy-MM-dd",
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::None,
        [ref]$parsed
    )) {
        throw "Invalid checkpoint date '$_'; expected YYYY-MM-DD."
    }
    $parsed
})
for ($index = 1; $index -lt $parsedDates.Count; $index++) {
    if ($parsedDates[$index] -le $parsedDates[$index - 1]) {
        throw "Checkpoint dates must be in strictly increasing order."
    }
}

$resolvedIceLines = (Resolve-Path -LiteralPath $IceLinesPath).Path
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$resolvedOutput = (Resolve-Path -LiteralPath $OutputDir).Path
$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) ("icelines-history-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temporaryRoot | Out-Null

try {
    $forecastPaths = @()
    foreach ($date in $CheckpointDate) {
        $forecastPath = Join-Path $temporaryRoot "icecast-$date.json"
        & $resolvedIceLines icecast season `
            --season $Season `
            --stats-season $StatsSeason `
            --replay-mode rolling `
            --through $date `
            --trials $Trials `
            --seed $Seed `
            --json `
            --out $forecastPath
        if ($LASTEXITCODE -ne 0) {
            throw "IceCast checkpoint generation failed for $date."
        }
        $forecastPaths += $forecastPath
    }

    $firstDate = $CheckpointDate[0]
    $lastDate = $CheckpointDate[-1]
    $historyPath = Join-Path $resolvedOutput "icecast-history-$firstDate-to-$lastDate.json"
    $historyArgs = @("icecast", "history")
    foreach ($forecastPath in $forecastPaths) {
        $historyArgs += @("--input", $forecastPath)
    }
    $historyArgs += @("--json", "--out", $historyPath)
    & $resolvedIceLines @historyArgs
    if ($LASTEXITCODE -ne 0) {
        throw "IceCast history generation failed."
    }

    $generatedAt = ([DateTimeOffset]::new($parsedDates[-1].AddDays(1), [TimeSpan]::Zero)).ToString("yyyy-MM-ddTHH:mm:ssZ")
    $seasonLabel = "{0}-{1:D2}" -f [int]($Season / 10000), [int](($Season % 10000) % 100)
    $cards = @(
        @{ Team = "NYR"; Name = "New York Rangers"; File = "forecast-history-card-nyr-$seasonLabel.json" },
        @{ Team = "SEA"; Name = "Seattle Kraken"; File = "forecast-history-card-sea-$seasonLabel.json" }
    )
    foreach ($card in $cards) {
        $cardPath = Join-Path $resolvedOutput $card.File
        & $resolvedIceLines icecast history-card `
            --input $historyPath `
            --team $card.Team `
            --team-name $card.Name `
            --generated-at $generatedAt `
            --out $cardPath
        if ($LASTEXITCODE -ne 0) {
            throw "IceCast history card generation failed for $($card.Team)."
        }
    }

    Write-Output "Generated $($CheckpointDate.Count)-checkpoint history: $historyPath"
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        $resolvedTemporary = (Resolve-Path -LiteralPath $temporaryRoot).Path
        $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
        if (-not $resolvedTemporary.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing cleanup outside the temporary directory: $resolvedTemporary"
        }
        Remove-Item -LiteralPath $resolvedTemporary -Recurse -Force
    }
}
