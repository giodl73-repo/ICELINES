param(
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Read-Text {
    param([Parameter(Mandatory = $true)][string]$Path)
    return Get-Content -Raw -Path $Path
}

function Assert-Text {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][string]$Pattern
    )

    if ($Text -notmatch $Pattern) {
        throw "$Name missing expected pattern: $Pattern"
    }
}

function Assert-NoText {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][string]$Pattern
    )

    if ($Text -match $Pattern) {
        throw "$Name unexpectedly matched pattern: $Pattern"
    }
}

$root = Read-Text "Cargo.toml"
$fetch = Read-Text "icelines-fetch/Cargo.toml"
$query = Read-Text "icelines-query/Cargo.toml"
$cli = Read-Text "icelines-cli/src/cli.rs"
$sliceSelectors = Read-Text "icelines-query/src/slice_selectors.rs"

Assert-Text "workspace manifest" $root 'fletch-core\s*=\s*\{[^}]*git\s*=\s*"https://github\.com/giodl73-repo/FLETCH\.git"'
Assert-Text "workspace manifest" $root 'slice-core\s*=\s*\{[^}]*git\s*=\s*"https://github\.com/giodl73-repo/SLICE"'
Assert-Text "workspace manifest" $root 'rev\s*=\s*"353564781f6cad53fc5a934178a7927824824e3e"'
Assert-Text "icelines-fetch manifest" $fetch 'fletch-core\.workspace\s*=\s*true'
Assert-Text "icelines-query manifest" $query 'slice-core\.workspace\s*=\s*true'
Assert-Text "CLI command surface" $cli 'name\s*=\s*"fletch-sources"'
Assert-Text "CLI command surface" $cli 'name\s*=\s*"fletch-partitions"'
Assert-Text "CLI command surface" $cli 'name\s*=\s*"fletch-quivers"'
Assert-Text "CLI command surface" $cli 'name\s*=\s*"fletch-cache-index"'
Assert-Text "SLICE selector surface" $sliceSelectors 'slice_core'

Assert-NoText "workspace root feature boundary" $root '(?m)^\s*cli\s*='
Assert-NoText "icelines-cli feature boundary" (Read-Text "icelines-cli/Cargo.toml") '(?m)^\s*cli\s*='

$result = [ordered]@{
    status = "target-not-met"
    fletch_core = "workspace git dependency; consumed by icelines-fetch"
    slice_core = "workspace git dependency at rev 353564781f6cad53fc5a934178a7927824824e3e; consumed by icelines-query"
    fletch_commands = @(
        "fetch fletch-sources",
        "fetch fletch-partitions",
        "fetch fletch-quivers",
        "fetch fletch-cache-index"
    )
    slice_surface = "icelines-query/src/slice_selectors.rs"
    lean_cli_feature = "missing"
    claim = "No standalone or lean CLI support is claimed."
}

if ($Json) {
    $result | ConvertTo-Json -Depth 4
} else {
    Write-Host "Rangers lean audit: target-not-met"
    Write-Host "  fletch-core: workspace git dependency consumed by icelines-fetch"
    Write-Host "  slice-core: workspace git dependency consumed by icelines-query"
    Write-Host "  fletch commands: fetch fletch-sources, fletch-partitions, fletch-quivers, fletch-cache-index"
    Write-Host "  slice selector: icelines-query/src/slice_selectors.rs"
    Write-Host "  cli feature: missing"
    Write-Host "  claim: No standalone or lean CLI support is claimed."
}
