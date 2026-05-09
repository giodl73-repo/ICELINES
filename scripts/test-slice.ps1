param(
    [ValidateSet(
        "list",
        "quick",
        "workspace-check",
        "viewmodel",
        "core",
        "cli-matrix",
        "cli",
        "query",
        "fetch",
        "web",
        "site",
        "workspace",
        "full"
    )]
    [string]$Slice = "list",

    [switch]$NoCapture
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Cargo {
    param([string[]]$CargoArgs)

    Write-Host ""
    Write-Host "cargo $($CargoArgs -join ' ')" -ForegroundColor Cyan
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

function Invoke-Test {
    param([string[]]$CargoArgs)

    if ($NoCapture) {
        Invoke-Cargo ($CargoArgs + @("--", "--nocapture"))
    } else {
        Invoke-Cargo $CargoArgs
    }
}

if ($Slice -eq "list") {
    @"
IceLines test slices

Usage:
  pwsh scripts/test-slice.ps1 <slice>
  pwsh scripts/test-slice.ps1 viewmodel -NoCapture

Fast daily slices:
  quick            cargo check --workspace + ViewModel tests
  workspace-check  compile every crate without running tests
  viewmodel        Campbell ViewModel contract/builder tests
  cli-matrix       Foster capability matrix regression tests

Crate slices:
  core             icelines-core tests
  cli              icelines-cli tests
  query            icelines-query tests
  fetch            icelines-fetch tests
  web              icelines-web tests
  site             icelines-site tests

Long gates:
  workspace        cargo test --workspace
  full             cargo test --workspace --no-fail-fast
"@ | Write-Host
    exit 0
}

switch ($Slice) {
    "quick" {
        Invoke-Cargo @("check", "--workspace")
        Invoke-Test @("test", "-p", "icelines-core", "view_model")
    }
    "workspace-check" {
        Invoke-Cargo @("check", "--workspace")
    }
    "viewmodel" {
        Invoke-Test @("test", "-p", "icelines-core", "view_model")
    }
    "core" {
        Invoke-Test @("test", "-p", "icelines-core")
    }
    "cli-matrix" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "foster_capability_matrix")
    }
    "cli" {
        Invoke-Test @("test", "-p", "icelines-cli")
    }
    "query" {
        Invoke-Test @("test", "-p", "icelines-query")
    }
    "fetch" {
        Invoke-Test @("test", "-p", "icelines-fetch")
    }
    "web" {
        Invoke-Test @("test", "-p", "icelines-web")
    }
    "site" {
        Invoke-Test @("test", "-p", "icelines-site")
    }
    "workspace" {
        Invoke-Test @("test", "--workspace")
    }
    "full" {
        Invoke-Test @("test", "--workspace", "--no-fail-fast")
    }
}
