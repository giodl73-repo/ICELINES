param(
    [string]$BinaryPath = "",
    [int]$ServePort = 18987,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $BinaryPath = Join-Path $RepoRoot "target\release\icelines.exe"
}

if (-not $SkipBuild) {
    Write-Host "cargo build --release -p icelines-cli" -ForegroundColor Cyan
    & cargo build --release -p icelines-cli
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$Binary = (Resolve-Path $BinaryPath).Path

function Invoke-Smoke {
    param(
        [string]$Name,
        [string[]]$CommandArgs,
        [string[]]$MustContain = @()
    )

    Write-Host "smoke: $Name" -ForegroundColor Cyan
    $env:NO_COLOR = "1"
    $env:COLUMNS = "80"
    $out = & $Binary @CommandArgs 2>&1
    $code = $LASTEXITCODE
    $text = ($out | Out-String)
    if ($code -ne 0) {
        Write-Error "smoke '$Name' failed with exit code $code`n$text"
    }
    foreach ($needle in $MustContain) {
        if (-not $text.Contains($needle)) {
            Write-Error "smoke '$Name' missing expected text '$needle'`n$text"
        }
    }
}

Invoke-Smoke "version" @("--version") @("icelines")
Invoke-Smoke "help" @("--help") @("Commands:", "query", "tui", "serve")
Invoke-Smoke "leaders" @("query", "leaders", "--top", "3", "--season", "20242025") @("Rank Player", "Connor McDavid")
Invoke-Smoke "goalies" @("query", "goalies", "--top", "3", "--season", "20242025") @("Rank Goalie", "SV%")
Invoke-Smoke "tui help" @("tui", "--help") @("Launch the interactive", "goalies", "poach")
Invoke-Smoke "serve help" @("serve", "--help") @("web dashboard", "--no-open")
Invoke-Smoke "docs" @("docs") @("IceLines", "Command Reference")
Invoke-Smoke "markdown export" @("export", "md", "leaders", "--out", "-", "--top", "3") @("type: leaderboard", "| Rank | Player |")
Invoke-Smoke "poach" @("poach", "--top", "3") @("Rank Player", "Why/Risk", "Source state:")

Write-Host "smoke: serve url" -ForegroundColor Cyan
$stdoutPath = Join-Path $env:TEMP "icelines-release-smoke-stdout.txt"
$stderrPath = Join-Path $env:TEMP "icelines-release-smoke-stderr.txt"
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
    Start-Sleep -Seconds 3
    $stdout = if (Test-Path $stdoutPath) { Get-Content $stdoutPath -Raw } else { "" }
    $stderr = if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw } else { "" }
    if ($serve.HasExited) {
        Write-Error "serve smoke exited early with code $($serve.ExitCode)`nSTDOUT:`n$stdout`nSTDERR:`n$stderr"
    }
    $expectedUrl = "http://127.0.0.1:$ServePort/"
    if (-not $stdout.Contains($expectedUrl)) {
        Write-Error "serve smoke did not print expected URL $expectedUrl`nSTDOUT:`n$stdout`nSTDERR:`n$stderr"
    }
}
finally {
    if (-not $serve.HasExited) {
        Stop-Process -Id $serve.Id -Force
        $serve.WaitForExit()
    }
}

Write-Host "release smoke passed for $Binary" -ForegroundColor Green
