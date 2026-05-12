//! Prince.5 CLI readability contracts.
//!
//! These subprocess checks keep representative text surfaces useful in an
//! 80-column terminal with color disabled. JSON and CSV contracts live in the
//! existing route-specific tests; this file focuses on human-readable output.

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn run_no_color(args: &[&str]) -> String {
    let out = Command::new(icelines_bin())
        .env("NO_COLOR", "1")
        .env("COLUMNS", "80")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run icelines binary: {e}"));
    assert!(
        out.status.success(),
        "command {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout should be utf-8")
}

fn assert_lines_fit_80(label: &str, output: &str) {
    for (idx, line) in output.lines().enumerate() {
        let width = line.chars().count();
        assert!(
            width <= 80,
            "{label} line {} exceeds 80 columns ({width}): {line:?}\n{output}",
            idx + 1
        );
    }
}

#[test]
fn prince_cli_leaders_no_color_fits_80_columns() {
    let output = run_no_color(&["query", "leaders", "--top", "3", "--season", "20242025"]);
    assert!(output.contains("Rank Player"));
    assert!(output.contains("Pts/82"));
    assert_lines_fit_80("leaders", &output);
}

#[test]
fn prince_cli_goalies_no_color_fits_80_columns() {
    let output = run_no_color(&["query", "goalies", "--top", "3", "--season", "20242025"]);
    assert!(output.contains("Rank Goalie"));
    assert!(output.contains("SV%"));
    assert_lines_fit_80("goalies", &output);
}

#[test]
fn prince_cli_poach_no_color_fits_80_columns_and_keeps_labels() {
    let output = run_no_color(&["poach", "--top", "3"]);
    assert!(output.contains("Rank Player"));
    assert!(output.contains("Why/Risk"));
    assert!(output.contains("Source state:"));
    assert_lines_fit_80("poach", &output);
}
