param(
    [string]$BinaryPath = "",
    [int]$ServePort = 18988,
    [string]$OutputDir = "",
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

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

function Assert-DashboardReady {
    param([string]$Url)

    $response = Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 10
    if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 300) {
        throw "dashboard route readiness failed for ${Url}: HTTP $($response.StatusCode)"
    }
    if (-not ($response.Content -like '*class="jaw-shell"*')) {
        throw "dashboard route readiness failed for ${Url}: dashboard shell marker missing"
    }
}

function Assert-Screenshot {
    param(
        [string]$Path,
        [string]$Size
    )

    if (-not (Test-Path $Path)) {
        throw "browser capture did not create $Path"
    }

    $parts = $Size.Split(",")
    if ($parts.Length -ne 2) {
        throw "invalid capture size '$Size'"
    }
    $expectedWidth = [int]$parts[0]
    $expectedHeight = [int]$parts[1]

    $bitmap = [System.Drawing.Bitmap]::new($Path)
    try {
        if ($bitmap.Width -ne $expectedWidth -or $bitmap.Height -ne $expectedHeight) {
            throw "browser capture dimensions mismatch for ${Path}: expected ${expectedWidth}x${expectedHeight}, got $($bitmap.Width)x$($bitmap.Height)"
        }

        $colors = [System.Collections.Generic.HashSet[string]]::new()
        $stepX = [Math]::Max(1, [int][Math]::Floor($bitmap.Width / 24))
        $stepY = [Math]::Max(1, [int][Math]::Floor($bitmap.Height / 24))
        for ($y = 0; $y -lt $bitmap.Height; $y += $stepY) {
            for ($x = 0; $x -lt $bitmap.Width; $x += $stepX) {
                $pixel = $bitmap.GetPixel($x, $y)
                [void]$colors.Add("$($pixel.R),$($pixel.G),$($pixel.B),$($pixel.A)")
            }
        }

        if ($colors.Count -lt 8) {
            throw "browser capture appears blank for ${Path}: sampled only $($colors.Count) distinct colors"
        }
    }
    finally {
        $bitmap.Dispose()
    }
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

    $viewports = @{
        Desktop = "1440,900"
        Tablet = "900,1100"
        Mobile = "390,844"
    }

    $captures = @(
        @{ Name = "dashboard-home-desktop"; Url = "$baseUrl/dashboard"; Size = $viewports.Desktop },
        @{ Name = "dashboard-leaders-desktop"; Url = "$baseUrl/dashboard?workspace=/leaders"; Size = $viewports.Desktop },
        @{ Name = "dashboard-goalies-desktop"; Url = "$baseUrl/dashboard?workspace=/goalies"; Size = $viewports.Desktop },
        @{ Name = "dashboard-poach-desktop"; Url = "$baseUrl/dashboard?workspace=/poach"; Size = $viewports.Desktop },
        @{ Name = "dashboard-favorites-tablet"; Url = "$baseUrl/dashboard?workspace=/favorites"; Size = $viewports.Tablet },
        @{ Name = "dashboard-watchlist-tablet"; Url = "$baseUrl/dashboard?workspace=/watchlist"; Size = $viewports.Tablet },
        @{ Name = "dashboard-schedule-tablet"; Url = "$baseUrl/dashboard?workspace=/schedule"; Size = $viewports.Tablet },
        @{ Name = "dashboard-fantasy-mobile"; Url = "$baseUrl/dashboard?workspace=/fantasy"; Size = $viewports.Mobile },
        @{ Name = "dashboard-team-season-mobile"; Url = "$baseUrl/dashboard?workspace=/team/EDM/season"; Size = $viewports.Mobile },
        @{ Name = "dashboard-player-mobile"; Url = "$baseUrl/dashboard?workspace=/player/8478402"; Size = $viewports.Mobile }
    )

    foreach ($capture in $captures) {
        $path = Join-Path $OutputDir "$($capture.Name).png"
        Write-Host "capture: $($capture.Name) $($capture.Size)" -ForegroundColor Cyan
        Assert-DashboardReady -Url $capture.Url
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
        Assert-Screenshot -Path $path -Size $capture.Size
    }

    Write-Host "web dashboard captures written to $OutputDir" -ForegroundColor Green
}
finally {
    if (-not $serve.HasExited) {
        Stop-Process -Id $serve.Id -Force
        $serve.WaitForExit()
    }
}
