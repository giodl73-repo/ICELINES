param(
    [ValidateSet(
        "list",
        "quick",
        "ci",
        "ci-core",
        "ci-fetch",
        "ci-all",
        "ci-clippy",
        "ci-fmt",
        "ci-release",
        "ci-system",
        "tui-snapshots",
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
  powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 <slice>
  powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 viewmodel -NoCapture
  powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-all -NoCapture

Fast daily slices:
  quick            cargo check --workspace + ViewModel tests
  workspace-check  compile every crate without running tests
  viewmodel        Campbell ViewModel contract/builder tests
  cli-matrix       Foster capability matrix regression tests
  tui-snapshots    app snapshot tests from the icelines binary target

CI gates:
  ci               local CI sequence: core, fetch, all, clippy, fmt, release, system
  ci-core          CI step "L0 - Unit tests (icelines-core)"
  ci-fetch         CI step "L1 - Integration tests (icelines-fetch)"
  ci-all           CI step "All tests"
  ci-clippy        CI step "Clippy (zero warnings)"
  ci-fmt           CI step "Format check"
  ci-release       CI step "Build release binary"
  ci-system        CI step "L2 - System tests"

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
    "ci" {
        Invoke-Test @("test", "-p", "icelines-core", "--lib")
        Invoke-Test @("test", "-p", "icelines-fetch")
        Invoke-Test @("test")
        Invoke-Cargo @("clippy", "--", "-D", "warnings")
        Invoke-Cargo @("fmt", "--check")
        Invoke-Cargo @("build", "--release", "-p", "icelines-cli")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "system_tests")
    }
    "ci-core" {
        Invoke-Test @("test", "-p", "icelines-core", "--lib")
    }
    "ci-fetch" {
        Invoke-Test @("test", "-p", "icelines-fetch")
    }
    "ci-all" {
        Invoke-Test @("test")
    }
    "ci-clippy" {
        Invoke-Cargo @("clippy", "--", "-D", "warnings")
    }
    "ci-fmt" {
        Invoke-Cargo @("fmt", "--check")
    }
    "ci-release" {
        Invoke-Cargo @("build", "--release", "-p", "icelines-cli")
    }
    "ci-system" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "system_tests")
    }
    "tui-snapshots" {
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "tui::screens::app_snapshot_tests")
    }
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
