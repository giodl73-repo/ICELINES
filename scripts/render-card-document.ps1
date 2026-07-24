param(
    [Parameter(Mandatory = $true)]
    [string[]]$Path,

    [string]$OutDir = (Join-Path $PSScriptRoot "..\dist\cards"),

    [ValidateRange(800, 2400)]
    [int]$Width = 1200,

    [switch]$Pdf,

    [switch]$ResolveAssets
)

$ErrorActionPreference = "Stop"
$RendererId = "icelines-reference-svg.v1"
$AssetCache = @{}
$AssetClient = if ($ResolveAssets) { [System.Net.Http.HttpClient]::new() } else { $null }
if ($AssetClient) {
    $AssetClient.Timeout = [TimeSpan]::FromSeconds(12)
    $AssetClient.DefaultRequestHeaders.UserAgent.ParseAdd("IceLines/$RendererId")
}

function Escape-Xml([AllowNull()][string]$Value) {
    if ($null -eq $Value) { return "" }
    return [System.Security.SecurityElement]::Escape($Value)
}

function Valid-Color([AllowNull()][string]$Value, [string]$Fallback) {
    if ($Value -match '^#[0-9a-fA-F]{6}$') { return $Value }
    return $Fallback
}

function Initials([string]$Name) {
    $parts = @($Name -split '[\s-]+' | Where-Object { $_ })
    if ($parts.Count -eq 0) { return "?" }
    return (($parts | Select-Object -First 2 | ForEach-Object { $_.Substring(0, 1) }) -join '').ToUpperInvariant()
}

function Add-SourceText {
    param(
        [System.Text.StringBuilder]$Svg,
        [System.Collections.Generic.List[string]]$Expected,
        [string]$Text,
        [int]$X,
        [int]$Y,
        [string]$Class = "body",
        [string]$Anchor = "start"
    )
    if ([string]::IsNullOrWhiteSpace($Text)) { return }
    [void]$Expected.Add($Text)
    $escaped = Escape-Xml $Text
    [void]$Svg.AppendLine("<text class=`"$Class`" x=`"$X`" y=`"$Y`" text-anchor=`"$Anchor`" data-source-text=`"true`">$escaped</text>")
}

function Add-SectionHeading {
    param(
        [System.Text.StringBuilder]$Svg,
        [System.Collections.Generic.List[string]]$Expected,
        [string]$Text,
        [ref]$Y,
        [int]$Width
    )
    $Y.Value += 22
    Add-SourceText $Svg $Expected $Text 48 $Y.Value "section"
    [void]$Svg.AppendLine("<line x1=`"48`" y1=`"$($Y.Value + 12)`" x2=`"$($Width - 48)`" y2=`"$($Y.Value + 12)`" class=`"rule`" />")
    $Y.Value += 34
}

function Add-WrappedSourceText {
    param(
        [System.Text.StringBuilder]$Svg,
        [System.Collections.Generic.List[string]]$Expected,
        [string]$Text,
        [int]$X,
        [ref]$Y,
        [int]$AvailableWidth,
        [string]$Class = "meta"
    )
    if ([string]::IsNullOrWhiteSpace($Text)) { return }
    [void]$Expected.Add($Text)
    $estimatedCharsPerLine = [math]::Max(30, [math]::Floor($AvailableWidth / 7.2))
    $lineCount = [math]::Max(1, [math]::Ceiling($Text.Length / $estimatedCharsPerLine))
    $height = $lineCount * 22
    $escaped = Escape-Xml $Text
    [void]$Svg.AppendLine("<foreignObject x=`"$X`" y=`"$($Y.Value - 16)`" width=`"$AvailableWidth`" height=`"$height`"><div xmlns=`"http://www.w3.org/1999/xhtml`" class=`"$Class wraptext`" data-source-text=`"true`">$escaped</div></foreignObject>")
    $Y.Value += $height
}

function Asset-ForSlot($Document, $Slot) {
    if (-not $Slot.asset_id) { return $null }
    return @($Document.assets | Where-Object { $_.id -eq $Slot.asset_id } | Select-Object -First 1)[0]
}

function Resolve-AssetDataUri($Asset) {
    if (-not $ResolveAssets -or -not $Asset -or $Asset.state -ne "available" -or
        $Asset.reference.reference_type -ne "external_url") {
        return $null
    }
    $url = [string]$Asset.reference.value
    if ($url -notmatch '^https://') { return $null }
    if ($AssetCache.ContainsKey($url)) {
        $cached = [string]$AssetCache[$url]
        return $(if ($cached) { $cached } else { $null })
    }

    try {
        $response = $AssetClient.GetAsync($url).GetAwaiter().GetResult()
        try {
            if (-not $response.IsSuccessStatusCode) { throw "HTTP $([int]$response.StatusCode)" }
            $mediaType = [string]$response.Content.Headers.ContentType.MediaType
            if ($mediaType -notmatch '^image/') { throw "unexpected content type '$mediaType'" }
            $bytes = $response.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult()
            $dataUri = "data:$mediaType;base64,$([Convert]::ToBase64String($bytes))"
            $AssetCache[$url] = $dataUri
            return $dataUri
        }
        finally {
            $response.Dispose()
        }
    }
    catch {
        # The document remains authoritative: an unresolved optional asset is
        # omitted and the supplied player label becomes the initials fallback.
        $AssetCache[$url] = ""
        return $null
    }
}

function Render-CardPage($Document, $Page, [string]$OutputPath, [int]$CanvasWidth) {
    if ($Document.schema -ne "card_document.v1") {
        throw "Unsupported card schema '$($Document.schema)'"
    }
    if ($Document.fingerprint -notmatch '^[0-9a-f]{64}$') {
        throw "Card fingerprint is not a lowercase SHA-256 value"
    }

    $team = if ($Document.theme.team_abbreviation) { $Document.theme.team_abbreviation } else { $Document.theme.ascii_identity }
    $primary = Valid-Color $Document.theme.primary "#14325c"
    $secondary = Valid-Color $Document.theme.secondary "#e6edf7"
    $accent = Valid-Color $Document.theme.accent "#ef3340"
    $surface = Valid-Color $Document.theme.surface "#ffffff"
    $ink = Valid-Color $Document.theme.text "#101820"
    $pageLabel = if ($Page.display_label) { $Page.display_label } else { $Page.literal_label }
    $expected = [System.Collections.Generic.List[string]]::new()
    $svg = [System.Text.StringBuilder]::new()
    $heightToken = "__CARD_HEIGHT__"
    $metadata = [ordered]@{
        document_id = $Document.document_id
        document_schema = $Document.schema
        document_fingerprint = $Document.fingerprint
        renderer_id = $RendererId
        asset_mode = if ($ResolveAssets) { "embedded_verified_urls" } else { "fallback_initials" }
        page_id = $Page.id
        page_order = $Page.order
    } | ConvertTo-Json -Compress

    [void]$svg.AppendLine("<svg xmlns=`"http://www.w3.org/2000/svg`" viewBox=`"0 0 $CanvasWidth $heightToken`" width=`"$CanvasWidth`" height=`"$heightToken`" role=`"img`" aria-labelledby=`"card-title card-desc`">")
    [void]$svg.AppendLine("<metadata>$(Escape-Xml $metadata)</metadata>")
    [void]$svg.AppendLine("<title id=`"card-title`">$(Escape-Xml "$($Document.title) - $pageLabel")</title>")
    [void]$svg.AppendLine("<desc id=`"card-desc`">$(Escape-Xml $Page.accessible_summary)</desc>")
    [void]$svg.AppendLine("<style>text{font-family:Inter,Segoe UI,Arial,sans-serif;fill:$ink}.title{font-size:38px;font-weight:800;fill:#fff}.subtitle{font-size:19px;fill:#fff}.section{font-size:22px;font-weight:800;fill:$primary}.group{font-size:17px;font-weight:800;fill:$primary}.name{font-size:16px;font-weight:700}.slot{font-size:13px;fill:#596579}.score{font-size:24px;font-weight:900;fill:$primary}.body{font-size:16px}.meta{font-size:13px;fill:#687487}.warning{font-size:15px;font-weight:700;fill:#8a3700}.wraptext{font-family:Inter,Segoe UI,Arial,sans-serif;line-height:1.4;overflow-wrap:anywhere}.rule{stroke:$secondary;stroke-width:2}.tile{fill:$surface;stroke:$secondary;stroke-width:2}.canvas{fill:#f4f7fb}.hero{fill:$primary}.accent{fill:$accent}</style>")
    [void]$svg.AppendLine("<rect class=`"canvas`" width=`"$CanvasWidth`" height=`"$heightToken`" />")
    [void]$svg.AppendLine("<rect class=`"hero`" width=`"$CanvasWidth`" height=`"190`" />")
    [void]$svg.AppendLine("<rect class=`"accent`" y=`"180`" width=`"$CanvasWidth`" height=`"10`" />")

    Add-SourceText $svg $expected $Document.title 48 70 "title"
    if ($Document.subtitle) { Add-SourceText $svg $expected $Document.subtitle 48 105 "subtitle" }
    Add-SourceText $svg $expected $pageLabel 48 150 "subtitle"
    Add-SourceText $svg $expected $Page.accessible_summary ($CanvasWidth - 48) 150 "subtitle" "end"
    $y = 220
    $resolvedAssetCount = 0
    $fallbackAssetCount = 0

    foreach ($warning in @($Document.warnings)) {
        Add-SourceText $svg $expected "WARNING: $($warning.message)" 48 $y "warning"
        $y += 24
    }

    foreach ($section in @($Page.sections)) {
        switch ($section.section_type) {
            "identity_header" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                if ($section.subtitle) {
                    Add-SourceText $svg $expected $section.subtitle 48 $y "meta"
                    $y += 24
                }
            }
            "lineup" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                foreach ($group in @($section.groups)) {
                    Add-SourceText $svg $expected $group.label 48 $y "group"
                    $y += 14
                    $slots = @($group.slots)
                    $gap = 14
                    for ($rowStart = 0; $rowStart -lt $slots.Count; $rowStart += 3) {
                        $rowSlots = @($slots | Select-Object -Skip $rowStart -First 3)
                        $tileWidth = [math]::Floor(($CanvasWidth - 96 - (($rowSlots.Count - 1) * $gap)) / [math]::Max(1, $rowSlots.Count))
                        $x = 48
                        foreach ($slot in $rowSlots) {
                        $name = if ($slot.subject_label) { $slot.subject_label } else { "Open" }
                        $score = if (@($slot.metrics).Count -gt 0) { $slot.metrics[0].display_text } else { "NR" }
                        [void]$svg.AppendLine("<rect class=`"tile`" x=`"$x`" y=`"$y`" width=`"$tileWidth`" height=`"104`" rx=`"12`" />")
                        [void]$svg.AppendLine("<circle cx=`"$($x + 47)`" cy=`"$($y + 50)`" r=`"32`" fill=`"$secondary`" />")
                        Add-SourceText $svg $expected (Initials $name) ($x + 47) ($y + 57) "group" "middle"
                        $asset = Asset-ForSlot $Document $slot
                        $assetDataUri = Resolve-AssetDataUri $asset
                        if ($assetDataUri) {
                            $href = Escape-Xml $assetDataUri
                            [void]$svg.AppendLine("<image href=`"$href`" x=`"$($x + 15)`" y=`"$($y + 18)`" width=`"64`" height=`"64`" preserveAspectRatio=`"xMidYMid slice`" />")
                            $resolvedAssetCount++
                        }
                        elseif ($slot.asset_id) {
                            $fallbackAssetCount++
                        }
                        Add-SourceText $svg $expected $slot.label ($x + 92) ($y + 26) "slot"
                        Add-SourceText $svg $expected $name ($x + 92) ($y + 52) "name"
                        Add-SourceText $svg $expected $score ($x + 92) ($y + 84) "score"
                            $x += $tileWidth + $gap
                        }
                        $y += 118
                    }
                    $y += 8
                }
            }
            "metric_strip" {
                $heading = if ($section.title) { $section.title } else { "Metrics" }
                Add-SectionHeading $svg $expected $heading ([ref]$y) $CanvasWidth
                foreach ($metric in @($section.metrics)) {
                    Add-SourceText $svg $expected "$($metric.metric.label): $($metric.display_text)" 64 $y "body"
                    $y += 25
                }
            }
            "probability_range" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                foreach ($range in @($section.ranges)) {
                    Add-SourceText $svg $expected "$($range.label): $($range.display_text)" 64 $y "body"
                    $y += 25
                }
            }
            "scenario_bridge" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                Add-SourceText $svg $expected "$($section.from_label) -> $($section.to_label)" 64 $y "meta"
                $y += 25
                foreach ($metric in @($section.metrics)) {
                    Add-SourceText $svg $expected "$($metric.metric.label): $($metric.display_text)" 64 $y "body"
                    $y += 25
                }
            }
            "player_list" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                foreach ($row in @($section.rows)) {
                    Add-SourceText $svg $expected $row.name 64 $y "name"
                    $y += 23
                    foreach ($metric in @($row.metrics)) {
                        Add-SourceText $svg $expected "$($metric.metric.label): $($metric.display_text)" 86 $y "meta"
                        $y += 21
                    }
                    $y += 10
                }
            }
            "state_notice" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                if ($section.detail) {
                    Add-SourceText $svg $expected $section.detail 64 $y "body"
                    $y += 25
                }
                foreach ($warning in @($section.warnings)) {
                    Add-SourceText $svg $expected "WARNING: $($warning.message)" 64 $y "warning"
                    $y += 25
                }
            }
            "decision" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                Add-SourceText $svg $expected $section.recommendation 64 $y "name"
                $y += 26
                foreach ($reason in @($section.rationale)) {
                    Add-WrappedSourceText $svg $expected $reason 86 ([ref]$y) ($CanvasWidth - 150) "meta"
                }
                foreach ($alternative in @($section.alternatives)) {
                    Add-SourceText $svg $expected $alternative.label 86 $y "body"
                    $y += 22
                    if ($alternative.detail) {
                        Add-WrappedSourceText $svg $expected $alternative.detail 108 ([ref]$y) ($CanvasWidth - 172) "meta"
                    }
                }
            }
            "timeline" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                foreach ($item in @($section.items)) {
                    Add-SourceText $svg $expected "$($item.effective_at) - $($item.label)" 64 $y "body"
                    $y += 24
                    if ($item.detail) {
                        Add-WrappedSourceText $svg $expected $item.detail 86 ([ref]$y) ($CanvasWidth - 150) "meta"
                    }
                }
            }
            "methodology" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                foreach ($method in @($section.methods)) {
                    Add-SourceText $svg $expected "$($method.label) [$($method.version)]" 64 $y "name"
                    $y += 22
                    Add-WrappedSourceText $svg $expected $method.summary 86 ([ref]$y) ($CanvasWidth - 150) "meta"
                }
                foreach ($limit in @($section.limitations)) {
                    Add-WrappedSourceText $svg $expected "LIMIT: $limit" 64 ([ref]$y) ($CanvasWidth - 128) "warning"
                    $y += 4
                }
            }
            "provenance" {
                Add-SectionHeading $svg $expected $section.title ([ref]$y) $CanvasWidth
                foreach ($id in @($section.provenance_ids)) {
                    Add-SourceText $svg $expected $id 64 $y "meta"
                    $y += 21
                }
            }
            default {
                Add-SectionHeading $svg $expected $section.id ([ref]$y) $CanvasWidth
            }
        }
    }

    $y += 35
    Add-SourceText $svg $expected "Document $($Document.document_id)" 48 $y "meta"
    Add-SourceText $svg $expected "Fingerprint $($Document.fingerprint)" ($CanvasWidth - 48) $y "meta" "end"
    $height = $y + 35
    [void]$svg.AppendLine("</svg>")
    $content = $svg.ToString().Replace($heightToken, [string]$height)
    [System.IO.File]::WriteAllText($OutputPath, $content, [System.Text.UTF8Encoding]::new($false))

    [xml]$parsed = [System.IO.File]::ReadAllText($OutputPath, [System.Text.Encoding]::UTF8)
    $rendered = @($parsed.SelectNodes('//*[@data-source-text="true"]') | ForEach-Object { $_.InnerText })
    foreach ($text in $expected) {
        if ($rendered -cnotcontains $text) {
            throw "Rendered text validation failed for '$text' in $OutputPath"
        }
    }
    return [ordered]@{
        svg = $OutputPath
        expected_text_count = $expected.Count
        height = $height
        resolved_asset_count = $resolvedAssetCount
        fallback_asset_count = $fallbackAssetCount
        metadata = ($metadata | ConvertFrom-Json)
    }
}

function Find-ChromiumBrowser {
    foreach ($candidate in @(
        "$env:ProgramFiles(x86)\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "$env:LOCALAPPDATA\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "$env:ProgramFiles(x86)\Google\Chrome\Application\chrome.exe",
        "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe"
    )) {
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    return $null
}

$resolvedOut = [System.IO.Path]::GetFullPath($OutDir)
[System.IO.Directory]::CreateDirectory($resolvedOut) | Out-Null
$results = @()
foreach ($input in $Path) {
    $resolvedInput = (Resolve-Path -LiteralPath $input).Path
    $document = [System.IO.File]::ReadAllText($resolvedInput, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
    $team = if ($document.theme.team_abbreviation) { $document.theme.team_abbreviation.ToLowerInvariant() } else { "card" }
    foreach ($page in @($document.pages | Sort-Object order)) {
        $fileStem = if ($document.card_kind -in @("season_simulation", "forecast_movement", "forecast_history")) {
            ([string]$document.document_id) -replace '[^A-Za-z0-9._-]', '-'
        } else {
            $team
        }
        $svgPath = Join-Path $resolvedOut "$fileStem-$($page.id).svg"
        $result = Render-CardPage $document $page $svgPath $Width
        if ($Pdf) {
            $browser = Find-ChromiumBrowser
            if (-not $browser) { throw "Microsoft Edge or Google Chrome is required for -Pdf conversion" }
            $pdfPath = [System.IO.Path]::ChangeExtension($svgPath, ".pdf")
            $uri = ([System.Uri]$svgPath).AbsoluteUri
            & $browser --headless=new --disable-gpu --no-pdf-header-footer "--print-to-pdf=$pdfPath" $uri | Out-Null
            if (-not (Test-Path -LiteralPath $pdfPath)) { throw "PDF conversion did not create $pdfPath" }
            $sidecar = [ordered]@{
                document_id = $document.document_id
                document_schema = $document.schema
                document_fingerprint = $document.fingerprint
                renderer_id = $RendererId
                asset_mode = if ($ResolveAssets) { "embedded_verified_urls" } else { "fallback_initials" }
                source_svg = [System.IO.Path]::GetFileName($svgPath)
                page_id = $page.id
            } | ConvertTo-Json -Depth 4
            [System.IO.File]::WriteAllText("$pdfPath.render.json", $sidecar, [System.Text.UTF8Encoding]::new($false))
            $result.pdf = $pdfPath
        }
        $results += [pscustomobject]$result
    }
}

$manifestPath = Join-Path $resolvedOut "render-manifest.json"
$manifest = [ordered]@{
    renderer_id = $RendererId
    generated_artifacts = $results
} | ConvertTo-Json -Depth 8
[System.IO.File]::WriteAllText($manifestPath, $manifest, [System.Text.UTF8Encoding]::new($false))
$results | ForEach-Object { Write-Output "VALID $($_.svg) ($($_.expected_text_count) source texts)" }
Write-Output "MANIFEST $manifestPath"
if ($AssetClient) { $AssetClient.Dispose() }
