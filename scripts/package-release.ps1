param(
    [string]$BinaryPath = "",
    [string]$OutputDir = "",
    [string]$ArtifactName = "",
    [switch]$SkipBuild,
    [switch]$SkipSmoke
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $RepoRoot "dist\release"
}
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $BinaryPath = Join-Path $RepoRoot "target\release\icelines.exe"
}
if ([string]::IsNullOrWhiteSpace($ArtifactName)) {
    $ArtifactName = "icelines-windows-x86_64.zip"
}

if (-not $SkipBuild) {
    Write-Host "cargo build --release -p icelines-cli" -ForegroundColor Cyan
    & cargo build --release -p icelines-cli
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$Binary = Resolve-Path $BinaryPath
$BinaryLeaf = Split-Path $Binary -Leaf
$ArchivePath = Join-Path $OutputDir $ArtifactName
$ChecksumPath = "$ArchivePath.sha256"
$StageRoot = Join-Path $OutputDir ".stage"
$StageDir = Join-Path $StageRoot ([IO.Path]::GetFileNameWithoutExtension($ArtifactName))
$VerifyDir = Join-Path $OutputDir ".verify"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Remove-Item -LiteralPath $StageRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $VerifyDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $ArchivePath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $ChecksumPath -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

Copy-Item -LiteralPath $Binary -Destination (Join-Path $StageDir $BinaryLeaf)
$binaryHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Binary).Hash.ToLowerInvariant()

$version = & $Binary --version
if ($LASTEXITCODE -ne 0) {
    Write-Error "Unable to read binary version from $Binary"
}
$commit = (& git -C $RepoRoot rev-parse HEAD).Trim()
$manifest = @(
    "artifact=$ArtifactName",
    "binary=$BinaryLeaf",
    "version=$($version -join ' ')",
    "source_commit=$commit",
    "binary_sha256=$binaryHash",
    "built_at_utc=$((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))"
)
Set-Content -Path (Join-Path $StageDir "ICELINES-PACKAGE.txt") -Value $manifest -Encoding utf8

if ($ArtifactName.EndsWith(".zip", [StringComparison]::OrdinalIgnoreCase)) {
    Compress-Archive -Path (Join-Path $StageDir "*") -DestinationPath $ArchivePath -Force
    Expand-Archive -LiteralPath $ArchivePath -DestinationPath $VerifyDir -Force
    if (-not (Test-Path (Join-Path $VerifyDir $BinaryLeaf))) {
        Write-Error "Packaged archive does not contain $BinaryLeaf"
    }
    if (-not (Test-Path (Join-Path $VerifyDir "ICELINES-PACKAGE.txt"))) {
        Write-Error "Packaged archive does not contain ICELINES-PACKAGE.txt"
    }
    $archiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ArchivePath).Hash.ToLowerInvariant()
    Set-Content -Path $ChecksumPath -Value "$archiveHash  $ArtifactName" -Encoding ascii
    if (-not (Test-Path $ChecksumPath)) {
        Write-Error "Packaged archive checksum was not written to $ChecksumPath"
    }
} else {
    Write-Error "Only .zip artifacts are supported by this local packaging script. GitHub Actions still builds cross-platform tarballs."
}

if (-not $SkipSmoke) {
    Write-Host "powershell -ExecutionPolicy Bypass -File scripts\release-smoke.ps1 -SkipBuild -BinaryPath $Binary" -ForegroundColor Cyan
    & powershell -ExecutionPolicy Bypass -File (Join-Path $RepoRoot "scripts\release-smoke.ps1") -SkipBuild -BinaryPath $Binary
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

Remove-Item -LiteralPath $StageRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $VerifyDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "packaged release artifact: $ArchivePath" -ForegroundColor Green
Write-Host "packaged release checksum: $ChecksumPath" -ForegroundColor Green
