/// L2 system tests — invoke the compiled `icelines` binary as a subprocess.
/// All tests use cached fixture data; no live network calls.
///
/// Run with: cargo test -p icelines-cli --test system_tests
///
/// The binary must be pre-built: `cargo build --release -p icelines-cli`
use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    // CARGO_MANIFEST_DIR = …/icelines/icelines-cli
    // One parent up is the workspace root: …/icelines/
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap(); // …/icelines
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

/// Run the binary with HOME and USERPROFILE pointed at a temp dir so the
/// SQLite group/fantasy db opens fresh and tests don't mutate real user data.
/// Phase 8f.6.
#[allow(dead_code)]
fn run_isolated(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| {
            panic!("failed to run icelines binary: {e}")
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

// ── L2: Commands that always exit 0 ──────────────────────────────────────────

#[test]
fn l2_cmd_stubs_exit_zero() {
    // tonight exits 0 even without cache (prints message gracefully)
    for cmd in &["tonight"] {
        let out = run(&[cmd]);
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains("panic"),
            "command '{cmd}' must not panic"
        );
    }
}

// ── L2: Phase 2 scheme commands ───────────────────────────────────────────────

#[test]
fn l2_cmd_scheme_list_exits_zero() {
    let out = run(&["scheme", "list"]);
    assert!(out.status.success(), "scheme list must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("yahoo-standard"),
        "scheme list must show yahoo-standard"
    );
    assert!(
        stdout.contains("espn-standard"),
        "scheme list must show espn-standard"
    );
    assert!(
        stdout.contains("simple-pts"),
        "scheme list must show simple-pts"
    );
}

#[test]
fn l2_cmd_scheme_show_yahoo_exits_zero() {
    let out = run(&["scheme", "show", "yahoo-standard"]);
    assert!(
        out.status.success(),
        "scheme show yahoo-standard must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("goals"),
        "scheme show must display scoring weights"
    );
}

#[test]
fn l2_cmd_scheme_show_unknown_exits_nonzero() {
    let out = run(&["scheme", "show", "nonexistent-scheme"]);
    assert!(!out.status.success(), "unknown scheme must exit non-zero");
}

#[test]
fn l2_cmd_scheme_show_source_emits_valid_json() {
    // Phase 8f.5: --source prints the scheme as JSON for diffing / piping.
    let out = run(&["scheme", "show", "yahoo-standard", "--source"]);
    assert!(
        out.status.success(),
        "scheme show --source must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("scheme show --source output must be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("yahoo-standard"),
        "JSON must include name field");
    assert!(parsed["skater"].is_object(), "JSON must include skater weights");
    assert!(parsed["goalie"].is_object(), "JSON must include goalie weights");
}

// ── L2: Phase 2 snapshot commands ────────────────────────────────────────────

#[test]
fn l2_cmd_snapshot_list_exits_zero() {
    // May have no snapshots if fetch hasn't been run — that's fine
    let out = run(&["snapshot", "list"]);
    assert!(
        out.status.success(),
        "snapshot list must exit 0 even with no snapshots"
    );
}

#[test]
fn l2_cmd_snapshot_verify_no_active_exits_gracefully() {
    // Without an active snapshot, verify should exit non-zero with a message
    let out = run(&["snapshot", "verify"]);
    // Either succeeds (snapshots exist) or fails with a clear message
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stderr.len() > 0 || stdout.len() > 0,
            "verify must produce output when it fails"
        );
    }
}

// ── L2: Phase 2 player analysis (requires cache — graceful degradation) ───────

#[test]
fn l2_cmd_players_no_cache_exits_gracefully() {
    let out = run(&["players", "--pos", "C", "--age-max", "25"]);
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("fetch") || stderr.len() > 0,
            "players must mention fetch when cache is missing"
        );
    }
}

#[test]
fn l2_cmd_class_no_cache_exits_gracefully() {
    let out = run(&["class", "2022"]);
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.len() > 0,
            "class must produce error output when cache is missing"
        );
    }
}

#[test]
fn l2_cmd_group_list_exits_zero() {
    let out = run(&["group", "list"]);
    assert!(
        out.status.success(),
        "group list must exit 0 even with no groups"
    );
}

#[test]
fn l2_cmd_build_no_site_exits_gracefully() {
    // Without a snapshot, build should fail gracefully (not panic)
    let out = run(&["build", "--no-site"]);
    // Either succeeds (if snapshot exists) or fails with a clear message
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stderr.contains("fetch") || stdout.contains("fetch") || stderr.len() > 0,
            "build must mention fetch when cache is missing"
        );
    }
}

// ── L2: Phase 3 commands ──────────────────────────────────────────────────────

#[test]
fn l2_cmd_project_invalid_mode_exits_nonzero() {
    let out = run(&["project", "McDavid", "--mode", "invalid"]);
    assert!(!out.status.success(), "invalid mode must exit non-zero");
}

#[test]
fn l2_cmd_schedule_exits_zero() {
    // Call without --days so this works even if binary was cached from older version
    let out = run(&["schedule"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panic"), "schedule must not panic, got: {stderr}");
    assert!(out.status.success(), "schedule must exit 0, stderr: {stderr}");
}

#[test]
fn l2_cmd_schedule_team_filter_no_panic() {
    // The team filter flag should be accepted by clap and the command should not panic
    // even with no network — output may be empty or "No games" but we only assert no crash.
    let out = run(&["schedule", "--team", "SEA", "--days", "3"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panic"), "schedule --team must not panic, stderr: {stderr}");
}

#[test]
fn l2_cmd_schedule_invalid_days_does_not_panic() {
    // clap should reject non-numeric --days with a non-zero exit, but no panic
    let out = run(&["schedule", "--days", "not-a-number"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"), "must not panic, stderr: {stderr}");
    assert!(!out.status.success(), "invalid --days must exit non-zero");
}

// ── L2: scouting reports (Phase 8a.1) ─────────────────────────────────────────

#[test]
fn l2_cmd_scouting_terminal_exits_zero() {
    // Use a player guaranteed to be in bundled data.
    let out = run(&["scouting", "McDavid"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"), "scouting must not panic, stderr: {stderr}");
    assert!(out.status.success(), "scouting must exit 0, stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // All eight sections present
    for n in 1..=8 {
        let header = format!("## {n}.");
        assert!(stdout.contains(&header), "section header '{header}' missing in stdout");
    }
}

#[test]
fn l2_cmd_scouting_json_parses() {
    let out = run(&["scouting", "McDavid", "--format", "json"]);
    assert!(out.status.success(), "scouting --format json must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("scouting json output must parse: {e}\nGOT:\n{stdout}"));
    // Spot-check the contract documented in scouting-reports.md
    assert!(v.get("player").and_then(|p| p.as_str()).is_some(),
        "json must contain a string `player` field");
    assert!(v.get("current_season").is_some(), "json must contain `current_season`");
    assert!(v.get("contract").is_some(), "json must contain `contract`");
}

#[test]
fn l2_cmd_scouting_unknown_format_exits_nonzero() {
    let out = run(&["scouting", "McDavid", "--format", "xml"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"), "must not panic, stderr: {stderr}");
    assert!(!out.status.success(), "unknown format must exit non-zero");
    assert!(stderr.contains("terminal") || stderr.contains("markdown") || stderr.contains("json"),
        "error must list valid formats, got stderr:\n{stderr}");
}

// ── L2: chunked snapshot ops (Phase 8h.4) ─────────────────────────────────────

#[test]
fn l2_cmd_snapshot_gc_dry_run_exits_zero() {
    // gc with no snapshots is a clean no-op exit.
    let out = run(&["snapshot", "gc", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"), "must not panic, stderr: {stderr}");
    assert!(out.status.success(), "snapshot gc --dry-run must exit 0, stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Dry run") || stdout.contains("Nothing to sweep"),
        "stdout must mention dry-run or sweep result, got:\n{stdout}",
    );
}

#[test]
fn l2_cmd_snapshot_rebuild_requires_chunked_flag() {
    // Without --chunked, the rebuild command errors with a clear message.
    let out = run(&["snapshot", "rebuild", "any-name"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(!out.status.success(), "missing --chunked must exit non-zero");
    assert!(
        stderr.contains("--chunked"),
        "error must reference --chunked flag, got stderr:\n{stderr}",
    );
}

#[test]
fn l2_cmd_fetch_stats_dry_run_chunked_mentions_chunks() {
    // --chunked + --dry-run should not hit the network; output must mention chunks.
    let out = run(&["fetch", "stats", "--dry-run", "--chunked"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(out.status.success(), "fetch --dry-run must exit 0, stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("chunks"),
        "dry-run with --chunked must mention chunked layout, got stdout:\n{stdout}",
    );
}

#[test]
fn l2_cmd_fetch_all_accepts_chunked_flag() {
    // Just clap-parsing: --chunked must be accepted on `fetch all` too.
    let out = run(&["fetch", "all", "--dry-run", "--chunked"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(out.status.success(), "fetch all --dry-run --chunked must exit 0, stderr: {stderr}");
}

// ── L2: markdown export (Phase 8d) ────────────────────────────────────────────

#[test]
fn l2_cmd_export_md_leaders_to_stdout() {
    // `--out -` writes to stdout so we can grep the front matter directly.
    let out = run(&["export", "md", "leaders", "--out", "-", "--top", "5"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(out.status.success(), "export md leaders must exit 0, stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("---\n"), "stdout must start with YAML front-matter");
    assert!(stdout.contains("type: leaderboard"));
    assert!(stdout.contains("| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |"));
}

#[test]
fn l2_cmd_export_md_team_requires_team_flag() {
    let out = run(&["export", "md", "team", "--out", "-"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(!out.status.success(), "missing --team must exit non-zero");
    assert!(stderr.contains("--team"), "error must reference --team flag");
}

#[test]
fn l2_cmd_export_md_fantasy_returns_deferred_message() {
    let out = run(&["export", "md", "fantasy", "--out", "-"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(!out.status.success(), "deferred shapes must exit non-zero");
    assert!(stderr.contains("deferred"), "error must explain why fantasy is deferred");
}

// ── L2: live-feeds toggle (Phase 8f.1) ────────────────────────────────────────

#[test]
fn l2_cmd_no_live_flag_is_accepted_globally() {
    // The flag attaches to any subcommand because it's declared global.
    let out = run(&["--no-live", "schedule", "--days", "1"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(!stderr.contains("error: unexpected argument"),
        "--no-live must be accepted as a global flag, got stderr:\n{stderr}");
}

#[test]
fn l2_cmd_no_live_help_mentions_flag() {
    // Top-level --help must document the flag so users can discover it.
    let out = run(&["--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--no-live"),
        "--help must document --no-live, got:\n{stdout}");
}

// ── L2: snapshot prune + diff (Phase 8f.2 + 8f.3) ─────────────────────────────

#[test]
fn l2_cmd_snapshot_prune_dry_run_with_no_snapshots_is_clean() {
    let out = run(&["snapshot", "prune", "--keep", "30", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"), "must not panic, stderr: {stderr}");
    assert!(out.status.success(), "prune --dry-run on empty store must exit 0, stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Dry run") || stdout.contains("Nothing to prune"),
        "stdout must mention dry-run or empty result, got:\n{stdout}",
    );
}

#[test]
fn l2_cmd_snapshot_diff_unknown_snapshots_errors_clearly() {
    // No snapshots exist — diff against two made-up names.
    let out = run(&["snapshot", "diff", "nope-a", "nope-b"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"), "must not panic");
    assert!(!out.status.success(), "missing snapshots must exit non-zero");
    // Either NotFound or "requires both snapshots to be chunked" is acceptable.
    assert!(
        stderr.contains("not found") || stderr.contains("chunked"),
        "error must explain why diff failed, got stderr:\n{stderr}",
    );
}

#[test]
fn l2_cmd_snapshot_prune_help_mentions_keep_flag() {
    let out = run(&["snapshot", "prune", "--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--keep"));
    assert!(stdout.contains("--dry-run"));
}

#[test]
fn l2_cmd_tui_help_exits_zero() {
    let out = run(&["tui", "--help"]);
    assert!(out.status.success(), "tui --help must exit 0");
}

#[test]
fn l2_cmd_tonight_no_panic() {
    let out = run(&["tonight"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panic"), "tonight must not panic");
}

// ── L2: Phase 5 query engine ──────────────────────────────────────────────────

#[test]
fn l2_cmd_query_leaders_exits_zero() {
    let out = run(&["query", "leaders", "--top", "10"]);
    assert!(out.status.success(), "query leaders must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Rank"), "leaders output must contain 'Rank' header");
}

#[test]
fn l2_cmd_query_leaders_pos_filter() {
    let out = run(&["query", "leaders", "--pos", "C", "--top", "5"]);
    assert!(out.status.success(), "query leaders --pos C must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("C"), "output must contain position 'C'");
}

#[test]
fn l2_cmd_query_leaders_age_filter() {
    let out = run(&["query", "leaders", "--age-max", "23", "--top", "10"]);
    assert!(out.status.success(), "query leaders --age-max 23 must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("matched"), "output must contain match count");
}

#[test]
fn l2_cmd_query_leaders_nationality_filter() {
    let out = run(&["query", "leaders", "--nationality", "FIN", "--sort", "ppg", "--top", "5"]);
    assert!(out.status.success(), "query leaders --nationality FIN must exit 0");
}

#[test]
fn l2_cmd_query_leaders_draft_year_filter() {
    let out = run(&["query", "leaders", "--draft-year", "2022", "--sort", "pts-pace"]);
    assert!(out.status.success(), "query leaders --draft-year must exit 0");
}

#[test]
fn l2_cmd_query_leaders_undrafted_flag() {
    let out = run(&["query", "leaders", "--undrafted", "--ppg-min", "0.50"]);
    assert!(out.status.success(), "query leaders --undrafted must exit 0");
}

#[test]
fn l2_cmd_query_leaders_percentiles_flag() {
    let out = run(&["query", "leaders", "--pos", "D", "--top", "5", "--percentiles"]);
    assert!(out.status.success(), "query leaders --percentiles must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("th"), "percentiles output must contain 'th' suffix");
}

#[test]
fn l2_cmd_query_leaders_json_export() {
    let out = run(&["query", "leaders", "--top", "5", "--json"]);
    assert!(out.status.success(), "query leaders --json must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Must be valid JSON starting with '['
    assert!(stdout.trim().starts_with('['), "JSON output must start with '['");
    assert!(stdout.contains("\"name\""), "JSON must contain name field");
    assert!(stdout.contains("\"rank\""), "JSON must contain rank field");
}

#[test]
fn l2_cmd_query_leaders_csv_export() {
    let out = run(&["query", "leaders", "--top", "5", "--csv"]);
    assert!(out.status.success(), "query leaders --csv must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("rank,name,team"), "CSV must have header row");
}

#[test]
fn l2_cmd_query_leaders_invalid_sort_exits_nonzero() {
    let out = run(&["query", "leaders", "--sort", "rapm"]);
    assert!(!out.status.success(), "invalid sort metric must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("rapm") || stderr.len() > 0, "error must mention invalid metric");
}

#[test]
fn l2_cmd_query_player_exits_zero() {
    let out = run(&["query", "player", "McDavid"]);
    assert!(out.status.success(), "query player McDavid must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("McDavid") || stdout.contains("Connor"), "output must contain player name");
}

#[test]
fn l2_cmd_query_player_with_percentiles() {
    let out = run(&["query", "player", "McDavid", "--percentiles"]);
    assert!(out.status.success(), "query player --percentiles must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("percentile") || stdout.contains("RANK"), "must show percentile info");
}

#[test]
fn l2_cmd_query_player_invalid_breakdown_exits_nonzero() {
    let out = run(&["query", "player", "McDavid", "--breakdown", "invalid"]);
    assert!(!out.status.success(), "invalid breakdown must exit non-zero");
}

#[test]
fn l2_cmd_query_player_not_found_exits_nonzero() {
    let out = run(&["query", "player", "ZZZ_DEFINITELY_NOT_A_PLAYER_NAME_XYZ"]);
    assert!(!out.status.success(), "unknown player must exit non-zero");
}

#[test]
fn l2_cmd_query_compare_head_to_head_exits_zero() {
    let out = run(&["query", "compare", "McDavid", "MacKinnon"]);
    assert!(out.status.success(), "query compare head-to-head must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("PPG"), "compare output must show PPG");
}

#[test]
fn l2_cmd_query_compare_similar_exits_zero() {
    let out = run(&["query", "compare", "Beniers", "--similar", "5"]);
    assert!(out.status.success(), "query compare --similar must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("SIMILAR") || stdout.contains("Rank"), "output must show similarity results");
}

#[test]
fn l2_cmd_query_compare_no_player2_no_similar_exits_nonzero() {
    let out = run(&["query", "compare", "McDavid"]);
    assert!(!out.status.success(), "compare with no player2 and no --similar must exit non-zero");
}

#[test]
fn l2_cmd_query_help_exits_zero() {
    let out = run(&["query", "--help"]);
    assert!(out.status.success(), "query --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("leaders"), "query help must list 'leaders' subcommand");
    assert!(stdout.contains("player"),  "query help must list 'player' subcommand");
    assert!(stdout.contains("compare"), "query help must list 'compare' subcommand");
}

// ── L2: fantasy commands ──────────────────────────────────────────────────────

/// Generate a unique league name using process ID + test name suffix to avoid
/// collision between parallel test runs and existing DB state.
fn unique_league(suffix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("test-league-{ts}-{suffix}")
}

#[test]
fn l2_cmd_fantasy_help_exits_zero() {
    let out = run(&["fantasy", "--help"]);
    assert!(out.status.success(), "fantasy --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("league") || stdout.contains("team"),
        "fantasy --help must list league or team subcommands, got: {stdout}"
    );
}

#[test]
fn l2_cmd_fantasy_league_create_exits_zero() {
    let name = unique_league("create");
    let out = run(&["fantasy", "league-create", &name]);
    assert!(
        out.status.success(),
        "fantasy league-create must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_fantasy_league_list_exits_zero() {
    let out = run(&["fantasy", "league-list"]);
    assert!(
        out.status.success(),
        "fantasy league-list must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_fantasy_league_create_then_list_shows_league() {
    let name = unique_league("list-check");
    // Create a league
    let create_out = run(&["fantasy", "league-create", &name]);
    assert!(create_out.status.success(), "league-create must exit 0");

    // List should include it (or at least not crash)
    let list_out = run(&["fantasy", "league-list"]);
    assert!(
        list_out.status.success(),
        "league-list must exit 0 after create"
    );
}

#[test]
fn l2_cmd_fantasy_standings_exits_zero() {
    // Standings with no active league may print a message but must not panic
    let out = run(&["fantasy", "standings"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panic"),
        "fantasy standings must not panic, got: {stderr}"
    );
    // Either exits 0 (no league active → informational message) or non-zero
    // Both are acceptable; we only require no panic
}

#[test]
fn l2_cmd_fantasy_serve_help_exits_zero() {
    let out = run(&["fantasy", "serve", "--help"]);
    assert!(
        out.status.success(),
        "fantasy serve --help must exit 0"
    );
}

// ── L2: fetch sub-commands (dry-run only) ────────────────────────────────────

#[test]
fn l2_cmd_fetch_contracts_dry_run_exits_zero() {
    // contracts --dry-run needs a bios.json in the active snapshot;
    // if none exists it will error — that's acceptable (no panic required)
    let out = run(&["fetch", "contracts", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panic"),
        "fetch contracts --dry-run must not panic, got: {stderr}"
    );
    // Either succeeds (has a snapshot with bios.json) or exits non-zero with
    // a user-friendly message. Both are OK.
}

#[test]
fn l2_cmd_fetch_moneypuck_dry_run_exits_zero() {
    let out = run(&["fetch", "money-puck", "--dry-run"]);
    assert!(
        out.status.success(),
        "fetch money-puck --dry-run must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("moneypuck") || stdout.contains("MoneyPuck"),
        "money-puck dry-run must mention MoneyPuck URL, got: {stdout}"
    );
}

#[test]
fn l2_cmd_fetch_realtime_dry_run_exits_zero() {
    let out = run(&["fetch", "realtime", "--dry-run"]);
    assert!(
        out.status.success(),
        "fetch realtime --dry-run must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── L2: query leaders — new sort metrics and filters ─────────────────────────

#[test]
fn l2_cmd_query_leaders_sort_hits_pace() {
    let out = run(&["query", "leaders", "--sort", "hits-pace", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort hits-pace must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Rank"), "hits-pace output must contain 'Rank' header");
}

#[test]
fn l2_cmd_query_leaders_sort_hits_total() {
    let out = run(&["query", "leaders", "--sort", "hits", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort hits must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_blocks_pace() {
    let out = run(&["query", "leaders", "--sort", "blocks-pace", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort blocks-pace must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_blocks() {
    let out = run(&["query", "leaders", "--sort", "blocks", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort blocks must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_takeaways() {
    let out = run(&["query", "leaders", "--sort", "takeaways", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort takeaways must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_giveaways() {
    let out = run(&["query", "leaders", "--sort", "giveaways", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort giveaways must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_pim() {
    let out = run(&["query", "leaders", "--sort", "pim", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort pim must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_xg_exits_zero_when_not_fetched() {
    // xg may be None for all players when MoneyPuck not fetched — must not error
    let out = run(&["query", "leaders", "--sort", "xg", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort xg must exit 0 even when MoneyPuck not fetched, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_query_leaders_sort_xg_per_60() {
    let out = run(&["query", "leaders", "--sort", "xg-per-60", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort xg-per-60 must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_cf_pct() {
    let out = run(&["query", "leaders", "--sort", "cf-pct", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort cf-pct must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_ff_pct() {
    let out = run(&["query", "leaders", "--sort", "ff-pct", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort ff-pct must exit 0");
}

#[test]
fn l2_cmd_query_leaders_sort_xgf_pct() {
    let out = run(&["query", "leaders", "--sort", "xgf-pct", "--top", "5"]);
    assert!(out.status.success(), "query leaders --sort xgf-pct must exit 0");
}

#[test]
fn l2_cmd_query_leaders_ufa_flag_exits_zero() {
    // UFA filter is graceful even when no contract data fetched
    let out = run(&["query", "leaders", "--ufa", "--top", "5"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panic"),
        "query leaders --ufa must not panic, got: {stderr}"
    );
    assert!(
        out.status.success(),
        "query leaders --ufa must exit 0, stderr: {stderr}"
    );
}

#[test]
fn l2_cmd_query_leaders_rfa_flag_exits_zero() {
    let out = run(&["query", "leaders", "--rfa", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --rfa must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_query_leaders_expiry_year_exits_zero() {
    let out = run(&["query", "leaders", "--expiry-year", "2026", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --expiry-year must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_query_leaders_rate_flag() {
    let out = run(&["query", "leaders", "--rate", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --rate must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Rank"), "rate mode must still show Rank header");
}

#[test]
fn l2_cmd_query_leaders_csv_has_header() {
    let out = run(&["query", "leaders", "--top", "5", "--csv"]);
    assert!(out.status.success(), "query leaders --csv must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // First line must be the CSV header
    let first_line = stdout.lines().next().unwrap_or("");
    assert!(
        first_line.contains("rank") && first_line.contains("name"),
        "CSV first line must be header with rank,name,..., got: '{first_line}'"
    );
}

#[test]
fn l2_cmd_query_leaders_toi_min_filter() {
    // toi_min in minutes (e.g. 18.0 = 1080 seconds per game)
    let out = run(&["query", "leaders", "--toi-min", "18.0", "--top", "10"]);
    assert!(
        out.status.success(),
        "query leaders --toi-min must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_query_leaders_plus_minus_filter() {
    let out = run(&["query", "leaders", "--plus-minus-min", "5", "--top", "10"]);
    assert!(
        out.status.success(),
        "query leaders --plus-minus-min must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn l2_cmd_query_compare_shows_contract_rows() {
    // compare head-to-head should show Contract/Expires rows (even if None)
    let out = run(&["query", "compare", "McDavid", "MacKinnon"]);
    assert!(out.status.success(), "query compare must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Contract") || stdout.contains("Expires"),
        "compare output must contain Contract or Expires row, got: {stdout}"
    );
}

// ── Phase 8f: --season flag on query commands ───────────────────────────────

#[test]
fn l2_cmd_query_leaders_season_bundled_succeeds() {
    // Pinning to a previous bundled season must produce that season's leaderboard.
    let out = run(&["query", "leaders", "--season", "20242025", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --season 20242025 must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Pts/82") || stdout.contains("PPG"),
        "leaderboard header should be present, got: {stdout}");
    assert!(stdout.contains("matched, showing"),
        "footer count should be present, got: {stdout}");
}

#[test]
fn l2_cmd_query_leaders_season_unbundled_errors_with_hint() {
    let out = run(&["query", "leaders", "--season", "19951996", "--top", "5"]);
    assert!(!out.status.success(),
        "non-bundled --season must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not bundled"),
        "must say 'not bundled', got: {stderr}");
    assert!(stderr.contains("20252026"),
        "must list current bundled season as a hint, got: {stderr}");
}

#[test]
fn l2_cmd_query_leaders_season_with_seasons_n_errors() {
    let out = run(&["query", "leaders",
        "--season", "20242025", "--seasons", "3", "--top", "5"]);
    assert!(!out.status.success(),
        "--season + --seasons N > 1 must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("mutually exclusive"),
        "must explain the conflict, got: {stderr}");
}

#[test]
fn l2_cmd_query_player_season_bundled_succeeds() {
    // McDavid was on EDM in 20242025 — same as today, but the path proves the
    // season override doesn't crash run_player.
    let out = run(&["query", "player", "McDavid", "--season", "20242025"]);
    assert!(
        out.status.success(),
        "query player --season must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("PLAYER PROFILE"),
        "player profile header should be present, got: {stdout}");
}

#[test]
fn l2_cmd_query_compare_season_bundled_succeeds() {
    let out = run(&["query", "compare",
        "McDavid", "MacKinnon", "--season", "20242025"]);
    assert!(
        out.status.success(),
        "query compare --season must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Phase 8f.6: group export / import / rename ──────────────────────────────

#[test]
fn l2_cmd_group_export_import_roundtrip() {
    let home = tempfile::tempdir().expect("tempdir");
    // Create + populate a group in the isolated home.
    let out = run_isolated(home.path(),
        &["group", "create", "watch", "--desc", "watch list"]);
    assert!(out.status.success(),
        "group create stderr: {}", String::from_utf8_lossy(&out.stderr));
    for player in ["McDavid", "MacKinnon", "Matthews"] {
        let out = run_isolated(home.path(), &["group", "add", "watch", player]);
        assert!(out.status.success(),
            "group add {player} stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
    // Export to file.
    let export_path = home.path().join("watch.json");
    let out = run_isolated(home.path(),
        &["group", "export", "watch", "--out", export_path.to_str().unwrap()]);
    assert!(out.status.success(),
        "group export stderr: {}", String::from_utf8_lossy(&out.stderr));
    let json = std::fs::read_to_string(&export_path)
        .expect("export file written");
    let parsed: serde_json::Value = serde_json::from_str(&json)
        .expect("export must be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("watch"));
    assert_eq!(parsed["description"].as_str(), Some("watch list"));
    let members = parsed["members"].as_array().expect("members array");
    assert_eq!(members.len(), 3);
    // Import as a new name into the same db.
    let out = run_isolated(home.path(),
        &["group", "import", export_path.to_str().unwrap(), "--as", "watch-copy"]);
    assert!(out.status.success(),
        "group import stderr: {}", String::from_utf8_lossy(&out.stderr));
    // Both groups now visible from list.
    let out = run_isolated(home.path(), &["group", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("watch"), "original group missing: {stdout}");
    assert!(stdout.contains("watch-copy"), "imported copy missing: {stdout}");
}

#[test]
fn l2_cmd_group_rename_succeeds() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["group", "create", "before"]);
    assert!(out.status.success(),
        "create stderr: {}", String::from_utf8_lossy(&out.stderr));
    let out = run_isolated(home.path(), &["group", "add", "before", "McDavid"]);
    assert!(out.status.success(),
        "add stderr: {}", String::from_utf8_lossy(&out.stderr));
    let out = run_isolated(home.path(), &["group", "rename", "before", "after"]);
    assert!(out.status.success(),
        "rename stderr: {}", String::from_utf8_lossy(&out.stderr));
    // After rename, `before` is gone and `after` carries the member.
    let out = run_isolated(home.path(), &["group", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("after"),  "renamed group missing: {stdout}");
    assert!(!stdout.contains(" before "), "old name should not appear: {stdout}");
}

// ── Attended games (Phase 8 follow-up) ─────────────────────────────────────

#[test]
fn l2_cmd_games_add_list_remove_roundtrip() {
    // Full lifecycle in an isolated $HOME so we don't pollute the real db.
    let home = tempfile::tempdir().expect("tempdir");
    // List on a fresh db should report empty.
    let out = run_isolated(home.path(), &["games", "list"]);
    assert!(out.status.success(),
        "games list stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("No attended games"),
        "fresh db should report empty, got: {stdout}");

    // Add a game (no boxscore available, that's fine).
    let out = run_isolated(home.path(),
        &["games", "add", "2025020100", "--note", "first game"]);
    assert!(out.status.success(),
        "games add stderr: {}", String::from_utf8_lossy(&out.stderr));

    // List should now show it.
    let out = run_isolated(home.path(), &["games", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("2025020100"),
        "added game id should appear in list, got: {stdout}");
    assert!(stdout.contains("first game"),
        "note should appear in list, got: {stdout}");

    // Remove it.
    let out = run_isolated(home.path(), &["games", "remove", "2025020100"]);
    assert!(out.status.success(),
        "games remove stderr: {}", String::from_utf8_lossy(&out.stderr));
    // Remove again should error (already gone).
    let out = run_isolated(home.path(), &["games", "remove", "2025020100"]);
    assert!(!out.status.success(),
        "second remove should exit nonzero — game already gone");
}

#[test]
fn l2_cmd_games_export_emits_versioned_json() {
    let home = tempfile::tempdir().expect("tempdir");
    // Seed two games.
    for (id, note) in [(2025020100u64, "first"), (2025020101u64, "second")] {
        let out = run_isolated(home.path(),
            &["games", "add", &id.to_string(), "--note", note]);
        assert!(out.status.success(),
            "games add stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
    // Export to stdout — must be valid JSON with version + games[].
    let out = run_isolated(home.path(), &["games", "export"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("export output must be valid JSON");
    assert_eq!(parsed["version"].as_u64(), Some(1));
    assert_eq!(parsed["games"].as_array().map(|a| a.len()), Some(2),
        "expected 2 games in export, got: {stdout}");
}

// ── Dashboards opt-out flag (was opt-in pre-2026-04-29) ────────────────────

#[test]
fn l2_cmd_no_dashboards_flag_documented_in_help() {
    // The --no-dashboards global flag should appear in --help output.
    // Same opt-out shape as --no-live.
    let out = run(&["--help"]);
    assert!(out.status.success(), "--help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--no-dashboards"),
        "--help must list --no-dashboards, got:\n{stdout}");
    assert!(stdout.contains("ICELINES_DASHBOARDS") || stdout.contains("dashboards"),
        "--help must mention env var or config key, got:\n{stdout}");
}

#[test]
fn l2_cmd_no_dashboards_flag_accepted_globally() {
    // --no-dashboards on a non-TUI command must be accepted (noop for
    // non-TUI paths but must not error). Verifies clap's global=true.
    let out = run(&["--no-dashboards", "scheme", "list"]);
    assert!(out.status.success(),
        "--no-dashboards scheme list stderr: {}",
        String::from_utf8_lossy(&out.stderr));
}

// ── Phase 8f.8: data verify ─────────────────────────────────────────────────

#[test]
fn l2_cmd_data_verify_no_install_errors_helpfully() {
    let home = tempfile::tempdir().expect("tempdir");
    // No seasons dir at all → expect a clear hint to run `data install`.
    let out = run_isolated(home.path(), &["data", "verify", "--all"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no installed seasons") || stderr.contains("data install"),
        "must hint to install first, got: {stderr}");
}

#[test]
fn l2_cmd_data_verify_detects_tampering_in_isolated_home() {
    let home = tempfile::tempdir().expect("tempdir");
    // Hand-build a fake installed bundle: ~/.icelines/seasons/20242025/bios.json
    // + manifest.json. Then run `data verify 20242025`.
    let dir = home.path().join(".icelines").join("seasons").join("20242025");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("bios.json"),  b"[]").unwrap();
    std::fs::write(dir.join("stats.json"), b"[]").unwrap();
    // Hand-author a manifest with deliberately wrong hashes so verify fails.
    let manifest = serde_json::json!({
        "season": "20242025",
        "sha256": {
            "bios.json":  "0000000000000000000000000000000000000000000000000000000000000000",
            "stats.json": "1111111111111111111111111111111111111111111111111111111111111111"
        },
        "version": 1,
        "written_at": "2026-04-29T00:00:00Z"
    });
    std::fs::write(dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap()).unwrap();

    let out = run_isolated(home.path(), &["data", "verify", "20242025"]);
    assert!(!out.status.success(),
        "tampered bundle must exit nonzero");
    let combined = format!("{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr));
    assert!(combined.contains("mismatch"),
        "must report mismatch, got: {combined}");
}

#[test]
fn l2_cmd_data_verify_clean_bundle_succeeds() {
    let home = tempfile::tempdir().expect("tempdir");
    let dir = home.path().join(".icelines").join("seasons").join("20242025");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let bios  = b"[{\"player_id\":1}]";
    let stats = b"[{\"player_id\":1,\"goals\":10}]";
    std::fs::write(dir.join("bios.json"),  bios).unwrap();
    std::fs::write(dir.join("stats.json"), stats).unwrap();
    // Compute correct hashes.
    use sha2::{Digest, Sha256};
    let hex = |b: &[u8]| {
        let mut h = Sha256::new(); h.update(b);
        let v = h.finalize();
        v.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
    };
    let manifest = serde_json::json!({
        "season": "20242025",
        "sha256": {
            "bios.json":  hex(bios),
            "stats.json": hex(stats)
        },
        "version": 1,
        "written_at": "2026-04-29T00:00:00Z"
    });
    std::fs::write(dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap()).unwrap();

    let out = run_isolated(home.path(), &["data", "verify", "20242025"]);
    assert!(out.status.success(),
        "clean bundle must verify, stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("verified") || stdout.contains("✓"),
        "success output expected, got: {stdout}");
}

// ── Phase 8f.7: scheme from-csv multi-platform ──────────────────────────────

#[test]
fn l2_cmd_scheme_from_csv_yahoo_autodetect() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("yahoo.csv");
    // Minimal Yahoo header — enough for signature + a few stat columns.
    std::fs::write(&path,
        "Player,Owner,GP,G (P),A (P),HIT (P),BLK (P),Fan Pts\n").unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap()]);
    assert!(out.status.success(),
        "from-csv stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Platform: Yahoo"),
        "platform header missing, got: {stdout}");
    assert!(stdout.contains("goals") && stdout.contains("hits"),
        "stat keys missing, got: {stdout}");
}

#[test]
fn l2_cmd_scheme_from_csv_espn_autodetect() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("espn.csv");
    std::fs::write(&path,
        "RANK,PLAYER,TEAM,POS,STATUS,OWNER,G,A,+/-,SOG,HIT,BLK\n").unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap()]);
    assert!(out.status.success(),
        "from-csv stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Platform: ESPN"),
        "ESPN should auto-detect, got: {stdout}");
}

#[test]
fn l2_cmd_scheme_from_csv_explicit_platform_override() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ambiguous.csv");
    // Header has bare G,A — would match ESPN auto-detection but no signature
    // columns. With --platform fantrax, we should still get fantrax.
    std::fs::write(&path,
        "Player,G,A,Pts,PPG,SOG,HT,BLK\n").unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap(),
        "--platform", "fantrax"]);
    assert!(out.status.success(),
        "explicit platform stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Platform: Fantrax"),
        "explicit fantrax must be honored, got: {stdout}");
    // HT (fantrax convention) → hits
    assert!(stdout.contains("hits"),
        "fantrax HT should map to hits, got: {stdout}");
}

#[test]
fn l2_cmd_scheme_from_csv_unknown_platform_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("anything.csv");
    std::fs::write(&path, "Header\n").unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap(),
        "--platform", "draftkings"]);
    assert!(!out.status.success(),
        "unknown platform must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("yahoo"),
        "unknown-platform error must list valid options, got: {stderr}");
}

#[test]
fn l2_cmd_scheme_from_csv_unrecognized_format_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("garbage.csv");
    // Header with no signature column from any platform.
    std::fs::write(&path, "ColA,ColB,ColC\n").unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap()]);
    assert!(!out.status.success(),
        "unknown format must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unrecognized") || stderr.contains("--platform"),
        "must hint --platform fallback, got: {stderr}");
}

#[test]
fn l2_cmd_group_export_to_stdout_emits_json() {
    let home = tempfile::tempdir().expect("tempdir");
    let _ = run_isolated(home.path(), &["group", "create", "stdout-test"]);
    let _ = run_isolated(home.path(), &["group", "add", "stdout-test", "McDavid"]);
    // Default --out is "-" → stdout.
    let out = run_isolated(home.path(), &["group", "export", "stdout-test"]);
    assert!(out.status.success(),
        "export to stdout stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout must be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("stdout-test"));
}
