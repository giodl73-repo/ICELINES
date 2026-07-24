$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("icelines-card-render-" + [guid]::NewGuid().ToString("N"))
try {
    & (Join-Path $PSScriptRoot "render-card-document.ps1") `
        -Path @(
            (Join-Path $root "examples\team-prognosis-card-nyr-2026-27.json"),
            (Join-Path $root "examples\team-prognosis-card-sea-2026-27.json"),
            (Join-Path $root "examples\fantasy-roster-card-dexters-dawgs-2026-10-05.json"),
            (Join-Path $root "examples\fantasy-draft-card-dexters-dawgs-pick-7.json"),
            (Join-Path $root "examples\fantasy-morning-card-dexters-dawgs-2026-10-08.json"),
            (Join-Path $root "examples\fantasy-trade-card-dexters-dawgs-fox-rantanen.json"),
            (Join-Path $root "examples\season-simulation-card-nyr-2026-27.json"),
            (Join-Path $root "examples\season-simulation-card-sea-2026-27.json"),
            (Join-Path $root "examples\season-simulation-card-nyr-2024-25.json"),
            (Join-Path $root "examples\season-simulation-card-sea-2024-25.json")
        ) `
        -OutDir $temp

    $svgs = @(Get-ChildItem -LiteralPath $temp -Filter *.svg)
    if ($svgs.Count -ne 20) { throw "Expected twenty SVG pages, got $($svgs.Count)" }
    foreach ($svg in $svgs) {
        [xml]$xml = [System.IO.File]::ReadAllText($svg.FullName, [System.Text.Encoding]::UTF8)
        $metadata = $xml.SelectSingleNode('/*[local-name()="svg"]/*[local-name()="metadata"]').InnerText
        if ($metadata -notmatch 'icelines-reference-svg\.v1') {
            throw "Missing renderer metadata in $($svg.Name)"
        }
        if ($metadata -notmatch 'fallback_initials') {
            throw "Deterministic render did not declare fallback-initials asset mode in $($svg.Name)"
        }
        if ($xml.OuterXml -match '<image[^>]+href="https://') {
            throw "Deterministic render leaked a network-dependent image URL in $($svg.Name)"
        }
        $text = @($xml.SelectNodes('//*[@data-source-text="true"]') | ForEach-Object { $_.InnerText }) -join "`n"
        if ($text -notmatch 'Fingerprint [0-9a-f]{64}') { throw "Missing fingerprint text in $($svg.Name)" }
        if ($svg.Name -match 'depth-chart' -and ($text -notmatch 'Projected lineup' -or $text -notmatch '\bNR\b')) {
            throw "Depth page lacks lineup or explicit NR score content in $($svg.Name)"
        }
        if ($svg.Name -match 'insider' -and
            $svg.Name -notmatch '^season-simulation-' -and
            $text -notmatch 'WARNING:') {
            throw "Insider page lacks warnings in $($svg.Name)"
        }
        if ($svg.Name -eq 'card-roster.svg' -and
            ($text -notmatch 'Nathan MacKinnon' -or $text -notmatch 'BN4' -or $text -notmatch 'IR\+2')) {
            throw "Fantasy roster page lacks required lineup and reserve content"
        }
        if ($svg.Name -eq 'card-roster-insider.svg' -and
            ($text -notmatch 'Same day' -or $text -notmatch 'Best calendar complement: WSH \(Class 8\)')) {
            throw "Fantasy insider page lacks transaction rules or schedule decision"
        }
        if ($svg.Name -eq 'card-draft-board.svg' -and
            ($text -notmatch 'Draft Jason Robertson' -or $text -notmatch 'Fallback: William Nylander')) {
            throw "Fantasy draft board lacks the sealed pick or fallback"
        }
        if ($svg.Name -eq 'card-draft-insider.svg' -and
            ($text -notmatch 'Schedule diversity' -or $text -notmatch 'not current claims')) {
            throw "Fantasy draft insider lacks scoring evidence or fixture warning"
        }
        if ($svg.Name -eq 'card-morning-skate.svg' -and
            ($text -notmatch 'Move Justin Brazeau to IR\+1' -or $text -notmatch 'Nathan MacKinnon')) {
            throw "Fantasy morning skate lacks its first action or legal lineup"
        }
        if ($svg.Name -eq 'card-morning-insider.svg' -and
            ($text -notmatch 'Darren Raddysh' -or $text -notmatch 'Goalie start evidence')) {
            throw "Fantasy morning insider lacks pickup or goalie evidence"
        }
        if ($svg.Name -eq 'card-trade-board.svg' -and
            ($text -notmatch 'Adam Fox' -or $text -notmatch 'Mikko Rantanen' -or $text -notmatch 'Reasonable offer range')) {
            throw "Fantasy trade board lacks packages or recommendation"
        }
        if ($svg.Name -eq 'card-trade-insider.svg' -and
            ($text -notmatch 'Before and after' -or $text -notmatch 'Open slots after')) {
            throw "Fantasy trade insider lacks team impact or legality"
        }
        if ($svg.Name -eq 'season-simulation-NYR-20242025-insider.svg' -and
            ($text -notmatch 'Actual team result' -or $text -notmatch 'Completed-season calibration')) {
            throw "Historical NYR replay lacks actual-result or calibration sections"
        }
    }
    $decisionDocument = [System.IO.File]::ReadAllText(
        (Join-Path $root "examples\team-prognosis-card-nyr-2026-27.json"),
        [System.Text.Encoding]::UTF8
    ) | ConvertFrom-Json
    $decisionDocument.document_id = "renderer-decision-fixture"
    $decisionDocument.title = "Renderer decision fixture"
    $decisionDocument.theme.team_abbreviation = "FTY"
    $decisionDocument.pages[1].sections += [pscustomobject]@{
        section_type = "decision"
        id = "schedule-equivalence-fixture"
        title = "Schedule spread"
        recommendation = "Best calendar complement: SEA (Class 2)"
        rationale = @("Eight exact-date schedule classes are loaded.")
        alternatives = @(
            [pscustomobject]@{
                id = "schedule-class-2"
                label = "Class 2: SEA, VAN"
                detail = "41.2% average within-class overlap"
            }
        )
        action_id = $null
        token = "schedule_edge"
        evidence_label = "confirmed"
    }
    $decisionInput = Join-Path $temp "decision-fixture.json"
    $decisionOut = Join-Path $temp "decision"
    [System.IO.File]::WriteAllText(
        $decisionInput,
        ($decisionDocument | ConvertTo-Json -Depth 100),
        [System.Text.UTF8Encoding]::new($false)
    )
    & (Join-Path $PSScriptRoot "render-card-document.ps1") -Path $decisionInput -OutDir $decisionOut
    [xml]$decisionSvg = [System.IO.File]::ReadAllText(
        (Join-Path $decisionOut "fty-insider.svg"),
        [System.Text.Encoding]::UTF8
    )
    $decisionText = @($decisionSvg.SelectNodes('//*[@data-source-text="true"]') | ForEach-Object { $_.InnerText })
    foreach ($expected in @(
        "Best calendar complement: SEA (Class 2)",
        "Eight exact-date schedule classes are loaded.",
        "Class 2: SEA, VAN",
        "41.2% average within-class overlap"
    )) {
        if ($decisionText -cnotcontains $expected) {
            throw "Decision renderer omitted '$expected'"
        }
    }
    Write-Output "PASS reference card renderer: 20 SVG pages, metadata and source text validated"
    Write-Output "PASS reference card decision sections: recommendation, rationale and alternatives validated"
}
finally {
    if (Test-Path -LiteralPath $temp) {
        $resolvedTemp = (Resolve-Path -LiteralPath $temp).Path
        $systemTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
        if (-not $resolvedTemp.StartsWith($systemTemp, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove non-temp path $resolvedTemp"
        }
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}
