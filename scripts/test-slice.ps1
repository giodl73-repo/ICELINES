param(
    [ValidateSet(
        "list",
        "quick",
        "ci",
        "ci-core",
        "ci-core-integration",
        "ci-query",
        "ci-fetch",
        "ci-web",
        "ci-site",
        "ci-all",
        "ci-cli-lib",
        "ci-cli-tui",
        "ci-cli-art-ross",
        "ci-cli-lindsay",
        "ci-cli-persona",
        "ci-cli-smoke",
        "ci-clippy",
        "ci-fmt",
        "ci-release",
        "ci-system",
        "ci-docs",
        "tui-snapshots",
        "scenarios",
        "scenarios-cli",
        "scenarios-query",
        "scenarios-web",
        "scenarios-tui",
        "web-captures",
        "selke",
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
  scenarios        active scenario harnesses by surface: TUI, CLI, query, web
  scenarios-cli    CLI persona/scenario harnesses
  scenarios-query  query persona/storyline harnesses
  scenarios-web    web persona/parity harnesses
  scenarios-tui    in-bin TUI persona/user-flow harness
  web-captures     headless Edge/Chrome dashboard screenshots into dist/
  selke            fantasy poacher ViewModel + CLI poach/watch/report tests

CI gates:
  ci               local CI sequence: all split gates, serial
  ci-core          Tests / core-lib
  ci-core-integration Tests / core-integration
  ci-query         Tests / query
  ci-fetch         Tests / fetch
  ci-web           Tests / web
  ci-site          Tests / site
  ci-cli-lib       Tests / cli-lib
  ci-cli-tui       Tests / cli-tui-bin
  ci-cli-art-ross  Tests / cli-art-ross
  ci-cli-lindsay   Tests / cli-lindsay
  ci-cli-persona   Tests / cli-persona
  ci-cli-smoke     Tests / cli-smoke
  ci-system        Tests / cli-system
  ci-docs          Tests / doc-tests
  ci-clippy        Quality / clippy
  ci-fmt           Quality / fmt
  ci-release       Build / release CLI + release smoke
  ci-all           legacy monolithic cargo test

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
        Invoke-Test @("test", "-p", "icelines-core", "--tests")
        Invoke-Test @("test", "-p", "icelines-query")
        Invoke-Test @("test", "-p", "icelines-fetch")
        Invoke-Test @("test", "-p", "icelines-web")
        Invoke-Test @("test", "-p", "icelines-site")
        Invoke-Test @("test", "-p", "icelines-cli", "--lib")
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "art_ross_a0_parity", "--test", "art_ross_a2_executor", "--test", "art_ross_a5_explain", "--test", "art_ross_w14_semantic", "--test", "art_ross_w14b_goalies", "--test", "art_ross_w15_trades", "--test", "art_ross_w23_tui_filter", "--test", "art_ross_w25_career_filter")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "lindsay_l3_golden", "--test", "lindsay_l5_subprocess")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "persona_foster", "--test", "persona_masterton_standalone", "--test", "persona_scenarios", "--test", "persona_wave2", "--test", "persona_wave3", "--test", "persona_wave4", "--test", "persona_wave5", "--test", "persona_wave6", "--test", "persona_wave7", "--test", "persona_wave9", "--test", "persona_wave10", "--test", "persona_wave11", "--test", "persona_wave16", "--test", "persona_wave18", "--test", "persona_wave20", "--test", "persona_wave23", "--test", "persona_wave25")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "system_tests", "--test", "system_tui_experiences")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "foster_capability_matrix", "--test", "proof_lib_smoke")
        Invoke-Test @("test", "--workspace", "--doc")
        Invoke-Cargo @("clippy", "--", "-D", "warnings")
        Invoke-Cargo @("fmt", "--check")
        Invoke-Cargo @("build", "--release", "-p", "icelines-cli")
    }
    "ci-core" {
        Invoke-Test @("test", "-p", "icelines-core", "--lib")
    }
    "ci-core-integration" {
        Invoke-Test @("test", "-p", "icelines-core", "--tests")
    }
    "ci-query" {
        Invoke-Test @("test", "-p", "icelines-query")
    }
    "ci-fetch" {
        Invoke-Test @("test", "-p", "icelines-fetch")
    }
    "ci-web" {
        Invoke-Test @("test", "-p", "icelines-web")
    }
    "ci-site" {
        Invoke-Test @("test", "-p", "icelines-site")
    }
    "ci-all" {
        Invoke-Test @("test")
    }
    "ci-cli-lib" {
        Invoke-Test @("test", "-p", "icelines-cli", "--lib")
    }
    "ci-cli-tui" {
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines")
    }
    "ci-cli-art-ross" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "art_ross_a0_parity", "--test", "art_ross_a2_executor", "--test", "art_ross_a5_explain", "--test", "art_ross_w14_semantic", "--test", "art_ross_w14b_goalies", "--test", "art_ross_w15_trades", "--test", "art_ross_w23_tui_filter", "--test", "art_ross_w25_career_filter")
    }
    "ci-cli-lindsay" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "lindsay_l3_golden", "--test", "lindsay_l5_subprocess")
    }
    "ci-cli-persona" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "persona_foster", "--test", "persona_masterton_standalone", "--test", "persona_scenarios", "--test", "persona_wave2", "--test", "persona_wave3", "--test", "persona_wave4", "--test", "persona_wave5", "--test", "persona_wave6", "--test", "persona_wave7", "--test", "persona_wave9", "--test", "persona_wave10", "--test", "persona_wave11", "--test", "persona_wave16", "--test", "persona_wave18", "--test", "persona_wave20", "--test", "persona_wave23", "--test", "persona_wave25")
    }
    "ci-cli-smoke" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "foster_capability_matrix", "--test", "proof_lib_smoke")
    }
    "ci-clippy" {
        Invoke-Cargo @("clippy", "--", "-D", "warnings")
    }
    "ci-fmt" {
        Invoke-Cargo @("fmt", "--check")
    }
    "ci-release" {
        Write-Host ""
        Write-Host "powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1" -ForegroundColor Cyan
        & powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    "ci-system" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "system_tests", "--test", "system_tui_experiences")
    }
    "ci-docs" {
        Invoke-Test @("test", "--workspace", "--doc")
    }
    "tui-snapshots" {
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "tui::screens::app_snapshot_tests")
    }
    "scenarios" {
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "persona_jack_adams")
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "persona_foster", "--test", "persona_masterton_standalone", "--test", "persona_scenarios", "--test", "persona_wave2", "--test", "persona_wave3", "--test", "persona_wave4", "--test", "persona_wave5", "--test", "persona_wave6", "--test", "persona_wave7", "--test", "persona_wave9", "--test", "persona_wave10", "--test", "persona_wave11", "--test", "persona_wave16", "--test", "persona_wave18", "--test", "persona_wave20", "--test", "persona_wave23", "--test", "persona_wave25")
        Invoke-Test @("test", "-p", "icelines-query", "--test", "persona_wave12", "--test", "persona_wave13")
        Invoke-Test @("test", "-p", "icelines-web", "--test", "persona_wave8", "--test", "persona_wave17", "--test", "persona_wave19", "--test", "persona_wave21_parity", "--test", "persona_wave22b_envelope")
    }
    "scenarios-cli" {
        Invoke-Test @("test", "-p", "icelines-cli", "--test", "persona_foster", "--test", "persona_masterton_standalone", "--test", "persona_scenarios", "--test", "persona_wave2", "--test", "persona_wave3", "--test", "persona_wave4", "--test", "persona_wave5", "--test", "persona_wave6", "--test", "persona_wave7", "--test", "persona_wave9", "--test", "persona_wave10", "--test", "persona_wave11", "--test", "persona_wave16", "--test", "persona_wave18", "--test", "persona_wave20", "--test", "persona_wave23", "--test", "persona_wave25")
    }
    "scenarios-query" {
        Invoke-Test @("test", "-p", "icelines-query", "--test", "persona_wave12", "--test", "persona_wave13")
    }
    "scenarios-web" {
        Invoke-Test @("test", "-p", "icelines-web", "--test", "persona_wave8", "--test", "persona_wave17", "--test", "persona_wave19", "--test", "persona_wave21_parity", "--test", "persona_wave22b_envelope")
    }
    "scenarios-tui" {
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "persona_jack_adams")
    }
    "web-captures" {
        Write-Host ""
        Write-Host "powershell -ExecutionPolicy Bypass -File scripts/web-dashboard-capture.ps1" -ForegroundColor Cyan
        & powershell -ExecutionPolicy Bypass -File scripts/web-dashboard-capture.ps1
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    "selke" {
        Invoke-Test @("test", "-p", "icelines-core", "view_model::poach")
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "l0_poach")
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "l0_poach_tui")
        Invoke-Test @("test", "-p", "icelines-cli", "--bin", "icelines", "l0_watch")
        Invoke-Test @("test", "-p", "icelines-web", "l1_poach")
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
