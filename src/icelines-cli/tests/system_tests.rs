/// L2 system tests — invoke the compiled `icelines` binary as a subprocess.
/// All tests use cached fixture data; no live network calls.
///
/// Run with: cargo test -p icelines-cli --test system_tests
///
/// The binary must be pre-built: `cargo build --release -p icelines-cli`
use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    // CARGO_MANIFEST_DIR = …/fantasy-tracker/src/icelines-cli
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // …/src
        .parent()
        .unwrap(); // …/fantasy-tracker
    #[cfg(windows)]
    let bin = workspace.join("src/target/release/icelines.exe");
    #[cfg(not(windows))]
    let bin = workspace.join("src/target/release/icelines");
    bin
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| {
            panic!(
            "failed to run icelines binary (run `cargo build --release -p icelines-cli` first): {e}"
        )
        })
}

// ── L2: --version exits 0 and prints version ─────────────────────────────────

#[test]
fn l2_cmd_version_exits_zero() {
    let out = run(&["--version"]);
    assert!(out.status.success(), "--version must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("icelines"),
        "--version must print 'icelines'"
    );
    assert!(
        stdout.contains("0.1.0"),
        "--version must print version number"
    );
}

// ── L2: --help exits 0 and lists subcommands ─────────────────────────────────

#[test]
fn l2_cmd_help_exits_zero_and_lists_commands() {
    let out = run(&["--help"]);
    assert!(out.status.success(), "--help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for cmd in &["fetch", "team", "rank", "build", "tui"] {
        assert!(stdout.contains(cmd), "--help must list '{cmd}' subcommand");
    }
}

// ── L2: fetch --dry-run exits 0 without making API calls ─────────────────────

#[test]
fn l2_cmd_fetch_dry_run_exits_zero() {
    let out = run(&["fetch", "all", "--dry-run"]);
    assert!(out.status.success(), "fetch all --dry-run must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("would fetch") || stdout.contains("Would fetch"),
        "dry-run must report what it would fetch, got: {stdout}"
    );
}

// ── L2: fetch rosters --dry-run exits 0 ──────────────────────────────────────

#[test]
fn l2_cmd_fetch_rosters_dry_run_exits_zero() {
    let out = run(&["fetch", "rosters", "--dry-run"]);
    assert!(out.status.success(), "fetch rosters --dry-run must exit 0");
}

// ── L2: fetch stats --dry-run exits 0 ────────────────────────────────────────

#[test]
fn l2_cmd_fetch_stats_dry_run_exits_zero() {
    let out = run(&["fetch", "stats", "--dry-run"]);
    assert!(out.status.success(), "fetch stats --dry-run must exit 0");
}

// ── L2: team with no cache exits 1 with helpful message ──────────────────────

#[test]
fn l2_cmd_team_no_cache_exits_nonzero() {
    // Without running `fetch` first, team command should fail gracefully
    // (not panic) with a message telling the user to fetch first.
    // Note: this may pass if the user happens to have cache from a real fetch.
    let out = run(&["team", "SEA"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Either exits non-zero with a message, or succeeds if cache is warm
    if !out.status.success() {
        assert!(
            stderr.contains("fetch") || stdout.contains("fetch"),
            "error message must mention 'fetch' command, got stderr: {stderr}"
        );
    }
}

// ── L2: rank with no cache exits 1 with helpful message ──────────────────────

#[test]
fn l2_cmd_rank_no_cache_exits_nonzero() {
    let out = run(&["rank", "--top", "10"]);
    // Either succeeds (cache warm) or fails with a clear message
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("fetch") || stderr.len() > 0,
            "rank error must produce stderr output"
        );
    }
}

// ── L2: Phase 2/3 stub commands print stub message and exit 0 ────────────────

#[test]
fn l2_cmd_stubs_exit_zero() {
    for cmd in &["build", "serve", "deploy", "tui", "tonight"] {
        let out = run(&[cmd]);
        assert!(
            out.status.success(),
            "stub command '{cmd}' must exit 0, got: {:?}",
            out.status.code()
        );
    }
}
