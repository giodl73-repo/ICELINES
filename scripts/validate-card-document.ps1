param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [string]$SchemaPath,

    [switch]$Summary
)

$ErrorActionPreference = "Stop"
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($SchemaPath)) {
    $SchemaPath = Join-Path $scriptRoot "..\design\schemas\card_document.v1.schema.json"
}
$documentPath = (Resolve-Path -LiteralPath $Path).Path
$schema = (Resolve-Path -LiteralPath $SchemaPath).Path

if (-not (Get-Command Test-Json -ErrorAction SilentlyContinue)) {
    $pwsh = Get-Command pwsh -ErrorAction SilentlyContinue
    if ($null -eq $pwsh) {
        throw "Card schema validation requires PowerShell 7 Test-Json (pwsh was not found)"
    }
    $arguments = @(
        "-NoProfile",
        "-File", $MyInvocation.MyCommand.Path,
        "-Path", $documentPath,
        "-SchemaPath", $schema
    )
    if ($Summary) { $arguments += "-Summary" }
    & $pwsh.Source @arguments
    exit $LASTEXITCODE
}

if (-not (Test-Json -LiteralPath $documentPath -SchemaFile $schema)) {
    throw "Card document does not satisfy card_document.v1: $documentPath"
}

$document = Get-Content -Raw -LiteralPath $documentPath | ConvertFrom-Json
if ($document.schema -ne "card_document.v1") {
    throw "Unsupported card schema '$($document.schema)'"
}
if ($document.fingerprint -notmatch "^[0-9a-f]{64}$") {
    throw "Card fingerprint is not a lowercase SHA-256 value"
}

if ($Summary) {
    Write-Output "$($document.title) [$($document.card_kind)]"
    foreach ($page in $document.pages | Sort-Object order) {
        $label = if ($page.display_label) { $page.display_label } else { $page.literal_label }
        Write-Output "Page $($page.order): $label"
        foreach ($section in $page.sections) {
            $title = if ($section.title) { $section.title } else { $section.id }
            Write-Output "  $($section.section_type): $title"
        }
    }
}

Write-Output "VALID card_document.v1 $documentPath"
