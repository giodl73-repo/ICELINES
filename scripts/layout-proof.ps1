param(
    [string]$LayoutName = "rangers-stats",
    [switch]$UseInstalled
)

$ErrorActionPreference = "Stop"

$tempHome = Join-Path ([System.IO.Path]::GetTempPath()) ("icelines-rangers-layout-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tempHome | Out-Null

function Invoke-Icelines {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Args,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $debugBinary = Join-Path (Get-Location) "target\debug\icelines.exe"
    if ($UseInstalled) {
        $cmd = "icelines"
        $fullArgs = @("--no-setup", "--no-live") + $Args
    } elseif (Test-Path $debugBinary) {
        $cmd = $debugBinary
        $fullArgs = @("--no-setup", "--no-live") + $Args
    } else {
        $cmd = "cargo"
        $fullArgs = @(
            "run", "-q", "-p", "icelines-cli", "--bin", "icelines",
            "--", "--no-setup", "--no-live"
        ) + $Args
    }

    Write-Host "== $Name =="
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $cmd @fullArgs 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    $text = ($output | Out-String)
    if ($exitCode -ne 0) {
        Write-Host $text
        throw "$Name failed with exit code $exitCode"
    }
    return $text
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Text,
        [Parameter(Mandatory = $true)]
        [string]$Needle,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    if (-not $Text.Contains($Needle)) {
        throw "$Name did not contain expected text: $Needle"
    }
}

try {
    $env:HOME = $tempHome
    $env:USERPROFILE = $tempHome
    $env:ICELINES_NO_LIVE = "1"
    $env:ICELINES_TEST_MODE = "1"

    $save = Invoke-Icelines -Name "layout-save" -Args @(
        "layout", "save", $LayoutName,
        "--center", "stats",
        "--left", "favorites-left",
        "--right", "schedule-right"
    )
    Assert-Contains $save "saved layout $LayoutName" "layout-save"

    $list = Invoke-Icelines -Name "layout-list" -Args @("layout", "list")
    Assert-Contains $list $LayoutName "layout-list"
    Assert-Contains $list "center=stats" "layout-list"
    Assert-Contains $list "left=favorites-left" "layout-list"
    Assert-Contains $list "right=schedule-right" "layout-list"

    $show = Invoke-Icelines -Name "layout-show" -Args @("layout", "show", $LayoutName)
    Assert-Contains $show '"center": "stats"' "layout-show"
    Assert-Contains $show '"left": "favorites-left"' "layout-show"
    Assert-Contains $show '"right": "schedule-right"' "layout-show"
    Assert-Contains $show '"active_context_policy": "preserve-active-context"' "layout-show"

    $delete = Invoke-Icelines -Name "layout-delete" -Args @("layout", "delete", $LayoutName)
    Assert-Contains $delete "deleted layout $LayoutName" "layout-delete"

    Write-Host "Rangers layout proof passed with isolated home $tempHome."
}
finally {
    Remove-Item -Recurse -Force $tempHome -ErrorAction SilentlyContinue
}
