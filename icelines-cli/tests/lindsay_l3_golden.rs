//! Phase Lindsay L.3 — stdout-golden parity (BENCH-R2 L2-B23).
//!
//! Captures stdout for every legacy `--sort` value of `query leaders`
//! against bundled season 20242025 and pins it against pre-L.3
//! goldens at `icelines-cli/tests/fixtures/lindsay_l3_pre/leaders-<sort>.golden.txt`.
//!
//! The L.3 spec routes `--sort` through `StatId::sort_cmp` for the
//! universal AI-06 tiebreak. This test fence catches:
//!   - silent ordering changes (the universal tiebreak should match
//!     the pre-Lindsay `(value, nhl_id asc)` tiebreak in every legacy
//!     metric — if it doesn't, that's an explicit decision needing
//!     a golden update),
//!   - format/display drift (column widths, separators, "X matched"
//!     footer wording),
//!   - the "improvement" special-case path through
//!     `compute_improvement_map`.
//!
//! Two-fence checkpoint: this same test runs again post-L.5 (BENCH-R2
//! L2-B23 second fence) when site/HTTP migration could re-touch
//! ordering paths.
//!
//! **Regenerate goldens** when an INTENDED ordering change ships:
//!     LINDSAY_L3_REGEN=1 cargo test --release -p icelines-cli \
//!         --test lindsay_l3_golden l2_lindsay_l3_golden_parity
//! Commit the diffs alongside the change that necessitated them.

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("lindsay_l3_pre")
}

/// Every legacy --sort value from `SortMetric::parse` (canonical hyphen-case
/// form per arm). 35 values total. Aliases (`pts_pace`, `ppg`, etc.) route
/// to the same arm — covered by separate L0 unit tests in `query.rs`, not
/// here.
const LEGACY_SORTS: &[&str] = &[
    // All-situations pace
    "pts-pace",
    "ppg",
    "g-pace",
    "gpg",
    // Raw season totals
    "pts",
    "goals",
    "assists",
    "gp",
    // Power play
    "pp-pts-pace",
    "pp-g-pace",
    "pp-pts",
    "pp-g",
    // Shorthanded
    "sh-g-pace",
    "sh-g",
    // GWG / OT
    "gwg-pace",
    "gwg",
    // Shots
    "shots-pace",
    "shots",
    "sh-pct",
    // Two-way / TOI / FO
    "plus-minus",
    "toi",
    "fo-pct",
    // Realtime physical
    "hits-pace",
    "hits",
    "blocks-pace",
    "blocks",
    "takeaways",
    "giveaways",
    "pim",
    // MoneyPuck
    "xg",
    "xg-per-60",
    "cf-pct",
    "ff-pct",
    "xgf-pct",
    // Trend
    "improvement",
];

/// Run `query leaders --sort <metric>` and return stdout (utf-8).
fn capture_leaders(sort: &str) -> String {
    let out = Command::new(icelines_bin())
        .args([
            "query", "leaders", "--sort", sort, "--top", "10", "--season", "20242025",
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to run icelines (`cargo build --release` first?): {e}"));
    // For sorts that exit non-zero (none expected today), capture the
    // combined output so a regression that flips success → failure
    // produces a comparable diff against the golden.
    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&out.stdout));
    if !out.status.success() {
        combined.push_str("\n--- STDERR ---\n");
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    combined
}

/// L.3 fence test (BENCH-R2 L2-B23). Captures stdout for every legacy
/// --sort value and asserts byte-equality against the L.3 goldens.
///
/// **Status: ACTIVE post-L.3.2** (un-ignored 2026-05-02). The legacy
/// `--sort` paths now apply the AI-06 universal `nhl_id asc` tiebreak,
/// so tied-value ordering is deterministic across process invocations.
/// Goldens were re-captured under deterministic sort and committed.
///
/// **What this fence catches**:
/// - Silent ordering changes during refactor (StatId migration, etc.)
/// - Format/display drift (column widths, separators, footer wording)
/// - The "improvement" special-case path through `compute_improvement_map`
///
/// Two-fence checkpoint: this same test runs again post-L.5 (BENCH-R2
/// L2-B23 second fence) when site/HTTP migration could re-touch
/// ordering paths.
///
/// **Regenerate goldens** when an INTENDED ordering change ships:
///     LINDSAY_L3_REGEN=1 cargo test --release -p icelines-cli \
///         --test lindsay_l3_golden l2_lindsay_l3_golden_parity
/// (No more `--include-ignored` needed — the test is active.)
/// Commit the diffs alongside the change that necessitated them.
#[test]
fn l2_lindsay_l3_golden_parity() {
    let regen = std::env::var("LINDSAY_L3_REGEN").is_ok();
    let dir = fixtures_dir();
    if regen {
        std::fs::create_dir_all(&dir).unwrap();
    }

    let mut failures = Vec::new();
    for sort in LEGACY_SORTS {
        let actual = capture_leaders(sort);
        let golden_path = dir.join(format!("leaders-{sort}.golden.txt"));

        if regen {
            std::fs::write(&golden_path, &actual).unwrap_or_else(|e| {
                panic!("failed to write golden {}: {e}", golden_path.display())
            });
            continue;
        }

        let golden = match std::fs::read_to_string(&golden_path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!(
                    "MISSING GOLDEN: --sort {sort} (path: {} — re-run with LINDSAY_L3_REGEN=1): {e}",
                    golden_path.display(),
                ));
                continue;
            }
        };

        if actual != golden {
            failures.push(format!(
                "DIVERGENCE: --sort {sort}\n\
                 ─── golden ({} bytes) ───\n{}\n\
                 ─── actual ({} bytes) ───\n{}\n",
                golden.len(),
                golden,
                actual.len(),
                actual,
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "L.3 stdout-golden parity FAILED for {}/{} sort metrics:\n\n{}",
            failures.len(),
            LEGACY_SORTS.len(),
            failures.join("\n\n"),
        );
    }
}
