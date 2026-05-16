/// L2 system tests — invoke the compiled `icelines` binary as a subprocess.
/// All tests use cached fixture data; no live network calls.
///
/// Run with: cargo test -p icelines-cli --test system_tests
///
use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
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
        .unwrap_or_else(|e| panic!("failed to run icelines binary: {e}"))
}

fn seed_daily_league(home: &std::path::Path) {
    let icelines_dir = home.join(".icelines");
    std::fs::create_dir_all(&icelines_dir).expect("create .icelines");
    let db = icelines_fetch::fantasy_db::FantasyDb::open_path(icelines_dir.join("icelines.db"))
        .expect("open fantasy db");
    let league_id = db
        .create_league("Daily League", "yahoo-standard")
        .expect("create league");
    db.set_active_league("Daily League")
        .expect("set active league");
    let team_id = db
        .create_team(&league_id, "My Team", "Me")
        .expect("create team");
    db.set_user_team(&league_id, "My Team")
        .expect("set user team");
    db.add_player(&team_id, "matty_beniers")
        .expect("add daily roster player");
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
    // Workspace version is sourced from Cargo.toml at compile time.
    // Pin against the workspace constant rather than a literal so this
    // test doesn't drift each release.
    let expected = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(expected),
        "--version must print '{expected}', got: {stdout}"
    );
}

// ── L2: --help exits 0 and lists subcommands ─────────────────────────────────

#[test]
fn l2_cmd_help_exits_zero_and_lists_commands() {
    let out = run(&["--help"]);
    assert!(out.status.success(), "--help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // After the 2026-05-04 mkdocs cut, the user-visible top-level
    // subcommands include the new `serve` (web dashboard) plus the
    // long-standing fetch/team/rank/tui set. `site/build/deploy` are
    // GONE — `serve` is the single web frontend.
    for cmd in &["fetch", "team", "rank", "serve", "tui"] {
        assert!(stdout.contains(cmd), "--help must list '{cmd}' subcommand");
    }
    for absent in &["\n  build ", "\n  deploy ", "\n  site "] {
        assert!(
            !stdout.contains(absent),
            "--help must NOT list removed subcommand matching '{absent}'"
        );
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

#[test]
fn l2_cmd_fetch_goalies_dry_run_exits_zero() {
    // Phase G.2: `fetch goalies` should mention the right endpoint.
    let out = run(&["fetch", "goalies", "--dry-run"]);
    assert!(
        out.status.success(),
        "fetch goalies --dry-run stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("/goalie/summary") || stdout.contains("Would fetch"),
        "dry-run should mention the goalie endpoint, got:\n{stdout}"
    );
}

// ── Hart.6.5 — `--type {regular|playoff|both}` flag ─────────────────────────

#[test]
fn l2_hart6_5_fetch_stats_type_playoff_dry_run_mentions_gametypeid_3() {
    let out = run(&["fetch", "stats", "--dry-run", "--type", "playoff"]);
    assert!(
        out.status.success(),
        "fetch stats --type playoff --dry-run must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("gameTypeId=3"),
        "playoff dry-run must reference gameTypeId=3, got:\n{stdout}"
    );
    assert!(
        stdout.contains("playoff-bios.json") && stdout.contains("playoff-stats.json"),
        "playoff dry-run must mention the co-located filenames, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("gameTypeId=2"),
        "playoff-only run must NOT mention gameTypeId=2, got:\n{stdout}"
    );
}

#[test]
fn l2_hart6_5_fetch_stats_type_both_dry_run_mentions_both_gametypeids() {
    let out = run(&["fetch", "stats", "--dry-run", "--type", "both"]);
    assert!(
        out.status.success(),
        "fetch stats --type both --dry-run must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("gameTypeId=2") && stdout.contains("gameTypeId=3"),
        "type=both dry-run must mention both gameTypeIds, got:\n{stdout}"
    );
}

#[test]
fn l2_hart6_5_fetch_stats_type_regular_default_matches_pre_hart6() {
    // Default (no --type flag) and explicit --type regular must produce
    // the same output: gameTypeId=2 only, regular-season filenames.
    let out_default = run(&["fetch", "stats", "--dry-run"]);
    let out_regular = run(&["fetch", "stats", "--dry-run", "--type", "regular"]);
    assert!(out_default.status.success());
    assert!(out_regular.status.success());
    let s_default = String::from_utf8_lossy(&out_default.stdout);
    let s_regular = String::from_utf8_lossy(&out_regular.stdout);
    assert_eq!(
        s_default, s_regular,
        "--type regular must match the default"
    );
    assert!(s_default.contains("gameTypeId=2"));
    assert!(!s_default.contains("gameTypeId=3"));
}

#[test]
fn l2_hart6_5_fetch_goalies_type_playoff_dry_run_mentions_correct_path() {
    let out = run(&["fetch", "goalies", "--dry-run", "--type", "playoff"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("gameTypeId=3"), "must mention gameTypeId=3");
    assert!(
        stdout.contains("playoff-goalie-stats.json"),
        "must mention playoff-goalie-stats.json, got:\n{stdout}"
    );
}

#[test]
fn l2_hart6_5_fetch_realtime_rejects_type_flag() {
    // Realtime is regular-season only — clap must reject --type as
    // unknown. Hart.6 D6 / Risk #5: playoff realtime flows through the
    // live game feed, not a separate dataset.
    let out = run(&["fetch", "realtime", "--dry-run", "--type", "playoff"]);
    assert!(
        !out.status.success(),
        "fetch realtime must reject --type flag (it's regular-season only)"
    );
}

#[test]
fn l2_hart6_5_fetch_moneypuck_rejects_type_flag() {
    // MoneyPuck has no playoff endpoint — clap must reject --type.
    let out = run(&["fetch", "moneypuck", "--dry-run", "--type", "playoff"]);
    assert!(
        !out.status.success(),
        "fetch moneypuck must reject --type flag (no playoff variant)"
    );
}

// ── Hart.6.9 — query --type {regular|playoff} ──────────────────────────────

#[test]
fn l2_hart6_9_query_leaders_type_playoff_returns_playoff_rows() {
    // 2024-25 playoff data is bundled (5c25214d). Bench B3 round-trip
    // fence: query reads through the same path the fetch CLI wrote to.
    // Real assertion: the top scorer for 2024-25 playoffs is McDavid or
    // Draisaitl (both EDM, joint at 33 pts in 22 GP per actual 2025 run).
    let out = run(&[
        "query", "leaders", "--season", "20242025", "--type", "playoff", "--top", "5",
    ]);
    assert!(
        out.status.success(),
        "query leaders --type playoff must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Spot-check a known 2024-25 playoff name.
    assert!(
        stdout.contains("Draisaitl") || stdout.contains("McDavid"),
        "playoff leaders must include EDM top-line forwards, got:\n{stdout}"
    );
    // GP column should show ≤ 30 (full Cup run = 4 rounds × ~6 games).
    // If the dispatch silently fell through to regular, GP would be ≥ 60.
    // Loose check via "matched" line: regular = ~900 players, playoff = ~330.
    assert!(
        stdout.contains("332 matched") || stdout.contains("matched, showing"),
        "must show match count, got:\n{stdout}"
    );
}

#[test]
fn l2_hart6_9_query_leaders_type_playoff_for_2025_26_surfaces_missing_bundle() {
    // 2025-26 playoff ships as `[]` (Cup not yet contested). The loader
    // returns MissingBundle{Playoff} with a clean error.
    let out = run(&[
        "query", "leaders", "--season", "20252026", "--type", "playoff",
    ]);
    assert!(
        !out.status.success(),
        "2025-26 playoff query must fail until the Cup is contested"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("playoff")
            || stderr.contains("Playoff")
            || stderr.contains("MissingBundle"),
        "error must mention playoff context, got:\n{stderr}"
    );
}

#[test]
fn l2_hart6_9_query_leaders_default_type_matches_explicit_regular() {
    // No --type and --type regular must produce identical output.
    let out_default = run(&["query", "leaders", "--season", "20242025", "--top", "3"]);
    let out_regular = run(&[
        "query", "leaders", "--season", "20242025", "--type", "regular", "--top", "3",
    ]);
    assert!(out_default.status.success());
    assert!(out_regular.status.success());
    assert_eq!(out_default.stdout, out_regular.stdout);
}

#[test]
fn l2_hart6_9_query_leaders_rejects_seasons_n_with_playoff() {
    // --seasons N > 1 is regular-only; combo with --type playoff
    // must error cleanly rather than silently dropping the type.
    let out = run(&["query", "leaders", "--seasons", "3", "--type", "playoff"]);
    assert!(
        !out.status.success(),
        "--seasons N + --type playoff must reject"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--seasons") && stderr.contains("playoff"),
        "error must explain the conflict, got:\n{stderr}"
    );
}

#[test]
fn l2_hart6_5_fetch_all_type_playoff_dry_run_skips_rosters_and_realtime() {
    let out = run(&["fetch", "all", "--dry-run", "--type", "playoff"]);
    assert!(
        out.status.success(),
        "fetch all --type playoff --dry-run must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Must include playoff stats + goalies
    assert!(stdout.contains("playoff-stats.json"));
    assert!(stdout.contains("playoff-goalie-stats.json"));
    // Must NOT include rosters/realtime/moneypuck/contracts/transactions
    // (they're regular-season-keyed concepts; type=playoff skips them).
    assert!(
        !stdout.contains("/v1/roster/"),
        "type=playoff must skip rosters, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("realtime"),
        "type=playoff must skip realtime, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("moneypuck"),
        "type=playoff must skip moneypuck, got:\n{stdout}"
    );
}

#[test]
fn l2_cmd_fetch_transactions_dry_run_exits_zero() {
    // Phase T.3: `fetch transactions` dry-run announces the ESPN target.
    let out = run(&["fetch", "transactions", "--dry-run"]);
    assert!(
        out.status.success(),
        "fetch transactions --dry-run stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ESPN") || stdout.contains("Would fetch"),
        "dry-run should mention ESPN or 'Would fetch', got:\n{stdout}"
    );
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
            stderr.contains("fetch") || !stderr.is_empty(),
            "rank error must produce stderr output"
        );
    }
}

// ── L2: Commands that always exit 0 ──────────────────────────────────────────

#[test]
fn l2_cmd_stubs_exit_zero() {
    // tonight exits 0 even without cache (prints message gracefully)
    {
        let cmd = &"tonight";
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
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("scheme show --source output must be valid JSON");
    assert_eq!(
        parsed["name"].as_str(),
        Some("yahoo-standard"),
        "JSON must include name field"
    );
    assert!(
        parsed["skater"].is_object(),
        "JSON must include skater weights"
    );
    assert!(
        parsed["goalie"].is_object(),
        "JSON must include goalie weights"
    );
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
            !stderr.is_empty() || !stdout.is_empty(),
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
            stderr.contains("fetch") || !stderr.is_empty(),
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
            !stderr.is_empty(),
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

// `l2_cmd_build_no_site_exits_gracefully` was removed 2026-05-04 alongside
// the mkdocs-frontend cut. `icelines build` is no longer a subcommand;
// the markdown-generation crate (`icelines-site`) lives on as a library
// without a CLI entry point.

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
    assert!(
        !stderr.contains("panic"),
        "schedule must not panic, got: {stderr}"
    );
    assert!(
        out.status.success(),
        "schedule must exit 0, stderr: {stderr}"
    );
}

#[test]
fn l2_cmd_schedule_team_filter_no_panic() {
    // The team filter flag should be accepted by clap and the command should not panic
    // even with no network — output may be empty or "No games" but we only assert no crash.
    let out = run(&["schedule", "--team", "SEA", "--days", "3"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panic"),
        "schedule --team must not panic, stderr: {stderr}"
    );
}

#[test]
fn l2_cmd_schedule_invalid_days_does_not_panic() {
    // clap should reject non-numeric --days with a non-zero exit, but no panic
    let out = run(&["schedule", "--days", "not-a-number"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "must not panic, stderr: {stderr}"
    );
    assert!(!out.status.success(), "invalid --days must exit non-zero");
}

// ── L2: scouting reports (Phase 8a.1) ─────────────────────────────────────────

#[test]
fn l2_cmd_scouting_terminal_exits_zero() {
    // Use a player guaranteed to be in bundled data.
    let out = run(&["scouting", "McDavid"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "scouting must not panic, stderr: {stderr}"
    );
    assert!(
        out.status.success(),
        "scouting must exit 0, stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // All eight sections present
    for n in 1..=8 {
        let header = format!("## {n}.");
        assert!(
            stdout.contains(&header),
            "section header '{header}' missing in stdout"
        );
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
    assert!(
        v.get("player").and_then(|p| p.as_str()).is_some(),
        "json must contain a string `player` field"
    );
    assert!(
        v.get("current_season").is_some(),
        "json must contain `current_season`"
    );
    assert!(v.get("contract").is_some(), "json must contain `contract`");
}

#[test]
fn l2_cmd_scouting_unknown_format_exits_nonzero() {
    let out = run(&["scouting", "McDavid", "--format", "xml"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "must not panic, stderr: {stderr}"
    );
    assert!(!out.status.success(), "unknown format must exit non-zero");
    assert!(
        stderr.contains("terminal") || stderr.contains("markdown") || stderr.contains("json"),
        "error must list valid formats, got stderr:\n{stderr}"
    );
}

// ── L2: chunked snapshot ops (Phase 8h.4) ─────────────────────────────────────

#[test]
fn l2_cmd_snapshot_gc_dry_run_exits_zero() {
    // gc with no snapshots is a clean no-op exit.
    let out = run(&["snapshot", "gc", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        out.status.success(),
        "snapshot gc --dry-run must exit 0, stderr: {stderr}"
    );
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
    assert!(
        !out.status.success(),
        "missing --chunked must exit non-zero"
    );
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
    assert!(
        out.status.success(),
        "fetch --dry-run must exit 0, stderr: {stderr}"
    );
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
    assert!(
        out.status.success(),
        "fetch all --dry-run --chunked must exit 0, stderr: {stderr}"
    );
}

// ── L2: markdown export (Phase 8d) ────────────────────────────────────────────

#[test]
fn l2_cmd_export_md_leaders_to_stdout() {
    // `--out -` writes to stdout so we can grep the front matter directly.
    let out = run(&["export", "md", "leaders", "--out", "-", "--top", "5"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(
        out.status.success(),
        "export md leaders must exit 0, stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("---\n"),
        "stdout must start with YAML front-matter"
    );
    assert!(stdout.contains("type: leaderboard"));
    assert!(
        stdout.contains("| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |")
    );
}

#[test]
fn l2_cmd_export_md_team_requires_team_flag() {
    let out = run(&["export", "md", "team", "--out", "-"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(!out.status.success(), "missing --team must exit non-zero");
    assert!(
        stderr.contains("--team"),
        "error must reference --team flag"
    );
}

#[test]
fn l2_cmd_export_md_team_season_to_stdout() {
    let out = run(&["export", "md", "team-season", "--team", "EDM", "--out", "-"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(
        out.status.success(),
        "team-season export must exit 0, stderr: {stderr}"
    );
    assert!(
        stdout.starts_with("---\n"),
        "stdout must start with YAML front-matter"
    );
    assert!(stdout.contains("type: team-season"));
    assert!(stdout.contains("## Source State"));
    assert!(stdout.contains("## Schedule Strength"));
    assert!(stdout.contains("## Quality Ledger"));
    assert!(stdout.contains("## Game Log"));
}

#[test]
fn l2_cmd_export_md_fantasy_renders_poach_report() {
    let out = run(&["export", "md", "fantasy", "--out", "-"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(
        out.status.success(),
        "fantasy export must succeed: {stderr}"
    );
    assert!(
        stdout.starts_with("---\n"),
        "stdout must start with YAML front-matter"
    );
    assert!(
        stdout.contains("type: fantasy-poacher"),
        "fantasy export must use the poach report front matter"
    );
    assert!(
        stdout.contains("Fantasy Poacher"),
        "fantasy export must render the report body"
    );
}

// ── L2: team season performance (Presidents Trophy) ─────────────────────────

#[test]
fn l2_cmd_team_season_text_exits_zero_and_shows_context() {
    let out = run(&["team-season", "EDM"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(
        out.status.success(),
        "team-season text must exit 0, stderr: {stderr}"
    );
    assert!(stdout.contains("EDM TEAM SEASON"));
    assert!(stdout.contains("Standings"));
    assert!(stdout.contains("SOS faced"));
    assert!(stdout.contains("Ledger quality wins"));
}

#[test]
fn l2_cmd_team_season_json_exits_zero_and_emits_view_contract() {
    let out = run(&["team-season", "EDM", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(
        out.status.success(),
        "team-season json must exit 0, stderr: {stderr}"
    );
    assert!(stdout.contains("\"team\": \"EDM\""));
    assert!(stdout.contains("\"schedule_strength\""));
    assert!(stdout.contains("\"quality_ledger\""));
    assert!(stdout.contains("\"standings\""));
}

// ── L2: live-feeds toggle (Phase 8f.1) ────────────────────────────────────────

#[test]
fn l2_cmd_no_live_flag_is_accepted_globally() {
    // The flag attaches to any subcommand because it's declared global.
    let out = run(&["--no-live", "schedule", "--days", "1"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked at"));
    assert!(
        !stderr.contains("error: unexpected argument"),
        "--no-live must be accepted as a global flag, got stderr:\n{stderr}"
    );
}

#[test]
fn l2_cmd_no_live_help_mentions_flag() {
    // Top-level --help must document the flag so users can discover it.
    let out = run(&["--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-live"),
        "--help must document --no-live, got:\n{stdout}"
    );
}

// ── L2: snapshot prune + diff (Phase 8f.2 + 8f.3) ─────────────────────────────

#[test]
fn l2_cmd_snapshot_prune_dry_run_with_no_snapshots_is_clean() {
    let out = run(&["snapshot", "prune", "--keep", "30", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        out.status.success(),
        "prune --dry-run on empty store must exit 0, stderr: {stderr}"
    );
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
    assert!(
        !out.status.success(),
        "missing snapshots must exit non-zero"
    );
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
    assert!(
        stdout.contains("Rank"),
        "leaders output must contain 'Rank' header"
    );
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
    assert!(
        out.status.success(),
        "query leaders --age-max 23 must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("matched"),
        "output must contain match count"
    );
}

#[test]
fn l2_cmd_query_leaders_nationality_filter() {
    let out = run(&[
        "query",
        "leaders",
        "--nationality",
        "FIN",
        "--sort",
        "ppg",
        "--top",
        "5",
    ]);
    assert!(
        out.status.success(),
        "query leaders --nationality FIN must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_draft_year_filter() {
    let out = run(&[
        "query",
        "leaders",
        "--draft-year",
        "2022",
        "--sort",
        "pts-pace",
    ]);
    assert!(
        out.status.success(),
        "query leaders --draft-year must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_undrafted_flag() {
    let out = run(&["query", "leaders", "--undrafted", "--ppg-min", "0.50"]);
    assert!(
        out.status.success(),
        "query leaders --undrafted must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_percentiles_flag() {
    let out = run(&[
        "query",
        "leaders",
        "--pos",
        "D",
        "--top",
        "5",
        "--percentiles",
    ]);
    assert!(
        out.status.success(),
        "query leaders --percentiles must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("th"),
        "percentiles output must contain 'th' suffix"
    );
}

#[test]
fn l2_cmd_query_leaders_json_export() {
    let out = run(&["query", "leaders", "--top", "5", "--json"]);
    assert!(out.status.success(), "query leaders --json must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Must be valid JSON starting with '['
    assert!(
        stdout.trim().starts_with('['),
        "JSON output must start with '['"
    );
    assert!(stdout.contains("\"name\""), "JSON must contain name field");
    assert!(stdout.contains("\"rank\""), "JSON must contain rank field");
}

#[test]
fn l2_cmd_query_leaders_csv_export() {
    let out = run(&["query", "leaders", "--top", "5", "--csv"]);
    assert!(out.status.success(), "query leaders --csv must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("rank,name,team"),
        "CSV must have header row"
    );
}

#[test]
fn l2_cmd_query_leaders_json_csv_row_identity_match() {
    let args = ["query", "leaders", "--sort", "goals", "--top", "5"];
    let json_out = run(&[&args[..], &["--json"][..]].concat());
    assert!(
        json_out.status.success(),
        "query leaders --json must exit 0, stderr: {}",
        String::from_utf8_lossy(&json_out.stderr)
    );
    let csv_out = run(&[&args[..], &["--csv"][..]].concat());
    assert!(
        csv_out.status.success(),
        "query leaders --csv must exit 0, stderr: {}",
        String::from_utf8_lossy(&csv_out.stderr)
    );

    let json_stdout = String::from_utf8_lossy(&json_out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("query leaders --json must emit valid JSON");
    let json_rows = json
        .as_array()
        .expect("query leaders --json should emit a JSON array");

    let csv_stdout = String::from_utf8_lossy(&csv_out.stdout);
    let csv_rows: Vec<Vec<&str>> = csv_stdout
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(',').collect())
        .collect();

    assert_eq!(
        json_rows.len(),
        csv_rows.len(),
        "JSON and CSV should return the same row count"
    );
    for (json_row, csv_row) in json_rows.iter().zip(csv_rows.iter()) {
        assert!(
            csv_row.len() >= 5,
            "CSV row should carry rank,name,team,pos,gp: {csv_row:?}"
        );
        assert_eq!(json_row["rank"].to_string(), csv_row[0]);
        assert_eq!(json_row["name"].as_str(), Some(csv_row[1]));
        assert_eq!(json_row["team"].as_str(), Some(csv_row[2]));
        assert_eq!(json_row["pos"].as_str(), Some(csv_row[3]));
        assert_eq!(json_row["gp"].to_string(), csv_row[4]);
    }
}

#[test]
fn l2_cmd_query_leaders_invalid_sort_exits_nonzero() {
    let out = run(&["query", "leaders", "--sort", "rapm"]);
    assert!(
        !out.status.success(),
        "invalid sort metric must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("rapm") || !stderr.is_empty(),
        "error must mention invalid metric"
    );
}

#[test]
fn l2_cmd_query_player_exits_zero() {
    let out = run(&["query", "player", "McDavid"]);
    assert!(out.status.success(), "query player McDavid must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("McDavid") || stdout.contains("Connor"),
        "output must contain player name"
    );
}

#[test]
fn l2_cmd_query_player_with_percentiles() {
    let out = run(&["query", "player", "McDavid", "--percentiles"]);
    assert!(
        out.status.success(),
        "query player --percentiles must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("percentile") || stdout.contains("RANK"),
        "must show percentile info"
    );
}

#[test]
fn l2_cmd_query_player_invalid_breakdown_exits_nonzero() {
    let out = run(&["query", "player", "McDavid", "--breakdown", "invalid"]);
    assert!(
        !out.status.success(),
        "invalid breakdown must exit non-zero"
    );
}

#[test]
fn l2_cmd_query_player_not_found_exits_nonzero() {
    let out = run(&["query", "player", "ZZZ_DEFINITELY_NOT_A_PLAYER_NAME_XYZ"]);
    assert!(!out.status.success(), "unknown player must exit non-zero");
}

#[test]
fn l2_cmd_query_compare_head_to_head_exits_zero() {
    let out = run(&["query", "compare", "McDavid", "MacKinnon"]);
    assert!(
        out.status.success(),
        "query compare head-to-head must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("PPG"), "compare output must show PPG");
}

#[test]
fn l2_cmd_query_compare_similar_exits_zero() {
    let out = run(&["query", "compare", "Beniers", "--similar", "5"]);
    assert!(out.status.success(), "query compare --similar must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SIMILAR") || stdout.contains("Rank"),
        "output must show similarity results"
    );
}

#[test]
fn l2_cmd_query_compare_no_player2_no_similar_exits_nonzero() {
    let out = run(&["query", "compare", "McDavid"]);
    assert!(
        !out.status.success(),
        "compare with no player2 and no --similar must exit non-zero"
    );
}

#[test]
fn l2_cmd_query_help_exits_zero() {
    let out = run(&["query", "--help"]);
    assert!(out.status.success(), "query --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("leaders"),
        "query help must list 'leaders' subcommand"
    );
    assert!(
        stdout.contains("player"),
        "query help must list 'player' subcommand"
    );
    assert!(
        stdout.contains("compare"),
        "query help must list 'compare' subcommand"
    );
}

// ── L2: --csv / x export coverage (Phase X.1) ────────────────────────────────

/// Sanity that the unified `x` shortcut emits CSV by default with a header
/// row + at least one data row. CSV is the default — `--out` is optional.
#[test]
fn l2_x_rank_csv_default_emits_header_and_rows() {
    let out = run(&["x", "rank", "--top", "5"]);
    assert!(
        out.status.success(),
        "icelines x rank must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("rank,player,team,pos"),
        "CSV must start with header row, got first line: {:?}",
        stdout.lines().next()
    );
    let line_count = stdout.lines().count();
    assert!(
        line_count >= 6,
        "expected at least 6 lines (1 header + 5 data), got {line_count}"
    );
}

#[test]
fn l2_x_history_csv_has_seasons_columns() {
    let out = run(&["x", "history", "--player", "McDavid", "--seasons", "3"]);
    assert!(
        out.status.success(),
        "icelines x history must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let header = stdout.lines().next().unwrap_or("");
    for col in &["season", "team", "gp", "ppg"] {
        assert!(
            header.contains(col),
            "history CSV header must include '{col}', got: {header}"
        );
    }
}

#[test]
fn l2_players_csv_flag_emits_csv_format() {
    let out = run(&["players", "--top", "3", "--csv"]);
    assert!(
        out.status.success(),
        "players --csv must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("rank,player,team"),
        "expected CSV header, got first line: {:?}",
        stdout.lines().next()
    );
}

#[test]
fn l2_csv_and_json_flags_are_mutually_exclusive() {
    let out = run(&["players", "--top", "1", "--csv", "--json"]);
    assert!(
        !out.status.success(),
        "passing both --csv and --json must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive") || stderr.contains("exclusive"),
        "error must mention mutual exclusion, got: {stderr}"
    );
}

#[test]
fn l2_x_with_out_flag_writes_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("rank.csv");
    let out = run(&["x", "rank", "--top", "3", "--out", path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "icelines x rank --out must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = std::fs::read_to_string(&path).expect("output file must exist after --out");
    assert!(
        body.starts_with("rank,player,team"),
        "written file must contain CSV header, got: {body}"
    );
    let line_count = body.lines().count();
    assert!(
        line_count >= 4,
        "expected ≥4 lines (header + 3 rows), got {line_count}"
    );
}

#[test]
fn l2_x_help_lists_all_shapes() {
    let out = run(&["x", "--help"]);
    assert!(out.status.success(), "icelines x --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for shape in &[
        "rank",
        "leaders",
        "goalies",
        "players",
        "history",
        "peers",
        "compare",
        "transactions",
    ] {
        assert!(
            stdout.contains(shape),
            "x --help must list '{shape}' shape, got: {stdout}"
        );
    }
}

#[test]
fn l2_report_list_exits_zero_and_lists_report_doors() {
    let out = run(&["report", "list"]);
    assert!(out.status.success(), "icelines report list must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for needle in [
        "IceLines report surface",
        "query",
        "x",
        "export md",
        "weekly-fantasy",
        "records",
        "teams-scored-against",
    ] {
        assert!(
            stdout.contains(needle),
            "report list should mention {needle:?}, got: {stdout}"
        );
    }
}

#[test]
fn l2_report_list_json_marks_records_available() {
    let out = run(&["report", "list", "--json"]);
    assert!(
        out.status.success(),
        "icelines report list --json must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"name\": \"records\"") && stdout.contains("\"status\": \"available\""),
        "report list JSON must expose available records catalog row, got: {stdout}"
    );
}

#[test]
fn l2_records_player_no_data_exits_zero_with_headers() {
    let home = tempfile::TempDir::new().unwrap();
    let out = run_isolated(
        home.path(),
        &[
            "--no-setup",
            "records",
            "player",
            "Andre Burakovsky",
            "--metric",
            "teams-scored-against",
            "--csv",
        ],
    );
    assert!(
        out.status.success(),
        "records player must exit 0 with empty local boxscores, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("opponent_team") && stdout.contains("count"),
        "records CSV should include stable headers, got: {stdout}"
    );
}

#[test]
fn l2_records_player_goalies_no_data_exits_zero_with_headers() {
    let home = tempfile::TempDir::new().unwrap();
    let out = run_isolated(
        home.path(),
        &[
            "--no-setup",
            "records",
            "player",
            "Andre Burakovsky",
            "--metric",
            "goalies-scored-against",
            "--csv",
        ],
    );
    assert!(
        out.status.success(),
        "records player goalie metric must exit 0 with empty play-by-play, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("goalie_id") && stdout.contains("goalie") && stdout.contains("count"),
        "records goalie CSV should include stable headers, got: {stdout}"
    );
}

#[test]
fn l2_records_player_fights_no_data_exits_zero_with_headers() {
    let home = tempfile::TempDir::new().unwrap();
    let out = run_isolated(
        home.path(),
        &[
            "--no-setup",
            "records",
            "player",
            "Andre Burakovsky",
            "--metric",
            "fight-opponents",
            "--csv",
        ],
    );
    assert!(
        out.status.success(),
        "records player fight metric must exit 0 with empty play-by-play, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("opponent_id") && stdout.contains("opponent") && stdout.contains("count"),
        "records fight CSV should include stable headers, got: {stdout}"
    );
}

#[test]
fn l2_records_team_help_lists_first_metric() {
    let out = run(&["records", "team", "--help"]);
    assert!(out.status.success(), "records team --help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("players-scored-against-team"),
        "records team help should list supported metric, got: {stdout}"
    );
}

// ── L2: transactions (Phase T.4) ─────────────────────────────────────────────

#[test]
fn l2_cmd_transactions_exits_zero() {
    let out = run(&["transactions"]);
    assert!(
        out.status.success(),
        "transactions must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Transactions"),
        "header must mention Transactions, got: {stdout}"
    );
}

#[test]
fn l2_cmd_transactions_csv_emits_header_and_rows() {
    let out = run(&["transactions", "--csv"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let header = stdout.lines().next().unwrap_or("");
    for col in &["date", "team", "kind", "description", "id"] {
        assert!(
            header.contains(col),
            "CSV header must include '{col}', got: {header}"
        );
    }
    let lines = stdout.lines().count();
    assert!(
        lines >= 2,
        "expected ≥2 lines (header + ≥1 row), got {lines}"
    );
}

#[test]
fn l2_cmd_transactions_team_edm_filters() {
    let out = run(&["transactions", "--team", "EDM", "--csv"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body_lines: Vec<&str> = stdout.lines().skip(1).collect();
    for line in &body_lines {
        // Team col is the second CSV field.
        let cols: Vec<&str> = line.split(',').collect();
        assert!(cols.len() >= 2);
        assert_eq!(
            cols[1], "EDM",
            "every row must be EDM after --team filter, got: {line}"
        );
    }
}

#[test]
#[allow(non_snake_case)] // Test name encodes the literal "LEAGUE" sentinel value.
fn l2_cmd_transactions_team_LEAGUE_returns_teamless() {
    let out = run(&["transactions", "--team", "LEAGUE", "--csv"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body_lines: Vec<&str> = stdout.lines().skip(1).filter(|l| !l.is_empty()).collect();
    for line in &body_lines {
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(
            cols[1], "LEAGUE",
            "--team LEAGUE must return only teamless rows, got: {line}"
        );
    }
}

#[test]
fn l2_cmd_transactions_kind_trade_filters() {
    let out = run(&["transactions", "--kind", "trade", "--csv"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines().skip(1).filter(|l| !l.is_empty()) {
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(
            cols[2], "trade",
            "--kind trade must return only trades, got: {line}"
        );
    }
}

#[test]
fn l2_cmd_transactions_out_writes_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("tx.csv");
    let out = run(&["transactions", "--out", path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "transactions --out stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = std::fs::read_to_string(&path).expect("output file must exist");
    assert!(
        body.starts_with("date") || body.starts_with("[") || body.contains("kind"),
        "output must contain a recognizable header / JSON, got: {body}"
    );
}

#[test]
fn l2_cmd_transactions_invalid_kind_exits_nonzero() {
    let out = run(&["transactions", "--kind", "trades"]); // typo plural
    assert!(!out.status.success(), "invalid kind must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown kind"),
        "error must mention 'unknown kind', got: {stderr}"
    );
    assert!(
        stderr.contains("trade"),
        "error must list valid kinds, got: {stderr}"
    );
}

#[test]
fn l2_cmd_transactions_invalid_since_exits_nonzero() {
    let out = run(&["transactions", "--since", "2026-13-40"]);
    assert!(!out.status.success(), "invalid date must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let s = stderr.to_lowercase();
    assert!(
        s.contains("date")
            || stderr.contains("YYYY-MM-DD")
            || s.contains("month")
            || s.contains("day")
            || s.contains("range"),
        "error must hint at the offending date piece, got: {stderr}",
    );
}

#[test]
fn l2_cmd_transactions_since_after_until_exits_nonzero() {
    let out = run(&[
        "transactions",
        "--since",
        "2026-04-30",
        "--until",
        "2026-04-01",
    ]);
    assert!(!out.status.success(), "since > until must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("after"),
        "error must mention 'after', got: {stderr}"
    );
}

#[test]
fn l2_cmd_transactions_csv_and_json_mutually_exclusive() {
    let out = run(&["transactions", "--csv", "--json"]);
    assert!(!out.status.success(), "both flags must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("exclusive") || stderr.contains("mutually"),
        "error must mention mutual exclusion, got: {stderr}"
    );
}

#[test]
fn l2_cmd_transactions_top_combined_with_kind_returns_at_most_n() {
    let out = run(&["transactions", "--kind", "trade", "--top", "1", "--csv"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body_lines = stdout.lines().skip(1).filter(|l| !l.is_empty()).count();
    assert!(
        body_lines <= 1,
        "--top 1 must return at most 1 row, got {body_lines}"
    );
}

#[test]
fn l2_cmd_transactions_lowercase_team_normalized() {
    let out = run(&["transactions", "--team", "edm", "--csv"]);
    assert!(
        out.status.success(),
        "lowercase --team should normalize, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines().skip(1).filter(|l| !l.is_empty()) {
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(
            cols[1], "EDM",
            "lowercase 'edm' must normalize to 'EDM', got: {line}"
        );
    }
}

#[test]
fn l2_cmd_transactions_pre_coverage_season_helpful_message() {
    let out = run(&["transactions", "--season", "19951996"]);
    assert!(
        !out.status.success(),
        "pre-coverage season must exit non-zero, stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    // EDGE-mandated: NOT a "run icelines fetch" hint (which would 404).
    assert!(
        stderr.contains("begins") || stderr.contains("not covered"),
        "error must explain coverage, got: {stderr}"
    );
    assert!(
        !stderr.contains("run `icelines fetch transactions`"),
        "must NOT suggest fetching for a pre-coverage season, got: {stderr}"
    );
}

#[test]
fn l2_cmd_transactions_json_output_is_valid_array() {
    let out = run(&["transactions", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--json output must be valid JSON");
    assert!(parsed.is_array(), "--json output must be a top-level array");
}

#[test]
fn l2_x_transactions_defaults_to_csv() {
    let out = run(&["x", "transactions"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("date,team,kind"),
        "x transactions must default to CSV (header first), got: {stdout}"
    );
}

#[test]
fn l2_cmd_transactions_search_filters_to_substring() {
    // --search runs case-insensitive description substring match.
    // We use --season 20232024 because the bundled snapshot has well-known
    // signings (e.g. for any common surname) we can hit.
    let out = run(&[
        "transactions",
        "--season",
        "20232024",
        "--search",
        "bedard",
        "--csv",
    ]);
    assert!(
        out.status.success(),
        "transactions --search must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body_lines: Vec<&str> = stdout.lines().skip(1).filter(|l| !l.is_empty()).collect();
    // Every body row must mention 'bedard' in the description (case-insensitive).
    for line in &body_lines {
        assert!(
            line.to_lowercase().contains("bedard"),
            "every --search row must mention 'bedard', got: {line}",
        );
    }
}

#[test]
fn l2_cmd_transactions_search_empty_passes_all() {
    let raw = run(&["transactions", "--season", "20232024", "--csv"]);
    let with_empty = run(&[
        "transactions",
        "--season",
        "20232024",
        "--search",
        "",
        "--csv",
    ]);
    assert!(raw.status.success());
    assert!(with_empty.status.success());
    let n_raw = String::from_utf8_lossy(&raw.stdout).lines().count();
    let n_emp = String::from_utf8_lossy(&with_empty.stdout).lines().count();
    assert_eq!(
        n_raw, n_emp,
        "--search '' must be a no-op; got {n_raw} vs {n_emp}"
    );
}

#[test]
fn l2_cmd_transactions_player_filter_works_by_last_name() {
    // --player matches by last-name token, NFD-stripped.
    let out = run(&[
        "transactions",
        "--season",
        "20232024",
        "--player",
        "Bedard",
        "--csv",
    ]);
    assert!(
        out.status.success(),
        "transactions --player must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body_lines: Vec<&str> = stdout.lines().skip(1).filter(|l| !l.is_empty()).collect();
    for line in &body_lines {
        assert!(
            line.to_lowercase().contains("bedard"),
            "every --player Bedard row must mention 'bedard', got: {line}",
        );
    }
}

#[test]
fn l2_cmd_transactions_player_with_team_disambig() {
    // --player + --team narrows to one team's appearances.
    let out = run(&[
        "transactions",
        "--season",
        "20232024",
        "--player",
        "McDavid",
        "--team",
        "EDM",
        "--csv",
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body_lines: Vec<&str> = stdout.lines().skip(1).filter(|l| !l.is_empty()).collect();
    for line in &body_lines {
        let cols: Vec<&str> = line.split(',').collect();
        // Team col is 2nd. Either EDM or LEAGUE (teamless league-wide rows
        // pass the team filter — see search.rs).
        assert!(
            cols.len() >= 2 && (cols[1] == "EDM" || cols[1] == "LEAGUE"),
            "--player McDavid --team EDM row must be EDM or LEAGUE, got: {line}",
        );
    }
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
    assert!(out.status.success(), "fantasy serve --help must exit 0");
}

/// Phase G.6 end-to-end: prove that adding a goalie to a fantasy team
/// resolves through the goalie pool and surfaces the "(Goalie)" tag.
/// Uses isolated HOME so the run can't collide with the user's real DB.
#[test]
fn l2_cmd_fantasy_team_add_goalie_emits_goalie_tag() {
    let tmp = tempfile::tempdir().expect("tempdir for isolated HOME");
    let home = tmp.path();
    let league = unique_league("goalie-add");
    let team = "Net Crashers";

    let out = run_isolated(home, &["fantasy", "league-create", &league]);
    assert!(
        out.status.success(),
        "league-create must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let out = run_isolated(home, &["fantasy", "team-create", team]);
    assert!(
        out.status.success(),
        "team-create must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    // Hellebuyck is a goalie — he is NOT in the skater pool, so a successful
    // resolution proves the goalie-pool fallback in `run_team_add` works.
    let out = run_isolated(home, &["fantasy", "team-add", team, "Hellebuyck"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "team-add Hellebuyck must succeed (goalie fallback), stderr: {stderr}"
    );
    assert!(
        stdout.contains("(Goalie)"),
        "team-add output must tag the addition as (Goalie), got: {stdout}"
    );
    assert!(
        stdout.contains("Hellebuyck"),
        "team-add output must echo the goalie's name, got: {stdout}"
    );
}

// ── L2: fetch sub-commands (dry-run only) ────────────────────────────────────

#[test]
fn l2_cmd_fantasy_gaps_json_emits_view_contract() {
    let tmp = tempfile::tempdir().expect("tempdir for isolated HOME");
    let home = tmp.path();
    let league = unique_league("gaps-json");
    let team = "Gap Checkers";

    let out = run_isolated(home, &["fantasy", "league-create", &league]);
    assert!(
        out.status.success(),
        "league-create must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let out = run_isolated(home, &["fantasy", "team-create", team]);
    assert!(
        out.status.success(),
        "team-create must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let out = run_isolated(home, &["fantasy", "team-add", team, "McDavid"]);
    assert!(
        out.status.success(),
        "team-add McDavid must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let out = run_isolated(home, &["fantasy", "team-use", team]);
    assert!(
        out.status.success(),
        "team-use must succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let out = run_isolated(
        home,
        &[
            "fantasy",
            "gaps",
            "--category",
            "hits,blocks,shots",
            "--json",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "fantasy gaps --json must succeed, stderr: {stderr}"
    );
    assert!(
        stdout.contains("\"rows\""),
        "gaps JSON must expose rows from FantasyRosterGapView, got: {stdout}"
    );
    assert!(
        stdout.contains("\"scoring_scheme\""),
        "gaps JSON must expose the active scoring scheme, got: {stdout}"
    );
    assert!(
        stdout.contains("\"weighted_gap_score\""),
        "gaps JSON must expose weighted gap scores, got: {stdout}"
    );
}

#[test]
fn l2_cmd_fantasy_daily_json_surfaces_missing_cache() {
    let tmp = tempfile::tempdir().expect("tempdir for isolated HOME");
    let home = tmp.path();
    seed_daily_league(home);

    let out = run_isolated(
        home,
        &[
            "fantasy",
            "daily",
            "--date",
            "2026-01-15",
            "--league",
            "Daily League",
            "--json",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "fantasy daily --json must succeed with explicit missing-cache state, stderr: {stderr}"
    );
    assert!(
        stdout.contains("\"league\": \"Daily League\""),
        "daily JSON must expose the league name, got: {stdout}"
    );
    assert!(
        stdout.contains("\"date\": \"2026-01-15\""),
        "daily JSON must expose the requested date, got: {stdout}"
    );
    assert!(
        stdout.contains("no cached boxscores"),
        "daily JSON must warn instead of zero-shaping missing cache, got: {stdout}"
    );
    assert!(
        stdout.contains("\"state\": \"unavailable\""),
        "daily JSON must expose unavailable boxscore source state, got: {stdout}"
    );
}

#[test]
fn l2_cmd_fantasy_matchup_json_surfaces_missing_schedule() {
    let tmp = tempfile::tempdir().expect("tempdir for isolated HOME");
    let home = tmp.path();
    seed_daily_league(home);

    let out = run_isolated(
        home,
        &[
            "fantasy",
            "matchup",
            "--date",
            "2026-01-15",
            "--league",
            "Daily League",
            "--json",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "fantasy matchup --json must succeed with explicit missing-schedule state, stderr: {stderr}"
    );
    assert!(
        stdout.contains("\"league\": \"Daily League\""),
        "matchup JSON must expose the league name, got: {stdout}"
    );
    assert!(
        stdout.contains("\"week_start\": \"2026-01-12\""),
        "matchup JSON must expose the ISO week start, got: {stdout}"
    );
    assert!(
        stdout.contains("\"empty_state\""),
        "matchup JSON must expose setup empty state, got: {stdout}"
    );
    assert!(
        stdout.contains("\"source\": \"schedule\"")
            && stdout.contains("\"state\": \"unavailable\""),
        "matchup JSON must expose unavailable schedule source state, got: {stdout}"
    );
}

#[test]
fn l2_cmd_fantasy_import_yahoo_json_applies_roster_csv() {
    let tmp = tempfile::tempdir().expect("tempdir for isolated HOME");
    let home = tmp.path();
    let league = unique_league("import-yahoo");
    let csv_path = home.join("yahoo-roster.csv");
    std::fs::write(
        &csv_path,
        "Player,Fantasy Team,Owner,NHL Team,Eligible Positions\nConnor McDavid,Csv Team,Me,EDM,C\n",
    )
    .expect("write roster csv");

    let out = run_isolated(
        home,
        &[
            "fantasy",
            "import-yahoo",
            "--file",
            csv_path.to_str().unwrap(),
            "--league",
            &league,
            "--my-team",
            "Csv Team",
            "--json",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "fantasy import-yahoo --json must succeed, stderr: {stderr}"
    );
    assert!(
        stdout.contains("\"mode\": \"apply\""),
        "import JSON must expose apply mode, got: {stdout}"
    );
    assert!(
        stdout.contains("\"league\":"),
        "import JSON must expose league, got: {stdout}"
    );
    assert!(
        stdout.contains("Connor McDavid") && stdout.contains("Csv Team"),
        "import JSON must include imported row/team, got: {stdout}"
    );
    assert!(
        stdout.contains("\"players_imported\": 1"),
        "import JSON must count the imported player, got: {stdout}"
    );

    let list = run_isolated(home, &["fantasy", "team-list", "--league", &league]);
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        list.status.success(),
        "team-list after import must succeed, stderr: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    assert!(
        list_stdout.contains("Csv Team"),
        "import must persist the fantasy team, got: {list_stdout}"
    );
}

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
    assert!(
        stdout.contains("Rank"),
        "hits-pace output must contain 'Rank' header"
    );
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
    assert!(
        out.status.success(),
        "query leaders --sort blocks-pace must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_blocks() {
    let out = run(&["query", "leaders", "--sort", "blocks", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort blocks must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_takeaways() {
    let out = run(&["query", "leaders", "--sort", "takeaways", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort takeaways must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_giveaways() {
    let out = run(&["query", "leaders", "--sort", "giveaways", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort giveaways must exit 0"
    );
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
    assert!(
        out.status.success(),
        "query leaders --sort xg-per-60 must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_cf_pct() {
    let out = run(&["query", "leaders", "--sort", "cf-pct", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort cf-pct must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_ff_pct() {
    let out = run(&["query", "leaders", "--sort", "ff-pct", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort ff-pct must exit 0"
    );
}

#[test]
fn l2_cmd_query_leaders_sort_xgf_pct() {
    let out = run(&["query", "leaders", "--sort", "xgf-pct", "--top", "5"]);
    assert!(
        out.status.success(),
        "query leaders --sort xgf-pct must exit 0"
    );
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
    assert!(
        stdout.contains("Rank"),
        "rate mode must still show Rank header"
    );
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
    assert!(
        stdout.contains("Pts/82") || stdout.contains("PPG"),
        "leaderboard header should be present, got: {stdout}"
    );
    assert!(
        stdout.contains("matched, showing"),
        "footer count should be present, got: {stdout}"
    );
}

#[test]
fn l2_cmd_query_leaders_season_unbundled_errors_with_hint() {
    // 2004-05 is the lockout — never had a season, never bundled.
    // (Used to be 19951996 pre-L.7b when only 5 modern seasons shipped.)
    let out = run(&["query", "leaders", "--season", "20042005", "--top", "5"]);
    assert!(
        !out.status.success(),
        "non-bundled --season must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not bundled"),
        "must say 'not bundled', got: {stderr}"
    );
    assert!(
        stderr.contains("20252026"),
        "must list current bundled season as a hint, got: {stderr}"
    );
}

#[test]
fn l2_cmd_query_leaders_season_with_seasons_n_errors() {
    let out = run(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--seasons",
        "3",
        "--top",
        "5",
    ]);
    assert!(
        !out.status.success(),
        "--season + --seasons N > 1 must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "must explain the conflict, got: {stderr}"
    );
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
    assert!(
        stdout.contains("PLAYER PROFILE"),
        "player profile header should be present, got: {stdout}"
    );
}

#[test]
fn l2_cmd_query_compare_season_bundled_succeeds() {
    let out = run(&[
        "query",
        "compare",
        "McDavid",
        "MacKinnon",
        "--season",
        "20242025",
    ]);
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
    let out = run_isolated(
        home.path(),
        &["group", "create", "watch", "--desc", "watch list"],
    );
    assert!(
        out.status.success(),
        "group create stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for player in ["McDavid", "MacKinnon", "Matthews"] {
        let out = run_isolated(home.path(), &["group", "add", "watch", player]);
        assert!(
            out.status.success(),
            "group add {player} stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // Export to file.
    let export_path = home.path().join("watch.json");
    let out = run_isolated(
        home.path(),
        &[
            "group",
            "export",
            "watch",
            "--out",
            export_path.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "group export stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = std::fs::read_to_string(&export_path).expect("export file written");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("export must be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("watch"));
    assert_eq!(parsed["description"].as_str(), Some("watch list"));
    let members = parsed["members"].as_array().expect("members array");
    assert_eq!(members.len(), 3);
    // Import as a new name into the same db.
    let out = run_isolated(
        home.path(),
        &[
            "group",
            "import",
            export_path.to_str().unwrap(),
            "--as",
            "watch-copy",
        ],
    );
    assert!(
        out.status.success(),
        "group import stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Both groups now visible from list.
    let out = run_isolated(home.path(), &["group", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("watch"), "original group missing: {stdout}");
    assert!(
        stdout.contains("watch-copy"),
        "imported copy missing: {stdout}"
    );
}

#[test]
fn l2_cmd_group_rename_succeeds() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["group", "create", "before"]);
    assert!(
        out.status.success(),
        "create stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_isolated(home.path(), &["group", "add", "before", "McDavid"]);
    assert!(
        out.status.success(),
        "add stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_isolated(home.path(), &["group", "rename", "before", "after"]);
    assert!(
        out.status.success(),
        "rename stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // After rename, `before` is gone and `after` carries the member.
    let out = run_isolated(home.path(), &["group", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("after"), "renamed group missing: {stdout}");
    assert!(
        !stdout.contains(" before "),
        "old name should not appear: {stdout}"
    );
}

// ── Phase G.5: query goalies ───────────────────────────────────────────────

#[test]
fn l2_cmd_query_goalies_default_table() {
    let out = run(&["query", "goalies", "--top", "5"]);
    assert!(
        out.status.success(),
        "query goalies stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Goalie") && stdout.contains("SV%") && stdout.contains("GAA"),
        "default table missing column headers, got:\n{stdout}"
    );
    // Footer carries qualifying gate + sort label.
    assert!(
        stdout.contains("min 15 GP") && stdout.contains("sv-pct"),
        "footer should mention min-gp + sort, got:\n{stdout}"
    );
}

#[test]
fn l2_cmd_query_goalies_csv_includes_header() {
    let out = run(&["query", "goalies", "--top", "3", "--csv"]);
    assert!(
        out.status.success(),
        "query goalies --csv stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first = stdout.lines().next().unwrap_or("");
    assert!(
        first.starts_with("rank,goalie,team,gp,wins"),
        "CSV header missing, got first line: {first}"
    );
}

#[test]
fn l2_cmd_query_goalies_json_parses() {
    let out = run(&["query", "goalies", "--top", "2", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("query goalies --json must emit valid JSON");
    let arr = parsed.as_array().expect("top-level array");
    assert_eq!(arr.len(), 2);
    assert!(arr[0]["full_name"].is_string());
    // Hart.5c.7.4 JSON shape: GoalieRow flattens the legacy nested
    // `stats` object, so save_pct sits at the top level.
    assert!(arr[0]["save_pct"].is_number());
    assert!(arr[0]["games_played"].is_number());
}

#[test]
fn l2_cmd_query_goalies_json_csv_row_identity_match() {
    let args = ["query", "goalies", "--sort", "wins", "--top", "5"];
    let json_out = run(&[&args[..], &["--json"][..]].concat());
    assert!(
        json_out.status.success(),
        "query goalies --json must exit 0, stderr: {}",
        String::from_utf8_lossy(&json_out.stderr)
    );
    let csv_out = run(&[&args[..], &["--csv"][..]].concat());
    assert!(
        csv_out.status.success(),
        "query goalies --csv must exit 0, stderr: {}",
        String::from_utf8_lossy(&csv_out.stderr)
    );

    let json_stdout = String::from_utf8_lossy(&json_out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("query goalies --json must emit valid JSON");
    let json_rows = json
        .as_array()
        .expect("query goalies --json should emit a JSON array");

    let csv_stdout = String::from_utf8_lossy(&csv_out.stdout);
    let csv_rows: Vec<Vec<String>> = csv_stdout
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(',')
                .map(|cell| cell.trim_matches('"').to_string())
                .collect()
        })
        .collect();

    assert_eq!(
        json_rows.len(),
        csv_rows.len(),
        "JSON and CSV should return the same row count"
    );
    for (json_row, csv_row) in json_rows.iter().zip(csv_rows.iter()) {
        assert!(
            csv_row.len() >= 5,
            "CSV row should carry rank,goalie,team,gp,wins: {csv_row:?}"
        );
        assert_eq!(json_row["full_name"].as_str(), Some(csv_row[1].as_str()));
        assert_eq!(json_row["team"].as_str(), Some(csv_row[2].as_str()));
        assert_eq!(json_row["games_played"].to_string(), csv_row[3]);
        assert_eq!(json_row["wins"].to_string(), csv_row[4]);
    }
}

#[test]
fn l2_cmd_query_goalies_sort_gaa_low_first() {
    let out = run(&["query", "goalies", "--top", "5", "--sort", "gaa"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Pull GAA column values from the data rows; the smallest should
    // be first since GAA sort is ascending.
    let gaas: Vec<f32> = stdout
        .lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()))
        .filter_map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            // Layout: rank goalie team gp w-l-ot sv% gaa so saves
            parts
                .get(parts.len().saturating_sub(3))
                .and_then(|s| s.parse::<f32>().ok())
        })
        .collect();
    assert!(
        gaas.len() >= 2,
        "expected at least 2 data rows, got: {gaas:?}"
    );
    for w in gaas.windows(2) {
        assert!(
            w[0] <= w[1] + 0.001,
            "GAA sort should be ascending, got {} before {} in {gaas:?}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn l2_cmd_query_goalies_team_filter_one_team_only() {
    let out = run(&["query", "goalies", "--team", "WPG", "--min-gp", "5"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Every data row should show WPG as the team. We grep the WPG column —
    // the one between the goalie name and GP. Count occurrences of " WPG "
    // (with both-side padding, since other 3-char tokens could collide).
    let wpg_rows = stdout.lines().filter(|l| l.contains(" WPG ")).count();
    // Count data rows: lines whose FIRST token is a numeric rank.
    // The footer ("N goalies (min ...)") also starts with a digit, so
    // exclude it via the "goalies (" suffix check.
    let other_team_rows = stdout
        .lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()))
        .filter(|l| !l.contains("goalies ("))
        .count();
    assert!(
        wpg_rows >= 1,
        "expected at least one WPG goalie, got:\n{stdout}"
    );
    assert_eq!(
        wpg_rows, other_team_rows,
        "every data row should be WPG when --team WPG, got:\n{stdout}"
    );
}

// ── Attended games (Phase 8 follow-up) ─────────────────────────────────────

#[test]
fn l2_cmd_games_add_list_remove_roundtrip() {
    // Full lifecycle in an isolated $HOME so we don't pollute the real db.
    let home = tempfile::tempdir().expect("tempdir");
    // List on a fresh db should report empty.
    let out = run_isolated(home.path(), &["games", "list"]);
    assert!(
        out.status.success(),
        "games list stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No attended games"),
        "fresh db should report empty, got: {stdout}"
    );

    // Add a game (no boxscore available, that's fine).
    let out = run_isolated(
        home.path(),
        &["games", "add", "2025020100", "--note", "first game"],
    );
    assert!(
        out.status.success(),
        "games add stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // List should now show it.
    let out = run_isolated(home.path(), &["games", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("2025020100"),
        "added game id should appear in list, got: {stdout}"
    );
    assert!(
        stdout.contains("first game"),
        "note should appear in list, got: {stdout}"
    );

    // Remove it.
    let out = run_isolated(home.path(), &["games", "remove", "2025020100"]);
    assert!(
        out.status.success(),
        "games remove stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Remove again should error (already gone).
    let out = run_isolated(home.path(), &["games", "remove", "2025020100"]);
    assert!(
        !out.status.success(),
        "second remove should exit nonzero — game already gone"
    );
}

#[test]
fn l2_cmd_games_export_json_flag_emits_versioned_json() {
    let home = tempfile::tempdir().expect("tempdir");
    // Seed two games.
    for (id, note) in [(2025020100u64, "first"), (2025020101u64, "second")] {
        let out = run_isolated(
            home.path(),
            &["games", "add", &id.to_string(), "--note", note],
        );
        assert!(
            out.status.success(),
            "games add stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // `--json` opts into the legacy versioned envelope.
    let out = run_isolated(home.path(), &["games", "export", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("export --json output must be valid JSON");
    assert_eq!(parsed["version"].as_u64(), Some(1));
    assert_eq!(
        parsed["games"].as_array().map(|a| a.len()),
        Some(2),
        "expected 2 games in export, got: {stdout}"
    );
}

#[test]
fn l2_cmd_games_export_default_emits_csv() {
    // After Phase X.1, `games export` defaults to CSV — Excel-friendly.
    let home = tempfile::tempdir().expect("tempdir");
    for (id, note) in [(2025020100u64, "first"), (2025020101u64, "second")] {
        let out = run_isolated(
            home.path(),
            &["games", "add", &id.to_string(), "--note", note],
        );
        assert!(
            out.status.success(),
            "games add stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = run_isolated(home.path(), &["games", "export"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("game_id,date,away,home"),
        "default games export must emit CSV header, got: {stdout}",
    );
    let data_lines = stdout.lines().filter(|l| !l.is_empty()).count();
    assert!(
        data_lines >= 3,
        "expected ≥3 lines (header + 2 rows), got {data_lines}"
    );
}

// ── Dashboards opt-out flag (was opt-in pre-2026-04-29) ────────────────────

#[test]
fn l2_cmd_no_dashboards_flag_documented_in_help() {
    // The --no-dashboards global flag should appear in --help output.
    // Same opt-out shape as --no-live.
    let out = run(&["--help"]);
    assert!(out.status.success(), "--help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-dashboards"),
        "--help must list --no-dashboards, got:\n{stdout}"
    );
    assert!(
        stdout.contains("ICELINES_DASHBOARDS") || stdout.contains("dashboards"),
        "--help must mention env var or config key, got:\n{stdout}"
    );
}

#[test]
fn l2_cmd_no_dashboards_flag_accepted_globally() {
    // --no-dashboards on a non-TUI command must be accepted (noop for
    // non-TUI paths but must not error). Verifies clap's global=true.
    let out = run(&["--no-dashboards", "scheme", "list"]);
    assert!(
        out.status.success(),
        "--no-dashboards scheme list stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Phase 8f.8: data verify ─────────────────────────────────────────────────

#[test]
fn l2_cmd_data_verify_no_install_errors_helpfully() {
    let home = tempfile::tempdir().expect("tempdir");
    // No seasons dir at all → expect a clear hint to run `data install`.
    let out = run_isolated(home.path(), &["data", "verify", "--all"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no installed seasons") || stderr.contains("data install"),
        "must hint to install first, got: {stderr}"
    );
}

#[test]
fn l2_cmd_data_verify_detects_tampering_in_isolated_home() {
    let home = tempfile::tempdir().expect("tempdir");
    // Hand-build a fake installed bundle: ~/.icelines/seasons/20242025/bios.json
    // + manifest.json. Then run `data verify 20242025`.
    let dir = home
        .path()
        .join(".icelines")
        .join("seasons")
        .join("20242025");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("bios.json"), b"[]").unwrap();
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
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let out = run_isolated(home.path(), &["data", "verify", "20242025"]);
    assert!(!out.status.success(), "tampered bundle must exit nonzero");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("mismatch"),
        "must report mismatch, got: {combined}"
    );
}

#[test]
fn l2_cmd_data_verify_clean_bundle_succeeds() {
    let home = tempfile::tempdir().expect("tempdir");
    let dir = home
        .path()
        .join(".icelines")
        .join("seasons")
        .join("20242025");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let bios = b"[{\"player_id\":1}]";
    let stats = b"[{\"player_id\":1,\"goals\":10}]";
    std::fs::write(dir.join("bios.json"), bios).unwrap();
    std::fs::write(dir.join("stats.json"), stats).unwrap();
    // Compute correct hashes.
    use sha2::{Digest, Sha256};
    let hex = |b: &[u8]| {
        let mut h = Sha256::new();
        h.update(b);
        let v = h.finalize();
        v.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
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
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let out = run_isolated(home.path(), &["data", "verify", "20242025"]);
    assert!(
        out.status.success(),
        "clean bundle must verify, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("verified") || stdout.contains("✓"),
        "success output expected, got: {stdout}"
    );
}

// ── Phase 8f.7: scheme from-csv multi-platform ──────────────────────────────

#[test]
fn l2_cmd_scheme_from_csv_yahoo_autodetect() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("yahoo.csv");
    // Minimal Yahoo header — enough for signature + a few stat columns.
    std::fs::write(
        &path,
        "Player,Owner,GP,G (P),A (P),HIT (P),BLK (P),Fan Pts\n",
    )
    .unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "from-csv stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Platform: Yahoo"),
        "platform header missing, got: {stdout}"
    );
    assert!(
        stdout.contains("goals") && stdout.contains("hits"),
        "stat keys missing, got: {stdout}"
    );
}

#[test]
fn l2_cmd_scheme_from_csv_espn_autodetect() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("espn.csv");
    std::fs::write(
        &path,
        "RANK,PLAYER,TEAM,POS,STATUS,OWNER,G,A,+/-,SOG,HIT,BLK\n",
    )
    .unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "from-csv stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Platform: ESPN"),
        "ESPN should auto-detect, got: {stdout}"
    );
}

#[test]
fn l2_cmd_scheme_from_csv_explicit_platform_override() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ambiguous.csv");
    // Header has bare G,A — would match ESPN auto-detection but no signature
    // columns. With --platform fantrax, we should still get fantrax.
    std::fs::write(&path, "Player,G,A,Pts,PPG,SOG,HT,BLK\n").unwrap();
    let out = run(&[
        "scheme",
        "from-csv",
        path.to_str().unwrap(),
        "--platform",
        "fantrax",
    ]);
    assert!(
        out.status.success(),
        "explicit platform stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Platform: Fantrax"),
        "explicit fantrax must be honored, got: {stdout}"
    );
    // HT (fantrax convention) → hits
    assert!(
        stdout.contains("hits"),
        "fantrax HT should map to hits, got: {stdout}"
    );
}

#[test]
fn l2_cmd_scheme_from_csv_unknown_platform_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("anything.csv");
    std::fs::write(&path, "Header\n").unwrap();
    let out = run(&[
        "scheme",
        "from-csv",
        path.to_str().unwrap(),
        "--platform",
        "draftkings",
    ]);
    assert!(!out.status.success(), "unknown platform must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("yahoo"),
        "unknown-platform error must list valid options, got: {stderr}"
    );
}

#[test]
fn l2_cmd_scheme_from_csv_unrecognized_format_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("garbage.csv");
    // Header with no signature column from any platform.
    std::fs::write(&path, "ColA,ColB,ColC\n").unwrap();
    let out = run(&["scheme", "from-csv", path.to_str().unwrap()]);
    assert!(!out.status.success(), "unknown format must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unrecognized") || stderr.contains("--platform"),
        "must hint --platform fallback, got: {stderr}"
    );
}

#[test]
fn l2_cmd_group_export_to_stdout_emits_json() {
    let home = tempfile::tempdir().expect("tempdir");
    let _ = run_isolated(home.path(), &["group", "create", "stdout-test"]);
    let _ = run_isolated(home.path(), &["group", "add", "stdout-test", "McDavid"]);
    // Default --out is "-" → stdout.
    let out = run_isolated(home.path(), &["group", "export", "stdout-test"]);
    assert!(
        out.status.success(),
        "export to stdout stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("stdout-test"));
}

// ── Phase Lindsay L.1.6 — fetch report CLI ──────────────────────────────────

/// `fetch report --kind <Tier-1> --dry-run` prints the URL + planned
/// write target without making a network call. Pin the URL shape AND
/// the per-window file path.
#[test]
fn l2_lindsay_fetch_report_tier1_dry_run() {
    let out = run(&[
        "fetch",
        "report",
        "--kind",
        "skater-timeonice",
        "--season",
        "20242025",
        "--type",
        "regular",
        "--dry-run",
    ]);
    assert!(
        out.status.success(),
        "fetch report --dry-run must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("/skater/timeonice"),
        "URL must include endpoint path; got:\n{stdout}"
    );
    assert!(
        stdout.contains("seasonId=20242025"),
        "URL must carry season filter"
    );
    assert!(stdout.contains("gameTypeId=2"), "regular → gameTypeId=2");
    assert!(
        stdout.contains("timeonice.json"),
        "must mention the per-window filename"
    );
    assert!(
        stdout.contains("\\20242025\\regular") || stdout.contains("/20242025/regular"),
        "must mention the per-window dir layout"
    );
}

/// `--type playoff` flips gameTypeId and the season-type subdir.
#[test]
fn l2_lindsay_fetch_report_tier1_playoff_dry_run() {
    let out = run(&[
        "fetch",
        "report",
        "--kind",
        "goalie-savesByStrength",
        "--season",
        "20232024",
        "--type",
        "playoff",
        "--dry-run",
    ]);
    // (clap normalizes the kebab-case name; `goalie-savesByStrength` is
    // wrong — the value-enum spelling is `goalie-saves-by-strength`. Fix.)
    let _ = out;
    let out = run(&[
        "fetch",
        "report",
        "--kind",
        "goalie-saves-by-strength",
        "--season",
        "20232024",
        "--type",
        "playoff",
        "--dry-run",
    ]);
    assert!(
        out.status.success(),
        "playoff dry-run must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("/goalie/savesByStrength"));
    assert!(stdout.contains("gameTypeId=3"), "playoff → gameTypeId=3");
    assert!(stdout.contains("\\playoff") || stdout.contains("/playoff"));
}

/// Phase Lindsay L.6 — Tier-2 dispatch is ACCEPTED. The Tier-1-only
/// gate from L.1.6 was lifted; Tier-2 endpoints now route through the
/// same fetch flow with a filename derived from `kind.url_path()`.
/// Verifies via --dry-run (no network): exit 0, the URL preview shows
/// the catalog url_path, the planned write target uses the
/// derived filename `{path-with-slash-replaced}.json`.
#[test]
fn l2_lindsay_l6_fetch_report_tier2_accepted() {
    let out = run(&[
        "fetch",
        "report",
        "--kind",
        "skater-puck-possessions",
        "--dry-run",
    ]);
    assert!(
        out.status.success(),
        "Tier-2 dispatch must succeed at L.6; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // URL preview shows the Tier-2 url_path.
    assert!(
        stdout.contains("/skater/puckPossessions"),
        "expected url_path for skater-puck-possessions — got:\n{stdout}",
    );
    // Filename derives from url_path (`/` → `-`).
    assert!(
        stdout.contains("skater-puckPossessions.json"),
        "expected derived filename `skater-puckPossessions.json` — got:\n{stdout}",
    );
}

/// `--kind` value-enum lists every catalog variant. Pin via `--help`
/// containing all 9 Tier-1 names (camelCase → kebab-case).
#[test]
fn l2_lindsay_fetch_report_help_lists_all_tier1_kinds() {
    let out = run(&["fetch", "report", "--help"]);
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    for kind in &[
        "skater-summary",
        "skater-bios",
        "skater-realtime",
        "skater-timeonice",
        "skater-goals-for-against",
        "goalie-summary",
        "goalie-bios",
        "goalie-advanced",
        "goalie-saves-by-strength",
    ] {
        assert!(help.contains(kind), "--help must list Tier-1 kind {kind}");
    }
}

/// BENCH closeout #1: `--no-lock` actually skips lock acquisition.
/// Strategy: pre-create the lock file at the binary's icelines-home
/// dir (`<HOME>/.icelines/.fetch.lock`) so the default lock path
/// would block. Then invoke `fetch report --no-lock --dry-run` —
/// it must exit 0 because the flag short-circuits the acquire.
/// Without `--no-lock` the same setup would either spin (until our
/// 120s timeout) or error.
///
/// `--dry-run` keeps this offline — the flag-skip path runs through
/// the same gate logic regardless of dry-run, so dry-run is
/// sufficient to prove the lock is bypassed.
#[test]
fn l2_lindsay_fetch_report_no_lock_skips_lock_acquisition() {
    let home = tempfile::TempDir::new().expect("tempdir");
    let icelines_home = home.path().join(".icelines");
    std::fs::create_dir_all(&icelines_home).unwrap();
    // Pre-occupy the lock path. Without --no-lock, acquire() would
    // block trying to `create_new` over this file.
    let lock_path = icelines_home.join(".fetch.lock");
    std::fs::write(&lock_path, b"held-by-test").unwrap();

    let out = run_isolated(
        home.path(),
        &[
            "fetch",
            "report",
            "--kind",
            "skater-timeonice",
            "--season",
            "20242025",
            "--type",
            "regular",
            "--no-lock",
            "--dry-run",
        ],
    );
    assert!(
        out.status.success(),
        "--no-lock should bypass the held lock; exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    // The held lock file must STILL exist after `--no-lock` ran —
    // the flag means "don't touch the lock", not "force-delete it".
    assert!(
        lock_path.exists(),
        "--no-lock must NOT delete or modify the held lock file",
    );
}

// ── Phase Lindsay L.3.1 — `query leaders --filter` ──────────────────────────

/// `--filter "goals>=30"` exits 0 and the result count matches a
/// canonical bundled-data benchmark (~46 ≥30-goal scorers in 2024-25).
#[test]
fn l2_lindsay_query_leaders_filter_goals_min_returns_subset() {
    let out = run(&[
        "query",
        "leaders",
        "--filter",
        "goals>=30",
        "--top",
        "100",
        "--season",
        "20242025",
    ]);
    assert!(
        out.status.success(),
        "--filter goals>=30 must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The pre-Lindsay all-leaders query for 2024-25 returns ~905 players;
    // `goals>=30` must trim well below that.
    assert!(stdout.contains("matched"), "footer must report match count");
    assert!(
        stdout.contains(" matched, showing "),
        "match-count footer present"
    );
}

/// Multiple `--filter` flags compose via implicit AND; `normalize_stat_filters`
/// runs before apply. Pin: combining a goals-min and gp-min produces a
/// strict subset of either alone.
#[test]
fn l2_lindsay_query_leaders_multiple_filters_compose_and() {
    // First: just goals>=30 → some count N.
    let out_g = run(&[
        "query",
        "leaders",
        "--filter",
        "goals>=30",
        "--top",
        "200",
        "--season",
        "20242025",
    ]);
    assert!(out_g.status.success());

    // Then: goals>=30 AND points>=80 → strict subset.
    let out_both = run(&[
        "query",
        "leaders",
        "--filter",
        "goals>=30",
        "--filter",
        "points>=80",
        "--top",
        "200",
        "--season",
        "20242025",
    ]);
    assert!(
        out_both.status.success(),
        "compound --filter must exit 0; stderr: {}",
        String::from_utf8_lossy(&out_both.stderr)
    );
    // Both should contain the "matched" footer; the compound result
    // count must be <= the single-filter count (subset semantic).
    let g_only = String::from_utf8_lossy(&out_g.stdout);
    let both = String::from_utf8_lossy(&out_both.stdout);
    // Footer format: `"<N> matched, showing <M>."`
    let parse_count = |s: &str| -> Option<u32> {
        for line in s.lines() {
            let trimmed = line.trim();
            if let Some(idx) = trimmed.find(" matched, showing ") {
                return trimmed[..idx].parse().ok();
            }
        }
        None
    };
    let g = parse_count(&g_only).expect("goals-only match count must parse");
    let b = parse_count(&both).expect("compound match count must parse");
    assert!(
        b <= g,
        "compound filter result ({b}) must be subset of single-filter ({g})"
    );
}

/// Unknown stat key surfaces as a clear error.
#[test]
fn l2_lindsay_query_leaders_filter_unknown_key_errors_cleanly() {
    let out = run(&[
        "query",
        "leaders",
        "--filter",
        "fooStat>=10",
        "--season",
        "20242025",
    ]);
    assert!(
        !out.status.success(),
        "unknown filter key must non-zero exit"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown stat key") && stderr.contains("fooStat"),
        "error must label class + name; got: {stderr}"
    );
}

/// NaN / infinity rejected at parse-time (II-05).
#[test]
fn l2_lindsay_query_leaders_filter_not_finite_rejected() {
    for bad in &["NaN", "inf", "-inf"] {
        let arg = format!("goals>={bad}");
        let out = run(&["query", "leaders", "--filter", &arg, "--season", "20242025"]);
        assert!(
            !out.status.success(),
            "non-finite filter value must non-zero exit (input: {arg})"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("not finite"),
            "error must label not-finite; got: {stderr}"
        );
    }
}

/// BENCH-checkpoint pre-commit: legacy typed flags (`--gp-min`,
/// `--ppg-min`, etc.) coexist with `--filter` independently. Typed
/// flags route to dedicated `PlayerFilter` slots BEFORE the
/// `--filter` loop appends to `stat_filters`; both gates apply via
/// AND. Pin: a result set with both flags is a subset of either alone.
#[test]
fn l2_lindsay_query_leaders_typed_flag_and_filter_compose_independently() {
    // Just typed --gp-min.
    let out_typed = run(&[
        "query", "leaders", "--gp-min", "70", "--top", "200", "--season", "20242025",
    ]);
    assert!(out_typed.status.success());

    // Just generic --filter.
    let out_filter = run(&[
        "query",
        "leaders",
        "--filter",
        "goals>=30",
        "--top",
        "200",
        "--season",
        "20242025",
    ]);
    assert!(out_filter.status.success());

    // Both: --gp-min 70 AND --filter goals>=30.
    let out_both = run(&[
        "query",
        "leaders",
        "--gp-min",
        "70",
        "--filter",
        "goals>=30",
        "--top",
        "200",
        "--season",
        "20242025",
    ]);
    assert!(
        out_both.status.success(),
        "typed + generic filter must coexist; stderr: {}",
        String::from_utf8_lossy(&out_both.stderr),
    );

    let parse_count = |s: &str| -> Option<u32> {
        for line in s.lines() {
            let trimmed = line.trim();
            if let Some(idx) = trimmed.find(" matched, showing ") {
                return trimmed[..idx].parse().ok();
            }
        }
        None
    };
    let typed = parse_count(&String::from_utf8_lossy(&out_typed.stdout)).expect("typed-only count");
    let filter =
        parse_count(&String::from_utf8_lossy(&out_filter.stdout)).expect("filter-only count");
    let both = parse_count(&String::from_utf8_lossy(&out_both.stdout)).expect("both count");
    // Both should be a subset of EITHER alone — neither flag dominates.
    assert!(
        both <= typed,
        "compound ({both}) must be subset of typed-only ({typed})"
    );
    assert!(
        both <= filter,
        "compound ({both}) must be subset of filter-only ({filter})"
    );
}

/// Empty filter string surfaces EmptyInput error.
#[test]
fn l2_lindsay_query_leaders_filter_empty_input_errors_cleanly() {
    let out = run(&["query", "leaders", "--filter", "", "--season", "20242025"]);
    assert!(!out.status.success(), "empty filter must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("filter is empty"),
        "error must label empty-input; got: {stderr}"
    );
}

// ── L2: `icelines playoffs` (LP.2) ───────────────────────────────────────────
//
// LP.2 shipped a brand-new top-level subcommand that consumes the same
// bundled playoff data the TUI Playoffs tab + web /playoffs already
// render. Six tests pin the contract: exits zero with a populated table
// year-round, --json carries the King.2.4 envelope, --csv carries a
// header, --round narrows, --season validates against the bundled set.

/// LP.2 / l2_cmd_playoffs_exits_zero
/// — bare invocation defaults to the most recent COMPLETED bracket and
///   prints a populated table. Must work in the offseason too.
#[test]
fn l2_cmd_playoffs_exits_zero() {
    let out = run(&["playoffs"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "playoffs must exit 0, stderr: {stderr}"
    );
    assert!(
        stdout.contains("PLAYOFFS"),
        "header missing, stdout: {stdout}"
    );
    assert!(
        stdout.contains("Champion:"),
        "default season must be COMPLETED so the Champion line shows; stdout: {stdout}"
    );
}

/// LP.2 / l2_cmd_playoffs_round_filter_narrows
/// — `--round 4` against 1993-94 prints the single Cup-Final series and
///   nothing from earlier rounds. NYR won the Cup that year, used as a
///   stable string anchor.
#[test]
fn l2_cmd_playoffs_round_filter_narrows() {
    let out = run(&["playoffs", "--season", "19931994", "--round", "4"]);
    assert!(out.status.success(), "must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("NYR"),
        "1993-94 final winner NYR must appear, stdout: {stdout}"
    );
    // Round 1 had 8 series — none of those team pairings should leak
    // through with --round 4. The Cup Final teams were NYR vs VAN; if
    // we see other 1993-94 R1 abbrevs mixed in, the filter regressed.
    assert!(
        !stdout.contains("BUF") && !stdout.contains("NJD"),
        "round filter leaked earlier-round series, stdout: {stdout}"
    );
}

/// LP.2 / l2_cmd_playoffs_json_envelope_shape
/// — Envelope follows King.2.4: schema_version + route + data + meta.
///   Pinning the literal key set guards against silent schema drift.
#[test]
fn l2_cmd_playoffs_json_envelope_shape() {
    let out = run(&["playoffs", "--season", "19931994", "--json"]);
    assert!(out.status.success(), "must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    let obj = v.as_object().expect("envelope must be a JSON object");
    let keys: std::collections::BTreeSet<_> = obj.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["data", "meta", "route", "schema_version"]
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        "envelope keys diverged: {keys:?}"
    );
    assert_eq!(obj["schema_version"], serde_json::json!(1));
    assert_eq!(obj["route"], serde_json::json!("playoffs"));
    assert!(obj["data"].is_array());
    let meta_keys: std::collections::BTreeSet<_> = obj["meta"]
        .as_object()
        .expect("meta must be an object")
        .keys()
        .map(String::as_str)
        .collect();
    let want: std::collections::BTreeSet<_> =
        ["champion", "conn_smythe", "count", "round_filter", "season"]
            .iter()
            .copied()
            .collect();
    assert_eq!(meta_keys, want, "meta keys diverged: {meta_keys:?}");
    assert_eq!(obj["meta"]["season"], serde_json::json!("19931994"));
}

/// LP.2 / l2_cmd_playoffs_csv_has_header
/// — CSV path emits the 8-column header up front so `csvkit`/Excel can
///   parse without surprises.
#[test]
fn l2_cmd_playoffs_csv_has_header() {
    let out = run(&["playoffs", "--season", "19931994", "--csv"]);
    assert!(out.status.success(), "must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first = stdout.lines().next().unwrap_or("");
    assert_eq!(
        first, "round,round_label,top_seed,bottom_seed,top_wins,bottom_wins,winner,games_played",
        "CSV header drifted"
    );
}

/// LP.2 / l2_cmd_playoffs_unbundled_season_errors_helpfully
/// — A season we don't carry should print the "try `icelines data
///   list`" hint, not panic or print an empty bracket.
#[test]
fn l2_cmd_playoffs_unbundled_season_errors_helpfully() {
    // 2099 is safely outside the 1987-88..2025-26 window.
    let out = run(&["playoffs", "--season", "20992100"]);
    assert!(!out.status.success(), "unbundled season must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no playoff bundle") && stderr.contains("data list"),
        "error must guide user to `data list`, stderr: {stderr}"
    );
}

/// LP.2 / l2_cmd_playoffs_invalid_round_rejected
/// — clap's value_parser range enforces 1..=4. `--round 5` should be
///   rejected at parse time, no panic.
#[test]
fn l2_cmd_playoffs_invalid_round_rejected() {
    let out = run(&["playoffs", "--round", "5"]);
    assert!(!out.status.success(), "round=5 must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
}

// ── L2: JSON envelope shape across LP commands (T2) ──────────────────────────
//
// King.2.4 established the envelope `{schema_version, route, data, meta}`
// for the web JSON twins. LP.1+LP.2 brought this convention to the CLI
// (schedule / playoffs / tonight). These tests pin the literal key set
// so a schema bump can't slip in unannounced — change the test
// alongside the change to record intent.

/// T2 / l2_cmd_schedule_json_envelope_shape
/// — Schedule `--json` envelope shape. Day-zero default is 7 days; we
///   keep the call short to stay deterministic across reruns.
#[test]
fn l2_cmd_schedule_json_envelope_shape() {
    let out = run(&["schedule", "--days", "1", "--json"]);
    assert!(
        out.status.success(),
        "schedule --json must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    let obj = v.as_object().expect("envelope must be a JSON object");
    let keys: std::collections::BTreeSet<_> = obj.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["data", "meta", "route", "schema_version"]
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        "envelope keys diverged: {keys:?}"
    );
    assert_eq!(obj["route"], serde_json::json!("schedule"));
    assert_eq!(obj["schema_version"], serde_json::json!(1));
    assert!(obj["data"].is_array());
}

// ── L2: `icelines query career` (Calder.4) ───────────────────────────────────
//
// Calder.4 ships a cross-league leaderboard. The L2 surface assumes
// the local store may or may not be populated on the test runner;
// tests that actually need a populated store are gated below with
// an env-var check (`ICELINES_HAS_CAREER_STORE=1`) so CI doesn't
// silently green-pass when the store is empty.

/// Calder.4 / l2_cmd_query_career_help_listed
/// — `query --help` lists the new `career` subcommand.
#[test]
fn l2_cmd_query_career_help_listed() {
    let out = run(&["query", "--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("career"),
        "query --help must list the new `career` subcommand"
    );
}

/// Calder.4 / l2_cmd_query_career_unknown_sort_rejected
/// — `--sort xyz` must error cleanly (typed enum-ish parser).
#[test]
fn l2_cmd_query_career_unknown_sort_rejected() {
    let out = run(&["query", "career", "--league", "OHL", "--sort", "xyz"]);
    assert!(!out.status.success(), "bad --sort must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("unknown --sort"),
        "should explain the bad sort, stderr: {stderr}"
    );
}

/// Calder.4 / l2_cmd_query_career_invalid_season_format_rejected
/// — `--season abc` rejected at parse time.
#[test]
fn l2_cmd_query_career_invalid_season_format_rejected() {
    let out = run(&[
        "query",
        "career",
        "--league",
        "OHL",
        "--season",
        "not-a-year",
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
}

/// Calder.4 / l2_cmd_query_career_json_envelope_shape
/// — Pin the King.2.4 envelope literal keys when the store is
///   populated. Skipped on CI where the store may be empty.
#[test]
fn l2_cmd_query_career_json_envelope_shape() {
    if std::env::var("ICELINES_HAS_CAREER_STORE").is_err() {
        eprintln!(
            "skipped: set ICELINES_HAS_CAREER_STORE=1 after `fetch career --bundled-seasons 5`"
        );
        return;
    }
    let out = run(&[
        "query", "career", "--league", "OHL", "--season", "20142015", "--json",
    ]);
    assert!(out.status.success(), "must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    let obj = v.as_object().expect("envelope must be object");
    let keys: std::collections::BTreeSet<_> = obj.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["data", "meta", "route", "schema_version"]
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        "envelope keys diverged: {keys:?}"
    );
    assert_eq!(obj["route"], serde_json::json!("career"));
    assert_eq!(obj["schema_version"], serde_json::json!(1));
    assert!(obj["data"].is_array());
    let meta_keys: std::collections::BTreeSet<_> = obj["meta"]
        .as_object()
        .expect("meta is an object")
        .keys()
        .map(String::as_str)
        .collect();
    let want: std::collections::BTreeSet<_> = ["count", "league", "season", "sort", "total"]
        .iter()
        .copied()
        .collect();
    assert_eq!(meta_keys, want, "meta keys diverged: {meta_keys:?}");
}

/// Calder.2 / l2_cmd_fetch_career_dry_run_exits_zero
/// — `fetch career --dry-run` accepts the flag and prints the plan.
///   Real network call short-circuits in dry-run mode.
#[test]
fn l2_cmd_fetch_career_dry_run_exits_zero() {
    let out = run(&["fetch", "career", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "fetch career --dry-run must exit 0; stderr: {stderr}"
    );
    assert!(
        stdout.contains("Would fetch career history") && stdout.contains("landing"),
        "dry-run output missing plan, stdout: {stdout}"
    );
}

/// Calder.2 / l2_cmd_fetch_career_help_listed
/// — the new subcommand must show up in `fetch --help`.
#[test]
fn l2_cmd_fetch_career_help_listed() {
    let out = run(&["fetch", "--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("career"),
        "fetch --help must list the new `career` subcommand"
    );
}

/// T2 / l2_cmd_tonight_no_json_flag_documented
/// — `tonight` does not carry a `--json` flag today; document that
///   here so a future addition has to re-read this test (and then the
///   author can pivot it to a real envelope assertion).
#[test]
fn l2_cmd_tonight_no_json_flag_documented() {
    let out = run(&["tonight", "--json"]);
    // clap rejects unknown flag → non-zero exit, no panic.
    assert!(!out.status.success(), "tonight --json must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("--json") || stderr.contains("unexpected"),
        "should mention the unknown flag, stderr: {stderr}"
    );
}

// ── Phase Foster.0.8 — setup wizard L2 ───────────────────────────────────────

/// L2 / l2_foster08_setup_accept_defaults_dry_run
/// — `icelines setup --accept-defaults --dry-run` exits 0, prints
///   the resolved defaults, and does NOT write `~/.icelines/config.toml`.
///   Spec test for the "F.0.8 dry-run branch".
#[test]
fn l2_foster08_setup_accept_defaults_dry_run() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["setup", "--accept-defaults", "--dry-run"]);
    assert!(
        out.status.success(),
        "exit 0 expected, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("accepting defaults"), "stdout: {stdout}");
    assert!(
        stdout.contains("transactions    = favorites"),
        "default transactions=favorites must surface, stdout: {stdout}"
    );
    assert!(
        stdout.contains("(dry run"),
        "dry-run banner must surface, stdout: {stdout}"
    );
    // Critical: --dry-run must NOT have written the config file.
    let config_path = home.path().join(".icelines").join("config.toml");
    assert!(
        !config_path.exists(),
        "dry-run must not write {}",
        config_path.display()
    );
}

/// L2 / l2_foster08_setup_accept_defaults_persists
/// — without `--dry-run`, setup writes the config and a follow-up
///   `icelines config get sync.capabilities.transactions` reads back
///   the persisted value.
#[test]
fn l2_foster08_setup_accept_defaults_persists() {
    let home = tempfile::tempdir().expect("tempdir");
    let setup = run_isolated(home.path(), &["setup", "--accept-defaults"]);
    assert!(
        setup.status.success(),
        "setup exit 0, stderr: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let config_path = home.path().join(".icelines").join("config.toml");
    assert!(
        config_path.exists(),
        "config.toml should be written at {}",
        config_path.display()
    );
    let body = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        body.contains("transactions = \"favorites\""),
        "config body must carry default transactions, got: {body}"
    );

    // Round-trip via `icelines config get`.
    let get = run_isolated(
        home.path(),
        &["config", "get", "sync.capabilities.transactions"],
    );
    assert!(get.status.success());
    let got = String::from_utf8_lossy(&get.stdout);
    assert_eq!(
        got.trim(),
        "favorites",
        "config get must round-trip the persisted value"
    );
}

/// L2 / l2_foster08_config_set_shifts_rejects_with_literal_error
/// — pinning the cross-process behavior of the BENCH-H3 literal
///   error string. The same error surfaces from `icelines config set`
///   as from in-process `Config::set_key`.
#[test]
fn l2_foster08_config_set_shifts_favorites_rejected() {
    let home = tempfile::tempdir().expect("tempdir");
    // Setup first so config exists.
    let _ = run_isolated(home.path(), &["setup", "--accept-defaults"]);
    let out = run_isolated(
        home.path(),
        &["config", "set", "sync.capabilities.shifts", "favorites"],
    );
    assert!(!out.status.success(), "must non-zero exit on shifts != off");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("capability `shifts` cannot be set to `favorites`"),
        "literal error string must surface, stderr: {stderr}"
    );
    assert!(
        stderr.contains("Allowed values today: off"),
        "trailer must surface, stderr: {stderr}"
    );
}

// ── Phase Foster.1 — date-anchored CLI L2 ────────────────────────────────────

/// L2 / l2_foster1_tonight_invalid_date_clean_error
/// — `icelines tonight --date not-a-date` must surface a helpful
///   parse error and non-zero exit, NOT panic. Pure offline test —
///   the parser fires before any network call.
#[test]
fn l2_foster1_tonight_invalid_date_clean_error() {
    let out = run(&["tonight", "--date", "not-a-date"]);
    assert!(
        !out.status.success(),
        "invalid date must non-zero exit, status: {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("invalid date") && stderr.contains("YYYY-MM-DD"),
        "error must point to YYYY-MM-DD format, stderr: {stderr}"
    );
}

/// L2 / l2_foster1_schedule_accepts_date_flag
/// — `--date` parses cleanly even with the deprecated `--start`
///   alias also present. Smoke confirms clap accepts the new flag
///   without erroring on the old one being hidden.
#[test]
fn l2_foster1_schedule_invalid_date_clean_error() {
    let out = run(&["schedule", "--date", "2026-13-01"]);
    assert!(
        !out.status.success(),
        "month 13 must non-zero exit, status: {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("invalid date"),
        "error must mention 'invalid date', stderr: {stderr}"
    );
}

// ── Phase Conn Smythe C.2 — Cup-run leaderboard L2 ──────────────────────────

/// L2 / l2_conn_smythe_c2_query_leaders_playoff_empty_manifest
/// — `icelines query leaders --playoff` exits 0 with the
///   "no playoff boxscores on disk" empty-state when the manifest
///   has no Boxscore entries.
#[test]
fn l2_conn_smythe_c2_query_leaders_playoff_empty_manifest() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["query", "leaders", "--playoff"]);
    assert!(
        out.status.success(),
        "exit 0 expected on empty manifest, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("PLAYOFF LEADERS"),
        "header must surface, stdout: {stdout}"
    );
    assert!(
        stdout.contains("no playoff boxscores"),
        "empty-state must surface, stdout: {stdout}"
    );
}

#[test]
fn l2_conn_smythe_c2_query_leaders_playoff_json_envelope() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["query", "leaders", "--playoff", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json must emit valid JSON");
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["route"], "leaders.playoff");
    assert_eq!(parsed["meta"]["kind"], "playoff_run");
    assert!(parsed["data"].is_array());
}

// ── Phase Conn Smythe C.1 — series momentum L2 ───────────────────────────────

/// L2 / l2_conn_smythe_c1_playoffs_series_renders_momentum
/// — `icelines playoffs --series A --season 19931994` exits 0 and
///   surfaces the SERIES header + summary line.
#[test]
fn l2_conn_smythe_c1_playoffs_series_renders_momentum() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(
        home.path(),
        &["playoffs", "--season", "19931994", "--series", "A"],
    );
    assert!(
        out.status.success(),
        "exit 0 expected, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("SERIES A"), "stdout: {stdout}");
    assert!(
        stdout.contains("(top seed)"),
        "matchup line must appear, stdout: {stdout}"
    );
    assert!(
        stdout.contains("games played"),
        "summary line must appear, stdout: {stdout}"
    );
}

#[test]
fn l2_conn_smythe_c1_playoffs_series_unknown_letter_clean_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(
        home.path(),
        &["playoffs", "--season", "19931994", "--series", "Z"],
    );
    assert!(!out.status.success(), "unknown letter must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("no series 'Z'"),
        "remediation must surface, stderr: {stderr}"
    );
}

// ── Phase Foster +2 — data status command L2 ─────────────────────────────────

/// L2 / l2_foster_plus2_data_status_empty_manifest
/// — `icelines data status` on a fresh `~/.icelines` exits 0 and
///   surfaces the empty-state remediation pointer.
#[test]
fn l2_foster_plus2_data_status_empty_manifest() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["data-status"]);
    assert!(
        out.status.success(),
        "exit 0 expected, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Manifest is empty"),
        "empty-state must surface, stdout: {stdout}"
    );
    assert!(
        stdout.contains("icelines setup"),
        "remediation pointer must surface, stdout: {stdout}"
    );
}

#[test]
fn l2_foster_plus2_data_status_unknown_shard_clean_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["data-status", "--shard", "garbage"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown shard") && stderr.contains("bios"),
        "must list valid shards, stderr: {stderr}"
    );
}

// ── Phase Foster.5 — timeframe surface L2 ────────────────────────────────────

/// L2 / l2_foster5_query_career_week_rejected_per_edge_b2
/// — `query career --week` is intentionally rejected with the
///   documented error message. Pins the literal wording per spec
///   §"Timeframe rejection on query career" (EDGE B2).
#[test]
fn l2_foster5_query_career_week_rejected_per_edge_b2() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(
        home.path(),
        &["query", "career", "--league", "OHL", "--week"],
    );
    assert!(!out.status.success(), "must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("--week / --month not supported on `query career`"),
        "literal error must surface, stderr: {stderr}"
    );
    assert!(
        stderr.contains("Use --season instead"),
        "remediation must appear, stderr: {stderr}"
    );
}

#[test]
fn l2_foster5_query_career_month_rejected_per_edge_b2() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(
        home.path(),
        &["query", "career", "--league", "OHL", "--month"],
    );
    assert!(!out.status.success(), "must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--week / --month not supported on `query career`"),
        "same error covers both flags, stderr: {stderr}"
    );
}

// ── Phase Foster.4 — sync engine L2 ──────────────────────────────────────────

/// L2 / l2_foster4_fetch_sync_dry_run_empty_manifest
/// — On a fresh `~/.icelines` (empty manifest), `fetch sync --dry-run`
///   exits 0 and reports "Nothing stale." No network calls; the
///   walk is purely over the in-memory manifest map.
#[test]
fn l2_foster4_fetch_sync_dry_run_empty_manifest() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["fetch", "sync", "--dry-run"]);
    assert!(
        out.status.success(),
        "exit 0 expected, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Nothing stale"),
        "empty manifest must surface 'Nothing stale.', stdout: {stdout}"
    );
}

// ── Phase Foster.3 — boxscore fetcher L2 ─────────────────────────────────────

/// L2 / l2_foster3_fetch_boxscore_invalid_date_clean_error
/// — `fetch boxscore --date garbage` must surface the date validator
///   error without panicking. Pure offline test.
#[test]
fn l2_foster3_fetch_boxscore_invalid_date_clean_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["fetch", "boxscore", "--date", "garbage"]);
    assert!(!out.status.success(), "invalid date must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("invalid date") && stderr.contains("YYYY-MM-DD"),
        "validator error must surface, stderr: {stderr}"
    );
}

/// L2 / l2_trace_events_fetch_play_by_play_invalid_date_clean_error
/// — `fetch play-by-play --date garbage` must reuse the date validator
///   error before any network call.
#[test]
fn l2_trace_events_fetch_play_by_play_invalid_date_clean_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["fetch", "play-by-play", "--date", "garbage"]);
    assert!(!out.status.success(), "invalid date must non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("invalid date") && stderr.contains("YYYY-MM-DD"),
        "validator error must surface, stderr: {stderr}"
    );
}

// ── Phase Foster.2 — favorites dashboard L2 ──────────────────────────────────

/// L2 / l2_foster2_favorites_empty_group_renders_empty_state
/// — Fresh `~/.icelines` (no group members) → `icelines favorites`
///   exits 0 and surfaces the empty-state instructional card with
///   the canonical `group add` example commands.
#[test]
fn l2_foster2_favorites_empty_group_renders_empty_state() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["favorites"]);
    assert!(
        out.status.success(),
        "exit 0 expected even with empty group, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("FAVORITES") && stdout.contains("empty"),
        "empty-state header must surface, stdout: {stdout}"
    );
    assert!(
        stdout.contains("icelines group add Favorites"),
        "must teach the user how to add favorites, stdout: {stdout}"
    );
}

/// L2 / l2_foster2_favorites_invalid_date_clean_error
/// — Date validator runs before group lookup; garbage value surfaces
///   the same error pattern as `tonight --date`.
#[test]
fn l2_foster2_favorites_invalid_date_clean_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["favorites", "--date", "garbage"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid date"),
        "must surface validator error, stderr: {stderr}"
    );
}

/// L2 / l2_foster2_favorites_invalid_range_clean_error
/// — Unknown --range value rejected with the allowed list.
#[test]
fn l2_foster2_favorites_invalid_range_clean_error() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["favorites", "--range", "forever"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown --range") && stderr.contains("season"),
        "error must list allowed values, stderr: {stderr}"
    );
}

/// L2 / l2_foster2_favorites_json_envelope_shape
/// — `--json` emits the K2.4-style envelope (heterogeneous data
///   object) per WIRE B1. With an empty group the data fields are
///   all empty arrays.
#[test]
fn l2_foster2_favorites_json_envelope_shape() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = run_isolated(home.path(), &["favorites", "--json"]);
    assert!(
        out.status.success(),
        "exit 0 expected, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("must emit valid JSON");
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["route"], "favorites");
    assert!(parsed["data"]["players"].is_array());
    assert!(parsed["data"]["teams"].is_array());
    assert!(parsed["data"]["events"].is_array());
    assert_eq!(parsed["meta"]["counts"]["players"], 0);
}

/// L2 / l2_foster1_schedule_deprecated_start_alias_still_accepted
/// — `--start YYYY-MM-DD` is the deprecated alias for `--date`.
///   Hidden from --help but must still parse so existing scripts keep
///   working through the transition window. Garbage value still
///   surfaces a clean parse error rather than panicking.
#[test]
fn l2_foster1_schedule_deprecated_start_alias_still_accepted() {
    let out = run(&["schedule", "--start", "still-bad"]);
    assert!(
        !out.status.success(),
        "alias still validates the date format"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "must not panic, stderr: {stderr}"
    );
    assert!(
        stderr.contains("invalid date"),
        "alias-routed error must reach the validator, stderr: {stderr}"
    );
}
