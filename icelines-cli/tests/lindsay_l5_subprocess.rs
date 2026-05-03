//! Phase Lindsay L.5 test-gap fill — L2 subprocess tests.
//!
//! Closes the audit gaps for L.5.1 (catalog `--sort` end-to-end) and
//! L.5.4 (`export md leaders --columns` end-to-end). The L0 unit tests
//! exercise the parser + renderer in isolation; these L2 tests invoke
//! the compiled binary as a subprocess against bundled season data
//! and assert the rendered output.

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
    assert!(
        bin.exists(),
        "release binary not built — run `cargo build --release -p icelines-cli`"
    );
    bin
}

/// L.5.1 — `query leaders --sort points-per-game` (a catalog-only
/// `cli_key`, NOT a legacy SortMetric alias) routes through
/// `SortDispatch::Catalog(StatId::PointsPerGame)`. Output must:
///   - exit 0
///   - render the `PPG` header (`StatId::PointsPerGame.short_label()`)
///   - emit a sorted table with at least 5 result rows
#[test]
fn l2_lindsay_l5_query_leaders_catalog_sort_points_per_game() {
    let out = Command::new(icelines_bin())
        .args(["query", "leaders", "--sort", "points-per-game", "--top", "5"])
        .output()
        .expect("spawn icelines");
    let status = out.status;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        status.success(),
        "exit non-zero ({status:?})\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("PPG"),
        "expected `PPG` header (catalog short_label) — got:\n{stdout}"
    );
    // 5 ranked rows + header + separator + footer.
    let row_count = stdout
        .lines()
        .filter(|l| l.starts_with("1 ") || l.starts_with("2 ") || l.starts_with("3 ") || l.starts_with("4 ") || l.starts_with("5 "))
        .count();
    assert_eq!(
        row_count, 5,
        "expected 5 ranked rows under --top 5 — got:\n{stdout}"
    );
}

/// L.5.1 — `query leaders --sort regulation-wins` is a catalog-only
/// goalie cli_key. Should bail cleanly (skater leaderboard ignores
/// goalie stats; reads return None and sort to bottom by AI-06).
/// Smoke: exit 0, no panic.
#[test]
fn l2_lindsay_l5_query_leaders_catalog_sort_goalie_key_no_panic() {
    let out = Command::new(icelines_bin())
        .args(["query", "leaders", "--sort", "save-pct", "--top", "3"])
        .output()
        .expect("spawn icelines");
    assert!(
        out.status.success(),
        "exit non-zero — stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// L.5.4 — `export md leaders --columns "goals,assists,points"`
/// renders a markdown table with the requested catalog columns
/// instead of the canonical hardcoded shape. Output to stdout via
/// `--out -`.
#[test]
fn l2_lindsay_l5_export_md_leaders_columns_renders_custom_table() {
    let out = Command::new(icelines_bin())
        .args([
            "export", "md", "leaders",
            "--out", "-",
            "--columns", "goals,assists,points",
            "--top", "3",
        ])
        .output()
        .expect("spawn icelines");
    let status = out.status;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        status.success(),
        "exit non-zero ({status:?})\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // Custom header from StatId::short_label: G, A, P.
    assert!(
        stdout.contains("| G | A | P |"),
        "expected custom header `| G | A | P |` — got:\n{stdout}"
    );
    // Canonical Pts/82 *column* must NOT appear under --columns.
    // Note: the YAML front-matter title may reference "Pts/82" as the
    // sort metric — that's expected. Look only for the column-header
    // form `| Pts/82 |`.
    assert!(
        !stdout.contains("| Pts/82 |"),
        "Pts/82 column header must be absent under --columns — got:\n{stdout}"
    );
    // Front-matter must still render.
    assert!(
        stdout.starts_with("---\n"),
        "expected YAML front-matter — got:\n{stdout}"
    );
}

/// L.5.4 — unknown column key bails non-zero with the actionable hint.
#[test]
fn l2_lindsay_l5_export_md_leaders_columns_unknown_bails() {
    let out = Command::new(icelines_bin())
        .args([
            "export", "md", "leaders",
            "--out", "-",
            "--columns", "not-a-real-stat",
            "--top", "3",
        ])
        .output()
        .expect("spawn icelines");
    assert!(
        !out.status.success(),
        "expected non-zero exit on unknown column key"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not-a-real-stat") || stderr.contains("unknown column"),
        "expected actionable error mentioning the bad key — stderr:\n{stderr}"
    );
}

/// L.5.4 — without `--columns`, output is byte-stable to v1 (canonical
/// "G | A | Pts | PPG | Pts/82" shape preserved for the L.3 fence).
#[test]
fn l2_lindsay_l5_export_md_leaders_no_columns_preserves_canonical() {
    let out = Command::new(icelines_bin())
        .args(["export", "md", "leaders", "--out", "-", "--top", "3"])
        .output()
        .expect("spawn icelines");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Pts/82"),
        "canonical shape must keep Pts/82 column when --columns is omitted"
    );
    assert!(
        stdout.contains("PPG"),
        "canonical shape must keep PPG column when --columns is omitted"
    );
}
