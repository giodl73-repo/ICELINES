param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactPath,
    [string]$ChecksumPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Artifact = Get-Item -LiteralPath (Resolve-Path $ArtifactPath)
if ([string]::IsNullOrWhiteSpace($ChecksumPath)) {
    $ChecksumPath = "$($Artifact.FullName).sha256"
}
$Checksum = Get-Item -LiteralPath (Resolve-Path $ChecksumPath)

$expected = ((Get-Content -LiteralPath $Checksum.FullName -TotalCount 1) -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Artifact.FullName).Hash.ToLowerInvariant()
if ($expected -ne $actual) {
    Write-Error "Checksum mismatch for $($Artifact.Name): expected $expected, got $actual"
}

$VerifyDir = Join-Path ([IO.Path]::GetTempPath()) "icelines-release-verify-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Force -Path $VerifyDir | Out-Null

try {
    if ($Artifact.Name.EndsWith(".zip", [StringComparison]::OrdinalIgnoreCase)) {
        Expand-Archive -LiteralPath $Artifact.FullName -DestinationPath $VerifyDir -Force
    } elseif ($Artifact.Name.EndsWith(".tar.gz", [StringComparison]::OrdinalIgnoreCase)) {
        & tar -xzf $Artifact.FullName -C $VerifyDir
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    } else {
        Write-Error "Unsupported artifact type: $($Artifact.Name). Expected .zip or .tar.gz."
    }

    $manifest = Get-ChildItem -LiteralPath $VerifyDir -Recurse -File -Filter "ICELINES-PACKAGE.txt" | Select-Object -First 1
    if ($null -eq $manifest) {
        Write-Error "Archive does not contain ICELINES-PACKAGE.txt"
    }

    $binary = Get-ChildItem -LiteralPath $VerifyDir -Recurse -File |
        Where-Object { $_.Name -eq "icelines" -or $_.Name -eq "icelines.exe" } |
        Select-Object -First 1
    if ($null -eq $binary) {
        Write-Error "Archive does not contain icelines binary"
    }

    $manifestRows = @{}
    foreach ($line in Get-Content -LiteralPath $manifest.FullName) {
        $parts = $line -split '=', 2
        if ($parts.Count -eq 2) {
            $manifestRows[$parts[0]] = $parts[1]
        }
    }

    if (-not $manifestRows.ContainsKey("artifact") -or $manifestRows["artifact"] -ne $Artifact.Name) {
        Write-Error "Manifest artifact does not match archive name: expected $($Artifact.Name)"
    }
    if (-not $manifestRows.ContainsKey("binary") -or $manifestRows["binary"] -ne $binary.Name) {
        Write-Error "Manifest binary does not match archive binary: expected $($binary.Name)"
    }
    if (-not $manifestRows.ContainsKey("binary_sha256")) {
        Write-Error "Manifest does not contain binary_sha256"
    }
    $expectedBinaryHash = $manifestRows["binary_sha256"].ToLowerInvariant()
    $actualBinaryHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary.FullName).Hash.ToLowerInvariant()
    if ($expectedBinaryHash -ne $actualBinaryHash) {
        Write-Error "Manifest binary_sha256 mismatch for $($binary.Name): expected $expectedBinaryHash, got $actualBinaryHash"
    }

    Write-Host "checksum verified: $($Artifact.Name)" -ForegroundColor Green
    Write-Host "archive contains verified members: $($binary.Name), $($manifest.Name)" -ForegroundColor Green
    Write-Host "binary hash verified: $($binary.Name)" -ForegroundColor Green
    Write-Host "package manifest:" -ForegroundColor Cyan
    Get-Content -LiteralPath $manifest.FullName
} finally {
    Remove-Item -LiteralPath $VerifyDir -Recurse -Force -ErrorAction SilentlyContinue
}
