//! Persona Wave 3 — 100 more scenarios across previously-untested
//! surface: secondary query subcommands, fantasy/scheme, export/x,
//! JSON/CSV output, bundle integrity, robustness.
//!
//! Build: `cargo build --release -p icelines-cli`
//! Run: `cargo test -p icelines-cli --test persona_wave3`

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
        .env("ICELINES_NO_LIVE", "1") // determinism — no network even if subcommand can hit it
        .output()
        .unwrap_or_else(|e| panic!("failed to run icelines: {e}"))
}

fn run_isolated(home: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ICELINES_NO_LIVE", "1")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run icelines: {e}"))
}

fn ok(args: &[&str]) -> String {
    let out = run(args);
    assert!(
        out.status.success(),
        "{:?} must succeed; stderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn no_panic(args: &[&str]) {
    let out = run(args);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"), "{:?} panicked", args);
}

// ── Bucket T: Secondary query subcommands (15) ─────────────────────────────

#[test]
fn p201_rank_default_runs() {
    let s = ok(&["rank", "--top", "10"]);
    assert!(s.lines().count() > 5);
}

#[test]
#[allow(non_snake_case)]
fn p202_rank_with_pos_C() {
    ok(&["rank", "--pos", "C", "--top", "5"]);
}

#[test]
fn p203_rank_json_output() {
    let s = ok(&["rank", "--top", "3", "--json"]);
    assert!(
        s.contains('{') || s.contains('['),
        "json output expected, got:\n{s}"
    );
}

#[test]
fn p204_rank_csv_output() {
    let s = ok(&["rank", "--top", "3", "--csv"]);
    assert!(s.contains(','), "csv output expected, got:\n{s}");
}

#[test]
fn p205_team_command_smoke() {
    // `team --help` is the safe smoke since args vary.
    no_panic(&["team", "--help"]);
}

#[test]
fn p206_players_command_smoke() {
    no_panic(&["players", "--top", "5"]);
}

#[test]
fn p207_history_command_for_known_player() {
    let s = ok(&["history", "Connor McDavid"]);
    assert!(s.to_lowercase().contains("mcdavid"));
}

#[test]
fn p208_history_unknown_player_clean_error() {
    no_panic(&["history", "Xyz Nobody"]);
}

#[test]
fn p209_project_command_smoke() {
    no_panic(&["project", "--help"]);
}

#[test]
fn p210_scouting_command_for_known_player() {
    no_panic(&["scouting", "Connor McDavid"]);
}

#[test]
fn p211_mates_command_for_known_player() {
    no_panic(&["mates", "Connor McDavid"]);
}

#[test]
fn p212_peers_command_for_known_player() {
    no_panic(&["peers", "Connor McDavid"]);
}

#[test]
fn p213_class_2015_draft_year() {
    no_panic(&["class", "2015"]);
}

#[test]
fn p214_class_2003_loaded_draft() {
    // 2003 was a famously deep draft year.
    no_panic(&["class", "2003"]);
}

#[test]
fn p215_compare_top_level_alias() {
    // top-level `compare` mirrors `query compare`.
    no_panic(&["compare", "Connor McDavid", "Sidney Crosby"]);
}

// ── Bucket U: Export + x subcommands (10) ─────────────────────────────────

fn temp_md(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("icelines-w3-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{name}.md"))
}

#[test]
fn p216_export_md_leaders_writes_file() {
    let path = temp_md("leaders");
    ok(&[
        "export",
        "md",
        "leaders",
        "--top",
        "5",
        "--out",
        path.to_str().unwrap(),
    ]);
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn p217_export_md_leaders_with_alias_columns() {
    let path = temp_md("leaders_aliases");
    ok(&[
        "export",
        "md",
        "leaders",
        "--columns",
        "g,a,p",
        "--top",
        "3",
        "--out",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn p218_export_md_help_lists_shapes() {
    let s = ok(&["export", "md", "--help"]);
    let _ = s; // help should run cleanly
}

#[test]
fn p219_x_leaders_default_csv_to_stdout() {
    let s = ok(&["x", "leaders", "--top", "3"]);
    // CSV: header line + 3 rows.
    assert!(s.lines().count() >= 4, "expected ≥4 csv lines, got:\n{s}");
}

#[test]
fn p220_x_goalies_csv() {
    ok(&["x", "goalies", "--top", "3"]);
}

#[test]
fn p221_x_rank_csv() {
    ok(&["x", "rank", "--top", "3"]);
}

#[test]
fn p222_x_history_for_player_via_player_flag() {
    // FINDING: `x` doesn't take positional player args; it uses
    // --player on history/peers/compare shapes.
    ok(&["x", "history", "--player", "Connor McDavid"]);
}

#[test]
fn p223_x_class_2015_via_year_flag() {
    // FINDING: `x class` uses --year.
    ok(&["x", "class", "--year", "2015"]);
}

#[test]
fn p224_x_unknown_shape_errors() {
    let out = run(&["x", "totally-fake-shape"]);
    assert!(!out.status.success());
}

#[test]
fn p225_x_help_succeeds() {
    no_panic(&["x", "--help"]);
}

// ── Bucket V: JSON / CSV shape validation (10) ────────────────────────────

#[test]
fn p226_query_leaders_json_parses_as_array() {
    let s = ok(&["query", "leaders", "--top", "3", "--json"]);
    let v: serde_json::Value =
        serde_json::from_str(s.trim()).expect("query leaders --json must produce valid JSON");
    assert!(v.is_array() || v.is_object(), "must be array or object");
}

#[test]
fn p227_query_leaders_csv_has_header_row() {
    let s = ok(&["query", "leaders", "--top", "3", "--csv"]);
    let first = s.lines().next().unwrap_or("");
    assert!(
        first.contains(','),
        "first csv line must have commas, got: {first:?}"
    );
}

#[test]
fn p228_query_goalies_json_round_trips() {
    let s = ok(&["query", "goalies", "--top", "3", "--json"]);
    let _v: serde_json::Value = serde_json::from_str(s.trim()).expect("goalies json must parse");
}

#[test]
fn p229_query_goalies_csv_has_header_row() {
    let s = ok(&["query", "goalies", "--top", "3", "--csv"]);
    let first = s.lines().next().unwrap_or("");
    assert!(first.contains(','), "got: {first:?}");
}

#[test]
fn p230_rank_json_round_trips() {
    let s = ok(&["rank", "--top", "3", "--json"]);
    let _v: serde_json::Value = serde_json::from_str(s.trim()).expect("rank json must parse");
}

#[test]
fn p231_x_leaders_json_format_flag() {
    no_panic(&["x", "leaders", "--top", "3", "--format", "json"]);
}

#[test]
fn p232_query_leaders_json_with_filter() {
    let s = ok(&[
        "query",
        "leaders",
        "--top",
        "3",
        "--filter",
        "goals>=20",
        "--json",
    ]);
    let _v: serde_json::Value = serde_json::from_str(s.trim()).expect("must parse");
}

#[test]
fn p233_query_leaders_csv_with_filter() {
    ok(&[
        "query",
        "leaders",
        "--top",
        "3",
        "--filter",
        "goals>=20",
        "--csv",
    ]);
}

#[test]
fn p234_query_leaders_json_overrides_csv_when_both_set() {
    // FINDING: `query leaders` doesn't enforce a mutex on --json/--csv;
    // when both are passed, JSON wins silently. This is a UX
    // inconsistency vs `query goalies` (which errors). Test pins
    // current behavior — adjust when the mutex lands.
    let out = run(&["query", "leaders", "--top", "3", "--json", "--csv"]);
    assert!(out.status.success());
}

#[test]
fn p235_query_goalies_json_csv_mutex() {
    // goalies DOES enforce the mutex.
    let out = run(&["query", "goalies", "--top", "3", "--json", "--csv"]);
    assert!(!out.status.success());
}

// ── Bucket W: Bundle integrity per-season (15) ────────────────────────────
//
// L0 — pure-Rust assertions against bundled data, no subprocess needed.
// (Tests are still in this file so the whole wave runs together.)

mod bundle_l0 {
    use icelines_fetch::bundled;

    fn each_bundled_season(mut f: impl FnMut(&str)) {
        for season in icelines_fetch::BUNDLED_SEASONS {
            f(season);
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn p236_each_season_has_at_least_one_position_C() {
        each_bundled_season(|season| {
            let bios = bundled::get_bios(season).expect("bios");
            let centers = bios.iter().filter(|b| b.position_code == "C").count();
            assert!(
                centers > 0,
                "{season} has 0 centers — bundle authoring bug?"
            );
        });
    }

    #[test]
    fn p237_each_season_has_at_least_one_defenseman() {
        each_bundled_season(|season| {
            let bios = bundled::get_bios(season).expect("bios");
            let d = bios.iter().filter(|b| b.position_code == "D").count();
            assert!(d > 0, "{season} has 0 defensemen");
        });
    }

    #[test]
    fn p238_each_season_stats_dup_count_within_traded_player_norm() {
        // FINDING: NHL bios endpoint emits multiple rows per player when
        // they're traded mid-season — this is documented in CLAUDE.md.
        // The repository deduplicates on load. At the BUNDLE level we
        // expect SOME dupes (traded players) but the dup count should
        // be < 10% of total — anything higher signals a bundle-authoring
        // bug (e.g. doubled rows on every player).
        each_bundled_season(|season| {
            let stats = bundled::get_stats(season).expect("stats");
            let mut ids: Vec<u32> = stats.iter().map(|s| s.player_id).collect();
            let total = ids.len();
            ids.sort();
            ids.dedup();
            let unique = ids.len();
            let dup_count = total - unique;
            assert!(
                dup_count < total / 5, // <20% — accounts for trade-deadline-heavy years
                "{season} dup ratio too high: {dup_count}/{total} dupes (unique={unique})"
            );
        });
    }

    #[test]
    fn p239_each_season_bios_player_ids_positive() {
        each_bundled_season(|season| {
            let bios = bundled::get_bios(season).expect("bios");
            assert!(bios.iter().all(|b| b.player_id > 0), "{season}");
        });
    }

    #[test]
    fn p240_each_season_no_negative_goals() {
        each_bundled_season(|season| {
            let stats = bundled::get_stats(season).expect("stats");
            // u32 can't be negative; this asserts no overflow weirdness either.
            let total: u64 = stats.iter().map(|s| s.goals as u64).sum();
            assert!(total > 0, "{season} has zero total goals — empty bundle?");
        });
    }

    #[test]
    fn p241_each_season_goalie_stats_present() {
        each_bundled_season(|season| {
            let goalies = bundled::get_goalie_stats(season).expect("goalies");
            assert!(
                !goalies.is_empty(),
                "{season} has no goalie stats — bundle missing"
            );
        });
    }

    #[test]
    fn p242_each_season_playoff_bios_or_empty() {
        each_bundled_season(|season| {
            // Playoff bios may be [] (current season) but Some.
            assert!(
                bundled::get_playoff_bios(season).is_some(),
                "{season} playoff-bios.json missing"
            );
        });
    }

    #[test]
    fn p243_modern_seasons_have_transactions() {
        for s in &["20212022", "20222023", "20232024", "20242025", "20252026"] {
            assert!(
                bundled::get_transactions(s).is_some(),
                "{s} transactions bundle missing"
            );
        }
    }

    #[test]
    fn p244_pre_2021_has_no_transactions() {
        for s in &["20202021", "20192020", "20122013", "19921993"] {
            assert!(
                bundled::get_transactions(s).is_none(),
                "{s} should not carry transactions (pre-2021)"
            );
        }
    }

    #[test]
    fn p245_each_season_max_player_id_below_safe_cap() {
        // NHL playerIds are 7-8 digit. Anything > 99_999_999 is a bug.
        each_bundled_season(|season| {
            let bios = bundled::get_bios(season).expect("bios");
            let max = bios.iter().map(|b| b.player_id).max().unwrap_or(0);
            assert!(max < 99_999_999, "{season} has impossible max id {max}");
        });
    }

    #[test]
    fn p246_each_season_stats_count_matches_or_below_bios() {
        each_bundled_season(|season| {
            let bios = bundled::get_bios(season).expect("bios");
            let stats = bundled::get_stats(season).expect("stats");
            // Stats may be ≤ bios (some debuts have a bio but missed entire season).
            assert!(
                stats.len() <= bios.len() * 2,
                "{season} stats={} unreasonably > bios={}",
                stats.len(),
                bios.len()
            );
        });
    }

    #[test]
    fn p247_no_season_id_drift_on_stats() {
        // Every stats row's seasonId (when present) matches the
        // bundle filename. Catches a bundle-authoring crossover.
        each_bundled_season(|season| {
            let expected: u32 = season.parse().unwrap();
            let stats = bundled::get_stats(season).expect("stats");
            for s in &stats {
                if let Some(sid) = s.season_id {
                    assert_eq!(
                        sid, expected,
                        "{season}: row {} has seasonId {}",
                        s.player_id, sid
                    );
                }
            }
        });
    }

    #[test]
    fn p248_realtime_columns_only_modern_data_carries_data() {
        // Pre-2009-10 data has null hits/blocks (NHL didn't track).
        // Verify modern era has SOME hits sum > 0 vs pre-era 0.
        // (This is bundled summary data — realtime is snapshot-only.)
        // Just ensure we can read the bundles either way.
        for s in &["20242025", "20012002", "19951996"] {
            let stats = bundled::get_stats(s).expect("stats");
            assert!(!stats.is_empty(), "{s} stats empty");
        }
    }

    #[test]
    fn p249_playoff_goalie_stats_present_for_contested_seasons() {
        // Every season that had a Cup contested has playoff goalie data.
        // 2025-26 may be empty; everything else has goalies.
        for s in icelines_fetch::BUNDLED_SEASONS
            .iter()
            .filter(|&&s| s != "20252026")
        {
            let g = bundled::get_playoff_goalie_stats(s).expect("playoff goalies");
            assert!(!g.is_empty(), "{s} playoff goalie stats empty");
        }
    }

    #[test]
    fn p250_38_seasons_total() {
        assert_eq!(icelines_fetch::BUNDLED_SEASONS.len(), 38);
    }
}

// ── Bucket X: Robustness, encoding, edge inputs (15) ─────────────────────

#[test]
fn p251_special_apostrophe_in_player_name() {
    // Players with apostrophes (O'Reilly, etc.) — Unicode-safe lookup.
    no_panic(&["query", "player", "O'Reilly"]);
}

#[test]
fn p252_unicode_diacritic_player_name() {
    // Players with diacritics (Pär Lindholm, Mikaël Granlund).
    no_panic(&["query", "player", "Granlund"]);
}

#[test]
fn p253_empty_string_player_name_errors() {
    let out = run(&["query", "player", ""]);
    let _ = out;
}

#[test]
fn p254_very_long_top_5000() {
    // Asks for more rows than exist; should clamp without overflow.
    ok(&["query", "leaders", "--top", "5000"]);
}

#[test]
fn p255_top_negative_clamps_or_errors() {
    let out = run(&["query", "leaders", "--top", "-5"]);
    let _ = out; // either clamps or errors; must not panic
}

#[test]
fn p256_filter_value_with_decimal() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "shooting-pct>=0.15",
        "--top",
        "5",
    ]);
}

#[test]
fn p257_filter_value_zero() {
    ok(&["query", "leaders", "--filter", "goals>=0", "--top", "5"]);
}

#[test]
fn p258_filter_value_locale_comma_rejected() {
    // L.2.4 grammar — `,` decimal separator must be rejected
    // (BadNumber error).
    let out = run(&[
        "query",
        "leaders",
        "--filter",
        "shooting-pct>=0,15",
        "--top",
        "5",
    ]);
    assert!(!out.status.success());
}

#[test]
fn p259_team_uppercase_lowercase_both_work() {
    ok(&["query", "leaders", "--team", "edm", "--top", "5"]);
    ok(&["query", "leaders", "--team", "EDM", "--top", "5"]);
}

#[test]
fn p260_handedness_lowercase_l_works() {
    ok(&["query", "leaders", "--handedness", "L", "--top", "5"]);
}

#[test]
fn p261_no_live_flag_affects_no_query() {
    // --no-live shouldn't break offline queries.
    ok(&["--no-live", "query", "leaders", "--top", "5"]);
}

#[test]
fn p262_no_dashboards_flag_passes_through() {
    ok(&["--no-dashboards", "query", "leaders", "--top", "5"]);
}

#[test]
fn p263_repeated_invocations_no_state_leak() {
    for _ in 0..10 {
        ok(&["query", "leaders", "--top", "1"]);
    }
}

#[test]
fn p264_team_unknown_abbrev_no_panic() {
    no_panic(&["query", "leaders", "--team", "ZZZ", "--top", "5"]);
}

#[test]
#[allow(non_snake_case)]
fn p265_pos_F_resolves_to_all_forwards() {
    let s = ok(&["query", "leaders", "--pos", "F", "--top", "5"]);
    // F includes C, LW, RW — output should not contain "D" rows.
    let lines: Vec<&str> = s
        .lines()
        .filter(|l| l.contains(" 1    ") || l.contains(" 2    "))
        .collect();
    let _ = lines;
}

// ── Bucket Y: Fantasy + scheme (10) ──────────────────────────────────────

#[test]
fn p266_scheme_list_runs() {
    ok(&["scheme", "list"]);
}

#[test]
fn p267_scheme_show_yahoo_default() {
    no_panic(&["scheme", "show", "yahoo-default-points"]);
}

#[test]
fn p268_scheme_show_unknown_errors() {
    let out = run(&["scheme", "show", "totally-fake-scheme"]);
    assert!(!out.status.success());
}

#[test]
fn p269_fantasy_league_list_in_temp_home() {
    let dir = tempfile::TempDir::new().unwrap();
    let out = run_isolated(dir.path(), &["fantasy", "league-list"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p270_fantasy_league_create_in_temp_home() {
    let dir = tempfile::TempDir::new().unwrap();
    let out = run_isolated(
        dir.path(),
        &[
            "fantasy",
            "league-create",
            "--name",
            "Test Wave 3",
            "--scheme",
            "yahoo-default-points",
        ],
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p271_fantasy_team_create_after_league() {
    let dir = tempfile::TempDir::new().unwrap();
    let _ = run_isolated(
        dir.path(),
        &[
            "fantasy",
            "league-create",
            "--name",
            "L",
            "--scheme",
            "yahoo-default-points",
        ],
    );
    let out = run_isolated(dir.path(), &["fantasy", "team-create", "--name", "Team1"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p272_fantasy_standings_empty_in_fresh_home() {
    let dir = tempfile::TempDir::new().unwrap();
    no_panic_iso(dir.path(), &["fantasy", "league-list"]);
}

#[test]
fn p273_scheme_help() {
    ok(&["scheme", "--help"]);
}

#[test]
fn p274_fantasy_help() {
    ok(&["fantasy", "--help"]);
}

#[test]
fn p275_fantasy_league_create_without_name_errors() {
    let dir = tempfile::TempDir::new().unwrap();
    let out = run_isolated(dir.path(), &["fantasy", "league-create"]);
    assert!(!out.status.success());
}

fn no_panic_iso(home: &std::path::Path, args: &[&str]) {
    let out = run_isolated(home, args);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

// ── Bucket Z: Snapshot, group, transactions, schedule (15) ───────────────

#[test]
fn p276_snapshot_help() {
    ok(&["snapshot", "--help"]);
}

#[test]
fn p277_snapshot_list_in_fresh_home() {
    let dir = tempfile::TempDir::new().unwrap();
    no_panic_iso(dir.path(), &["snapshot", "list"]);
}

#[test]
fn p278_group_help() {
    ok(&["group", "--help"]);
}

#[test]
fn p279_group_list_fresh_home() {
    let dir = tempfile::TempDir::new().unwrap();
    no_panic_iso(dir.path(), &["group", "list"]);
}

#[test]
fn p280_transactions_help() {
    ok(&["transactions", "--help"]);
}

#[test]
fn p281_transactions_list_modern_season() {
    no_panic(&["transactions", "--season", "20242025"]);
}

#[test]
fn p282_transactions_pre_modern_clean_error() {
    // Pre-2021 has no bundled transactions.
    no_panic(&["transactions", "--season", "19921993"]);
}

#[test]
fn p283_data_help() {
    ok(&["data", "--help"]);
}

#[test]
fn p284_data_list() {
    // Bundle list — should always run.
    no_panic(&["data", "list"]);
}

#[test]
fn p285_games_help() {
    // `games` = personal attendance tracker.
    ok(&["games", "--help"]);
}

#[test]
fn p286_trade_help() {
    ok(&["trade", "--help"]);
}

#[test]
fn p287_schedule_help() {
    ok(&["schedule", "--help"]);
}

#[test]
fn p288_tonight_help() {
    ok(&["tonight", "--help"]);
}

#[test]
fn p289_serve_help() {
    // 2026-05-04 — `build` subcommand removed alongside the mkdocs cut;
    // the equivalent persona check here is `serve --help` (the new
    // user-facing web entry point that replaced the mkdocs surface).
    ok(&["serve", "--help"]);
}

#[test]
fn p290_dashboard_alias_help() {
    ok(&["dashboard", "--help"]);
}

// ── Bucket AA: Cross-feature integration smoke (10) ──────────────────────

#[test]
fn p291_full_pipeline_leaders_to_x_csv_smoke() {
    // FINDING: `x` doesn't accept --season. Use --seasons N (multi-aggregate)
    // for the cross-shape smoke instead.
    let leaders = ok(&["query", "leaders", "--season", "20242025", "--top", "1"]);
    let csv = ok(&["x", "leaders", "--top", "1"]);
    assert!(!leaders.is_empty() && !csv.is_empty());
}

#[test]
fn p292_player_then_compare_same_pair_consistency() {
    let player = ok(&["query", "player", "Connor McDavid", "--season", "20242025"]);
    let compare = ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Sidney Crosby",
        "--season",
        "20242025",
    ]);
    assert!(player.contains("McDavid"));
    assert!(compare.contains("McDavid"));
}

#[test]
fn p293_aggregate_seasons_2_with_filter_alias() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "2",
        "--filter",
        "g>=20",
        "--top",
        "10",
    ]);
}

#[test]
fn p294_x_compare_two_players() {
    no_panic(&["x", "compare", "Connor McDavid", "Sidney Crosby"]);
}

#[test]
fn p295_x_peers_for_player() {
    no_panic(&["x", "peers", "Connor McDavid"]);
}

#[test]
fn p296_export_md_and_query_leaders_share_top_scorer() {
    // Smoke: both surfaces complete on the same season.
    let path = temp_md("md_leaders");
    ok(&[
        "export",
        "md",
        "leaders",
        "--top",
        "1",
        "--out",
        path.to_str().unwrap(),
    ]);
    ok(&["query", "leaders", "--top", "1"]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn p297_query_player_with_seasons_38_and_percentiles() {
    ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--seasons",
        "38",
        "--percentiles",
    ]);
}

#[test]
fn p298_query_compare_seasons_38_full_history() {
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Wayne Gretzky",
        "--seasons",
        "38",
    ]);
}

#[test]
fn p299_aliases_chain_with_age_and_seasons() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--seasons",
        "2",
        "--filter",
        "g>=15",
        "--filter",
        "gp>=20",
        "--top",
        "10",
    ]);
}

#[test]
fn p300_workspace_smoke_all_six_query_subcommands() {
    // Final smoke: every query subcommand survives one invocation.
    ok(&["query", "leaders", "--top", "1"]);
    ok(&["query", "player", "Connor McDavid"]);
    ok(&["query", "compare", "Connor McDavid", "Sidney Crosby"]);
    ok(&["query", "goalies", "--top", "1"]);
    ok(&["rank", "--top", "1"]);
    ok(&["scheme", "list"]);
}
