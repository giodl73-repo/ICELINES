param(
    [string]$BinaryPath = "",
    [int]$ServePort = 18990,
    [string]$OutputDir = "",
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $BinaryPath = Join-Path $RepoRoot "target\release\icelines.exe"
}
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $RepoRoot "dist\window-browser-review"
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
    throw "No supported headless Edge or Chrome installation was found."
}

function Assert-Html {
    param(
        [string]$Url,
        [string[]]$Required,
        [string[]]$Forbidden = @()
    )
    $response = Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 15
    if ($response.StatusCode -ne 200) {
        throw "Window browser review expected HTTP 200 for $Url; found $($response.StatusCode)"
    }
    foreach ($marker in $Required) {
        if (-not $response.Content.Contains($marker)) {
            throw "Window browser review missing '$marker' at $Url"
        }
    }
    foreach ($marker in $Forbidden) {
        if ($response.Content.Contains($marker)) {
            throw "Window browser review unexpectedly found '$marker' at $Url"
        }
    }
}

function Assert-Screenshot {
    param(
        [string]$Path,
        [int]$Width,
        [int]$Height
    )
    if (-not (Test-Path $Path)) {
        throw "Browser did not create $Path"
    }
    $bitmap = [System.Drawing.Bitmap]::new($Path)
    try {
        if ($bitmap.Width -ne $Width -or $bitmap.Height -ne $Height) {
            throw "Screenshot dimensions for $Path are $($bitmap.Width)x$($bitmap.Height), expected ${Width}x${Height}"
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
            throw "Screenshot $Path appears blank; sampled only $($colors.Count) colors"
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
$stdoutPath = Join-Path $env:TEMP "icelines-window-browser-stdout.txt"
$stderrPath = Join-Path $env:TEMP "icelines-window-browser-stderr.txt"
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
            throw "Serve exited early with code $($serve.ExitCode)`nSTDOUT:`n$stdout`nSTDERR:`n$stderr"
        }
        try {
            Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/window/balanced.v1/20262027" -TimeoutSec 2 | Out-Null
            $ready = $true
            break
        }
        catch {
            Start-Sleep -Milliseconds 500
        }
    }
    if (-not $ready) {
        throw "Timed out waiting for the Window web route"
    }

    $boardUrl = "$baseUrl/window/balanced.v1/20262027"
    $focusedUrl = "$boardUrl`?team=NYR"
    $cardUrl = "$baseUrl/icecast/20262027/NYR/window"
    $insiderUrl = "$cardUrl`?page=insider"
    Assert-Html -Url $boardUrl -Required @(
        '<html lang="en">',
        'name="viewport"',
        'class="skip-link"',
        '<main id="main" tabindex="-1">',
        '<caption>',
        'aria-label="Organization Window standings"',
        '<th scope="col">Team</th>',
        '<th scope="row">',
        'NR means a declared comparability gate withheld rank'
    )
    Assert-Html -Url $focusedUrl -Required @('The Window', '>NYR</a>') -Forbidden @('>SEA</a>')
    Assert-Html -Url $cardUrl -Required @(
        '<html lang="en">',
        'class="skip-link"',
        '<main id="main"',
        'New York Rangers organization Window'
    )
    Assert-Html -Url $insiderUrl -Required @('The Insider', 'Methodology')

    $captures = @(
        @{ Name = "window-board-desktop"; Url = $boardUrl; Width = 1440; Height = 900 },
        @{ Name = "window-board-mobile"; Url = $boardUrl; Width = 390; Height = 844 },
        @{ Name = "window-nyr-focused-tablet"; Url = $focusedUrl; Width = 900; Height = 1000 },
        @{ Name = "window-nyr-card-desktop"; Url = $cardUrl; Width = 1440; Height = 900 },
        @{ Name = "window-nyr-insider-mobile"; Url = $insiderUrl; Width = 390; Height = 844 }
    )
    foreach ($capture in $captures) {
        $path = Join-Path $OutputDir "$($capture.Name).png"
        Write-Host "capture: $($capture.Name) $($capture.Width)x$($capture.Height)" -ForegroundColor Cyan
        & $Browser `
            "--headless=new" `
            "--disable-gpu" `
            "--disable-background-networking" `
            "--disable-component-update" `
            "--disable-sync" `
            "--no-first-run" `
            "--hide-scrollbars" `
            "--window-size=$($capture.Width),$($capture.Height)" `
            "--screenshot=$path" `
            $capture.Url | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Browser capture failed for $($capture.Url)"
        }
        Assert-Screenshot -Path $path -Width $capture.Width -Height $capture.Height
    }

    Write-Host "Window browser review passed; captures: $OutputDir" -ForegroundColor Green
}
finally {
    if (-not $serve.HasExited) {
        Stop-Process -Id $serve.Id -Force
        $serve.WaitForExit()
    }
}
