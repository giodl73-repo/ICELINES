param(
    [switch]$Json,
    [switch]$Strict,
    [string]$OutFile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-HeaderValue {
    param(
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $escaped = [regex]::Escape($Name)
    $match = [regex]::Match(
        $Text,
        "(?im)^\*\*$escaped\*\*\s*:\s*(.+?)\s*$|^$escaped\s*:\s*(.+?)\s*$"
    )
    if (-not $match.Success) {
        return $null
    }
    foreach ($group in $match.Groups[1..2]) {
        if ($group.Success -and -not [string]::IsNullOrWhiteSpace($group.Value)) {
            return $group.Value.Trim()
        }
    }
    return $null
}

function Get-Title {
    param([Parameter(Mandatory = $true)][string]$Text)

    $match = [regex]::Match($Text, "(?m)^#\s+(.+?)\s*$")
    if ($match.Success) {
        return $match.Groups[1].Value.Trim()
    }
    return $null
}

function Get-Classification {
    param(
        [Parameter(Mandatory = $true)][string]$Kind,
        [AllowNull()][string]$Status
    )

    if ($Kind -eq "index") {
        return "index"
    }
    if ([string]::IsNullOrWhiteSpace($Status)) {
        return "needs_review"
    }
    $value = $Status.ToLowerInvariant()
    if ($Kind -eq "spec") {
        if ($value -match "cancelled|retired|superseded") { return "retired" }
        if ($value -match "deferred|parked") { return "deferred" }
        if ($value -match "implemented|complete|closed") { return "canonical_implemented" }
        if ($value -match "accepted|active|draft|partial") { return "active_spec" }
        return "needs_review"
    }
    if ($value -match "superseded") { return "superseded" }
    if ($value -match "closed|complete|implemented|wrapped") { return "historical_complete" }
    if ($value -match "active|in progress|planned|blocked|ready") { return "active_plan" }
    return "needs_review"
}

function Convert-ToRepoPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $root = (Get-Location).Path
    $full = [System.IO.Path]::GetFullPath($Path)
    return $full.Substring($root.Length).TrimStart('\', '/').Replace('\', '/')
}

function Get-LocalLinks {
    param(
        [Parameter(Mandatory = $true)][string]$SourcePath,
        [Parameter(Mandatory = $true)][string]$Text
    )

    $sourceDirectory = Split-Path -Parent $SourcePath
    $links = @()
    foreach ($match in [regex]::Matches($Text, '\[[^\]]+\]\(([^)]+)\)')) {
        $target = $match.Groups[1].Value.Trim().Trim('<', '>')
        if ($target -match '^(https?:|mailto:|file:|#)' -or $target.StartsWith('/')) {
            continue
        }
        $pathOnly = ($target -split '#', 2)[0]
        $pathOnly = ($pathOnly -split '\?', 2)[0]
        if ([string]::IsNullOrWhiteSpace($pathOnly)) {
            continue
        }
        $resolved = [System.IO.Path]::GetFullPath((Join-Path $sourceDirectory $pathOnly))
        $links += [ordered]@{
            target = $target
            resolved = Convert-ToRepoPath $resolved
            exists = Test-Path -LiteralPath $resolved
        }
    }
    return @($links)
}

$specIndexPath = "design/specs/INDEX.md"
$planIndexPath = "design/plans/INDEX.md"
$specIndex = Get-Content -Raw $specIndexPath
$planIndex = Get-Content -Raw $planIndexPath
$activeBlock = [regex]::Match(
    $planIndex,
    '(?s)<!-- active-plans:start -->(.*?)<!-- active-plans:end -->'
)
if (-not $activeBlock.Success) {
    throw "design/plans/INDEX.md is missing active-plan markers"
}
$activePlanPaths = @(
    [regex]::Matches($activeBlock.Groups[1].Value, '\[[^\]]+\]\(([^)#]+\.md)\)') |
        ForEach-Object { "design/plans/$($_.Groups[1].Value)" }
)

$markdownFiles = @(
    Get-ChildItem design/specs, design/plans -File -Filter '*.md' |
        Sort-Object FullName
)
$allDesignMarkdown = @(
    Get-ChildItem design -Recurse -File -Filter '*.md' |
        Sort-Object FullName
)

$inbound = @{}
$brokenLinks = @()
foreach ($file in $allDesignMarkdown) {
    $sourcePath = Convert-ToRepoPath $file.FullName
    $text = Get-Content -Raw $file.FullName
    foreach ($link in Get-LocalLinks $file.FullName $text) {
        if (-not $inbound.ContainsKey($link.resolved)) {
            $inbound[$link.resolved] = @()
        }
        $inbound[$link.resolved] += $sourcePath
        if (-not $link.exists) {
            $brokenLinks += [ordered]@{
                source = $sourcePath
                target = $link.target
                resolved = $link.resolved
            }
        }
    }
}

$documents = @()
foreach ($file in $markdownFiles) {
    $path = Convert-ToRepoPath $file.FullName
    $text = Get-Content -Raw $file.FullName
    $kind = if ($file.Name -eq 'INDEX.md') {
        'index'
    } elseif ($path.StartsWith('design/specs/')) {
        'spec'
    } else {
        'plan'
    }
    $title = Get-Title $text
    $status = Get-HeaderValue $text 'Status'
    $date = Get-HeaderValue $text 'Date'
    $specification = Get-HeaderValue $text 'Specification'
    $parentPlan = Get-HeaderValue $text 'Parent plan'
    $supersedes = Get-HeaderValue $text 'Supersedes'
    $archiveWhen = Get-HeaderValue $text 'Archive when'
    $classification = Get-Classification $kind $status
    $indexed = if ($kind -eq 'spec') {
        $specIndex.Contains("($($file.Name))")
    } elseif ($kind -eq 'plan') {
        $planIndex.Contains("($($file.Name))")
    } else {
        $true
    }
    $issues = @()
    if ($kind -ne 'index') {
        if ([string]::IsNullOrWhiteSpace($title)) { $issues += 'missing_title' }
        if ([string]::IsNullOrWhiteSpace($status)) { $issues += 'missing_status' }
        if ([string]::IsNullOrWhiteSpace($date)) { $issues += 'missing_date' }
        if (-not $indexed) { $issues += 'not_indexed' }
    }
    if ($kind -eq 'plan' -and $activePlanPaths -contains $path) {
        if ([string]::IsNullOrWhiteSpace($specification)) {
            $issues += 'active_plan_missing_specification'
        }
        if ([string]::IsNullOrWhiteSpace($archiveWhen)) {
            $issues += 'active_plan_missing_archive_condition'
        }
    }
    [array]$inboundSources = if ($inbound.ContainsKey($path)) {
        @($inbound[$path] | Sort-Object -Unique)
    } else {
        @()
    }
    $inboundCount = @($inboundSources).Count
    $documents += [ordered]@{
        path = $path
        kind = $kind
        title = $title
        date = $date
        status = $status
        classification = $classification
        canonical_active = $activePlanPaths -contains $path
        indexed = $indexed
        specification = $specification
        parent_plan = $parentPlan
        supersedes = $supersedes
        archive_when = $archiveWhen
        inbound_link_count = $inboundCount
        inbound_links = $inboundSources
        issues = @($issues)
    }
}

$classificationCounts = [ordered]@{}
foreach ($document in $documents) {
    $key = [string]$document['classification']
    if (-not $classificationCounts.Contains($key)) {
        $classificationCounts[$key] = 0
    }
    $classificationCounts[$key]++
}
$issueCounts = [ordered]@{}
foreach ($document in $documents) {
    foreach ($issue in @($document['issues'])) {
        if (-not $issueCounts.Contains($issue)) {
            $issueCounts[$issue] = 0
        }
        $issueCounts[$issue]++
    }
}

$result = [ordered]@{
    schema = 'icelines.documentation_inventory.v1'
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    totals = [ordered]@{
        documents = $documents.Count
        specs = @($documents | Where-Object kind -eq 'spec').Count
        plans = @($documents | Where-Object kind -eq 'plan').Count
        indexes = @($documents | Where-Object kind -eq 'index').Count
        canonical_active_plans = $activePlanPaths.Count
        documents_with_issues = @($documents | Where-Object { $_.issues.Count -gt 0 }).Count
        broken_local_links = $brokenLinks.Count
    }
    active_plans = $activePlanPaths
    classification_counts = $classificationCounts
    issue_counts = $issueCounts
    broken_links = @($brokenLinks)
    documents = @($documents)
}

$jsonText = $result | ConvertTo-Json -Depth 10
if (-not [string]::IsNullOrWhiteSpace($OutFile)) {
    $parent = Split-Path -Parent $OutFile
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Set-Content -Path $OutFile -Value $jsonText -Encoding utf8
}

if ($Json) {
    $jsonText
} else {
    Write-Host "IceLines documentation audit"
    Write-Host "  documents: $($result.totals.documents) ($($result.totals.specs) specs, $($result.totals.plans) plans, $($result.totals.indexes) indexes)"
    Write-Host "  canonical active plans: $($result.totals.canonical_active_plans)"
    Write-Host "  documents with issues: $($result.totals.documents_with_issues)"
    Write-Host "  broken local links: $($result.totals.broken_local_links)"
    foreach ($entry in $classificationCounts.GetEnumerator()) {
        Write-Host "  $($entry.Key): $($entry.Value)"
    }
    if ($OutFile) {
        Write-Host "  wrote: $OutFile"
    }
}

if ($Strict -and (
    $result.totals.broken_local_links -gt 0 -or
    $result.totals.documents_with_issues -gt 0 -or
    $result.totals.canonical_active_plans -gt 8
)) {
    exit 1
}
