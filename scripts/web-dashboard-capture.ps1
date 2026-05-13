param(
    [string]$BinaryPath = "",
    [int]$ServePort = 18988,
    [string]$OutputDir = "",
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $BinaryPath = Join-Path $RepoRoot "target\release\icelines.exe"
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $RepoRoot "dist\web-dashboard-captures"
}

function Resolve-Browser {
    $candidates = @(
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe"
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return (Resolve-Path $candidate).Path
        }
    }
    throw "No supported headless browser found. Install Microsoft Edge or Google Chrome, then rerun this script."
}

if (-not $SkipBuild) {
    Write-Host "cargo build --release -p icelines-cli" -ForegroundColor Cyan
    & cargo build --release -p icelines-cli
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$Binary = (Resolve-Path $BinaryPath).Path
$Browser = Resolve-Browser
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$stdoutPath = Join-Path $env:TEMP "icelines-web-dashboard-capture-stdout.txt"
$stderrPath = Join-Path $env:TEMP "icelines-web-dashboard-capture-stderr.txt"
Remove-Item -LiteralPath $stdoutPath -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $stderrPath -ErrorAction SilentlyContinue

$serve = Start-Process `
    -FilePath $Binary `
    -ArgumentList @("--no-live", "serve", "--no-open", "--port", "$ServePort") `
    -RedirectStandardOutput $stdoutPath `
    -RedirectStandardError $stderrPath `
    -WindowStyle Hidden `
    -PassThru

try {
    $baseUrl = "http://127.0.0.1:$ServePort"
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) {
        if ($serve.HasExited) {
            $stdout = if (Test-Path $stdoutPath) { Get-Content $stdoutPath -Raw } else { "" }
            $stderr = if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw } else { "" }
            throw "serve exited early with code $($serve.ExitCode)`nSTDOUT:`n$stdout`nSTDERR:`n$stderr"
        }
        try {
            Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/dashboard" -TimeoutSec 2 | Out-Null
            $ready = $true
            break
        } catch {
            Start-Sleep -Milliseconds 500
        }
    }
    if (-not $ready) {
        throw "Timed out waiting for $baseUrl/dashboard"
    }

    $captures = @(
        @{ Name = "dashboard-leaders-desktop"; Url = "$baseUrl/dashboard?workspace=/leaders"; Size = "1440,900" },
        @{ Name = "dashboard-poach-desktop"; Url = "$baseUrl/dashboard?workspace=/poach"; Size = "1440,900" },
        @{ Name = "dashboard-fantasy-mobile"; Url = "$baseUrl/dashboard?workspace=/fantasy"; Size = "390,844" },
        @{ Name = "dashboard-team-season-mobile"; Url = "$baseUrl/dashboard?workspace=/team/EDM/season"; Size = "390,844" }
    )

    foreach ($capture in $captures) {
        $path = Join-Path $OutputDir "$($capture.Name).png"
        Write-Host "capture: $($capture.Name) $($capture.Size)" -ForegroundColor Cyan
        & $Browser `
            "--headless=new" `
            "--disable-gpu" `
            "--hide-scrollbars" `
            "--window-size=$($capture.Size)" `
            "--screenshot=$path" `
            $capture.Url | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "browser capture failed for $($capture.Url)"
        }
        if (-not (Test-Path $path)) {
            throw "browser capture did not create $path"
        }
    }

    Write-Host "web dashboard captures written to $OutputDir" -ForegroundColor Green
}
finally {
    if (-not $serve.HasExited) {
        Stop-Process -Id $serve.Id -Force
        $serve.WaitForExit()
    }
}
