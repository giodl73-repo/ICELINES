//! Phase Lady Byng (LB.5) — L2 dispatch smokes for `icelines tui --start`.
//!
//! Each test invokes the compiled binary with a known-bad argument and
//! asserts:
//! 1. Exit is non-zero (resolution failures must NOT silently land in
//!    the alt-screen — the user has to see the error).
//! 2. The error message is printed to normal stderr (NOT inside the
//!    raw-mode TUI), so the test can grep for it directly.
//!
//! The render-correctness smokes for the happy path live as L0 in
//! `tui/screens/mod.rs::app_snapshot_tests::lb_smoke_*`.
//!
//! Run with: cargo test -p icelines-cli --test system_tui_experiences

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    #[cfg(windows)]
    let bin = workspace.join("target/release/icelines.exe");
    #[cfg(not(windows))]
    let bin = workspace.join("target/release/icelines");
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

/// LB.5 / lb_l2_unknown_slug_exits_nonzero
/// — `tui --start zzzz` must exit non-zero with stderr listing valid
///   slugs. Failure means resolution slipped past the parse-time
///   guard and might have entered the alt-screen.
#[test]
fn lb_l2_unknown_slug_exits_nonzero() {
    let out = run(&["tui", "--start", "zzzz"]);
    assert!(
        !out.status.success(),
        "expected non-zero exit; got success. stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown surface")
            && stderr.contains("Valid:")
            && stderr.contains("goalies"),
        "stderr missing expected error chrome, got:\n{stderr}"
    );
}

/// LB.5 / lb_l2_typo_slug_includes_suggestion
/// — `tui --start goalie` (singular) must surface the
///   `did you mean 'goalies'?` hint produced by the Levenshtein-1
///   suggestion path.
#[test]
fn lb_l2_typo_slug_includes_suggestion() {
    let out = run(&["tui", "--start", "goalie"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("did you mean 'goalies'?"),
        "stderr missing typo suggestion, got:\n{stderr}"
    );
}

/// LB.5 / lb_l2_unknown_team_lists_valid_abbrevs
/// — `tui --start team:ZZZ` must list all 32 valid abbrevs.
#[test]
fn lb_l2_unknown_team_lists_valid_abbrevs() {
    let out = run(&["tui", "--start", "team:ZZZ"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown team abbreviation")
            && stderr.contains("EDM")
            && stderr.contains("TOR"),
        "stderr missing team-abbrev list, got:\n{stderr}"
    );
}

/// LB.5 / lb_l2_empty_parameterized_arg_rejected
/// — `tui --start "player:"` must hit the parse-time guard, not slip
///   through to a normalize_name="" lookup.
#[test]
fn lb_l2_empty_parameterized_arg_rejected() {
    let out = run(&["tui", "--start", "player:"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("requires an argument") && stderr.contains("player:Bedard"),
        "stderr missing empty-arg guidance, got:\n{stderr}"
    );
}

/// LB.5 / lb_l2_ambiguous_player_lists_candidates
/// — `tui player Smith` must list multiple candidates with team +
///   season + role. Sebastian Aho problem.
#[test]
fn lb_l2_ambiguous_player_lists_candidates() {
    let out = run(&["tui", "player", "Smith"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Post-LB review fix #6 changed `pick one:` to `pick one (showing
    // N of M):` so ambiguous-candidate output stays within ~15 lines
    // even with 37 Smiths across 38 bundled seasons. Match the prefix
    // both forms share.
    assert!(
        stderr.contains("ambiguous name 'Smith'") && stderr.contains("pick one"),
        "stderr missing ambiguity listing, got:\n{stderr}"
    );
    // At least 3 distinct pids should be listed for "Smith" across
    // 38 bundled seasons.
    let pid_lines = stderr.matches("player:").count();
    assert!(
        pid_lines >= 3,
        "expected ≥3 candidate pids in ambiguity listing, got {pid_lines}:\n{stderr}"
    );
}

// pid-form bypass coverage lives at L0 in start_slug.rs
// (`l0_pid_resolution_passes_through`). An L2 version would
// successfully launch the TUI on a non-TTY subprocess and hang
// indefinitely (the event loop spins on poll-with-no-events), so the
// L0 unit test is the right home for that contract.

/// LB.5 / lb_l2_non_tty_menu_exits_clean
/// — `icelines menu < /dev/null` exits 0 with the redirect message.
///   When stdin isn't a terminal the menu must NOT block on
///   `read_line`.
#[test]
fn lb_l2_non_tty_menu_exits_clean() {
    use std::io::Write as _;
    let mut child = Command::new(icelines_bin())
        .arg("menu")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn icelines menu");
    // Close stdin immediately — no TTY, EOF.
    drop(child.stdin.take().unwrap().flush());
    drop(child.stdin.take()); // already taken above; this is fine
    let out = child.wait_with_output().expect("wait for menu");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "non-TTY menu must exit 0; got {:?}\nstderr:\n{stderr}",
        out.status
    );
    assert!(
        stderr.contains("interactive terminal"),
        "missing redirect message, got:\n{stderr}"
    );
}
