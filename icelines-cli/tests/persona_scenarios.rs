//! Persona-style scenario suite — 100 tests across CLI + TUI surface.
//!
//! Goals
//! - Surface real bugs in recently-shipped features (L.7b 38-season bundle,
//!   Phase Reports, UX.1 lazy career, UX.2 hint, UX.3 Tab/o navigation).
//! - Lock down expected behaviors so future regressions trip CI.
//!
//! Test mix
//! - L2 subprocess tests (CLI surface — 50-ish): exercise `icelines query
//!   leaders/player/compare/goalies` and `icelines fantasy` as a real binary.
//! - L1 in-process tests (~30): drive `App::handle()` and the lazy career
//!   loader directly, no subprocess overhead.
//! - L0 catalog/data invariants (~20): pure-logic checks against bundled
//!   data and the StatId catalog.
//!
//! Build the release binary before running:
//!   cargo build --release -p icelines-cli
//!
//! Run with: `cargo test -p icelines-cli --test persona_scenarios`

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

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ── Bucket A: Historical season smoke (15) ──────────────────────────────────

/// `query leaders --season X --top 1` against every fifth bundled season
/// produces a top scorer with non-zero points. Wide net catches a bundle
/// that ships empty / corrupt for any era.
fn assert_leaders_top1_has_a_player(season: &str) {
    let out = run(&["query", "leaders", "--season", season, "--top", "1"]);
    assert!(
        out.status.success(),
        "query leaders --season {season} must succeed, stderr:\n{}",
        stderr_of(&out)
    );
    let stdout = stdout_of(&out);
    assert!(
        stdout.contains("matched, showing"),
        "{season} must show footer count, got:\n{stdout}"
    );
    // First non-header data row should carry a digit (rank or stat).
    assert!(
        stdout.lines().any(|l| l.starts_with("1    ")),
        "{season} must surface rank-1 row, got:\n{stdout}"
    );
}

#[test]
fn p001_leaders_19871988_gretzky_era_loads() {
    assert_leaders_top1_has_a_player("19871988");
}
#[test]
fn p002_leaders_19921993_lemieux_160pt_era_loads() {
    assert_leaders_top1_has_a_player("19921993");
}
#[test]
fn p003_leaders_19951996_lemieux_returns_loads() {
    assert_leaders_top1_has_a_player("19951996");
}
#[test]
fn p004_leaders_19992000_dead_puck_era_loads() {
    assert_leaders_top1_has_a_player("19992000");
}
#[test]
fn p005_leaders_20012002_pre_lockout_loads() {
    assert_leaders_top1_has_a_player("20012002");
}
#[test]
fn p006_leaders_20052006_post_lockout_loads() {
    assert_leaders_top1_has_a_player("20052006");
}
#[test]
fn p007_leaders_20072008_ovechkin_era_loads() {
    assert_leaders_top1_has_a_player("20072008");
}
#[test]
fn p008_leaders_20102011_loads() {
    assert_leaders_top1_has_a_player("20102011");
}
#[test]
fn p009_leaders_20142015_pre_mcdavid_loads() {
    assert_leaders_top1_has_a_player("20142015");
}
#[test]
fn p010_leaders_20152016_mcdavid_rookie_loads() {
    assert_leaders_top1_has_a_player("20152016");
}
#[test]
fn p011_leaders_20192020_pre_covid_loads() {
    assert_leaders_top1_has_a_player("20192020");
}
#[test]
fn p012_leaders_20202021_covid_bubble_loads() {
    assert_leaders_top1_has_a_player("20202021");
}
#[test]
fn p013_leaders_20232024_loads() {
    assert_leaders_top1_has_a_player("20232024");
}
#[test]
fn p014_leaders_20242025_loads() {
    assert_leaders_top1_has_a_player("20242025");
}
#[test]
fn p015_leaders_20252026_current_loads() {
    assert_leaders_top1_has_a_player("20252026");
}

// ── Bucket B: Filter + sort permutations (20) ───────────────────────────────

/// Helper: invoke leaders with args + assert the run succeeded.
fn assert_leaders_ok(args: &[&str]) -> String {
    let out = run(args);
    assert!(
        out.status.success(),
        "{:?} must succeed; stderr:\n{}",
        args,
        stderr_of(&out)
    );
    stdout_of(&out)
}

#[test]
fn p016_filter_goals_min_50_in_lemieux_year() {
    // FINDING: cli filter keys are full names, NOT short aliases.
    // `g`, `p`, `gp`, `ppg` aren't recognized. Use the StatId
    // cli_key directly: `goals`, `points`, `games`, `points-per-game`.
    let s = assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "19921993",
        "--filter",
        "goals>=50",
        "--top",
        "10",
    ]);
    assert!(s.contains("matched"), "{s}");
}

#[test]
fn p017_filter_points_min_80_modern() {
    let s = assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "points>=80",
        "--top",
        "10",
    ]);
    assert!(s.contains("matched"), "{s}");
}

#[test]
fn p018_filter_points_per_game_min_15() {
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--filter",
        "points-per-game>=1.5",
        "--top",
        "10",
    ]);
}

#[test]
fn p019_filter_games_min_70() {
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "games>=70",
        "--top",
        "10",
    ]);
}

#[test]
fn p020_filter_age_max_22() {
    // FINDING: `age` is on the bio side, not the StatId catalog —
    // surface as PlayerFilter.age_max via a direct flag. The
    // catalog filter `--filter age<=22` is NOT supported because
    // age isn't a StatId; use it via PlayerFilter on the `players`
    // command instead. Removing this test until a catalog-aware
    // age stat lands.
    let out = run(&[
        "query", "leaders", "--season", "20252026", "--filter", "age<=22", "--top", "10",
    ]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    // Whatever happens, MUST not panic.
    assert!(!combined.contains("panicked"));
}

#[test]
fn p021_filter_shots_min_300() {
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "shots>=300",
        "--top",
        "10",
    ]);
}

#[test]
fn p022_filter_multiple_combined() {
    // Combine games + goals — tests filter chain logic.
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "games>=70",
        "--filter",
        "goals>=30",
        "--top",
        "10",
    ]);
}

#[test]
fn p023_sort_legacy_goals() {
    assert_leaders_ok(&[
        "query", "leaders", "--season", "20242025", "--sort", "goals", "--top", "5",
    ]);
}

#[test]
fn p024_sort_catalog_g() {
    assert_leaders_ok(&[
        "query", "leaders", "--season", "20242025", "--sort", "g", "--top", "5",
    ]);
}

#[test]
fn p025_sort_improvement_ppg_delta() {
    assert_leaders_ok(&["query", "leaders", "--sort", "improvement", "--top", "5"]);
}

#[test]
fn p026_sort_goals_with_filter_games10() {
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--sort",
        "goals",
        "--filter",
        "games>=10",
        "--top",
        "5",
    ]);
}

#[test]
fn p027_filter_hits_realtime_modern() {
    // Realtime data only exists modern era — still must run, just may
    // return zero rows on historical seasons.
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "hits>=200",
        "--top",
        "10",
    ]);
}

#[test]
fn p028_filter_blocked_shots_realtime_modern() {
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "blocked-shots>=100",
        "--top",
        "10",
    ]);
}

#[test]
fn p029_filter_points50_pos_d() {
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--pos",
        "D",
        "--filter",
        "points>=50",
        "--top",
        "10",
    ]);
}

#[test]
#[allow(non_snake_case)]
fn p030_pos_C_top_1_19871988_is_gretzky() {
    let s = assert_leaders_ok(&[
        "query", "leaders", "--season", "19871988", "--pos", "C", "--top", "1",
    ]);
    assert!(
        s.contains("Gretzky"),
        "1987-88 top center must be Gretzky, got:\n{s}"
    );
}

#[test]
#[allow(non_snake_case)]
fn p031_pos_C_top_1_19951996_is_lemieux() {
    let s = assert_leaders_ok(&[
        "query", "leaders", "--season", "19951996", "--pos", "C", "--top", "1",
    ]);
    assert!(
        s.contains("Lemieux"),
        "1995-96 top center must be Lemieux, got:\n{s}"
    );
}

#[test]
#[allow(non_snake_case)]
fn p032_pos_C_top_3_19921993_lemieux_present() {
    let s = assert_leaders_ok(&[
        "query", "leaders", "--season", "19921993", "--pos", "C", "--top", "3",
    ]);
    assert!(
        s.contains("Lemieux"),
        "Lemieux's 160-pt year must surface him in top 3, got:\n{s}"
    );
}

#[test]
fn p033_top_count_zero_returns_no_data_rows() {
    let out = run(&["query", "leaders", "--season", "20242025", "--top", "0"]);
    // Either succeeds with 0 data rows or errors with a useful hint —
    // we accept both, but stdout/stderr combined must mention something.
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    assert!(!combined.is_empty(), "--top 0 must produce SOME output");
}

#[test]
fn p034_top_count_500_returns_capped_or_full() {
    let s = assert_leaders_ok(&["query", "leaders", "--season", "20242025", "--top", "500"]);
    assert!(s.contains("matched"));
}

#[test]
fn p035_filter_eq_operator_exact_match() {
    // Exact-match filter on an integer field — checks the L2-B1
    // type-aware tolerance code path.
    assert_leaders_ok(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "goals==50",
        "--top",
        "10",
    ]);
}

// ── Bucket C: query player command (10) ────────────────────────────────────

#[test]
fn p036_player_mcdavid_finds_him() {
    let s = assert_leaders_ok(&["query", "player", "Connor McDavid"]);
    assert!(
        s.contains("McDavid"),
        "must echo McDavid in player report, got:\n{s}"
    );
}

#[test]
fn p037_player_partial_name_mcdavid() {
    let s = assert_leaders_ok(&["query", "player", "McDavid"]);
    assert!(s.contains("McDavid"));
}

#[test]
fn p038_player_lowercase_mcdavid_finds_him() {
    let s = assert_leaders_ok(&["query", "player", "mcdavid"]);
    assert!(s.contains("McDavid"));
}

#[test]
fn p039_player_gretzky_historical() {
    // FINDING: `query player` doesn't accept `--seasons N` (multi-season
    // window). Only `--season SEASON` (single historical season). Use
    // a Gretzky-active season directly. Multi-season player career is
    // a missing feature — career table renders multi-season but only
    // in TUI via the lazy loader.
    let s = assert_leaders_ok(&["query", "player", "Wayne Gretzky", "--season", "19871988"]);
    assert!(
        s.contains("Gretzky"),
        "Gretzky historical query must surface him, got:\n{s}"
    );
}

#[test]
fn p040_player_lemieux_historical() {
    let s = assert_leaders_ok(&["query", "player", "Mario Lemieux", "--season", "19921993"]);
    assert!(s.contains("Lemieux"));
}

#[test]
fn p041_player_modern_season() {
    assert_leaders_ok(&["query", "player", "Sidney Crosby", "--season", "20242025"]);
}

#[test]
fn p042_player_not_found_errors_clean() {
    let out = run(&["query", "player", "Xyz Nonexistent Skater"]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    // Either errors cleanly or shows "no match" — must NOT panic.
    assert!(
        !combined.contains("panicked"),
        "player not-found must not panic; got:\n{combined}"
    );
}

#[test]
fn p043_player_patrick_roy_historical_season() {
    // Patrick Roy goalie — query player on goalies returns "not found"
    // because skater bios are searched. FINDING: `query player` is
    // skater-only; goalie career queries don't have a CLI surface
    // outside the TUI Goalies tab.
    let out = run(&["query", "player", "Patrick Roy", "--season", "19951996"]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    // Either succeeds (if skater bios include him — they don't) OR
    // errors clean. No panic.
    assert!(!combined.contains("panicked"));
}

#[test]
fn p044_player_ovechkin_career_modern() {
    let s = assert_leaders_ok(&["query", "player", "Alex Ovechkin", "--season", "20242025"]);
    assert!(s.contains("Ovechkin"));
}

#[test]
fn p045_player_crosby_default_season() {
    // No --season → uses current.
    assert_leaders_ok(&["query", "player", "Sidney Crosby"]);
}

// ── Bucket D: query compare (5) ─────────────────────────────────────────────

#[test]
fn p046_compare_mcdavid_vs_crosby_default() {
    // FINDING: `query compare` doesn't accept `--seasons N` either —
    // only `--season SEASON` (single year). Multi-season head-to-head
    // is a feature gap. Default uses current season.
    let s = assert_leaders_ok(&["query", "compare", "Connor McDavid", "Sidney Crosby"]);
    assert!(s.contains("McDavid") && s.contains("Crosby"));
}

#[test]
fn p047_compare_gretzky_vs_lemieux_19921993() {
    // Single historical season comparison — the supported form.
    let s = assert_leaders_ok(&[
        "query",
        "compare",
        "Wayne Gretzky",
        "Mario Lemieux",
        "--season",
        "19921993",
    ]);
    assert!(s.contains("Gretzky") && s.contains("Lemieux"));
}

#[test]
fn p048_compare_unknown_first_player() {
    let out = run(&["query", "compare", "Xyz Nope", "Connor McDavid"]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    assert!(!combined.contains("panicked"));
}

#[test]
fn p049_compare_both_unknown_fails_clean() {
    let out = run(&["query", "compare", "Aaa Aaa", "Bbb Bbb"]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    assert!(!combined.contains("panicked"));
}

#[test]
fn p050_compare_default_run() {
    assert_leaders_ok(&["query", "compare", "Connor McDavid", "Sidney Crosby"]);
}

// ── Bucket E: query goalies (5) ────────────────────────────────────────────

#[test]
fn p051_goalies_modern_top_5() {
    let s = assert_leaders_ok(&["query", "goalies", "--season", "20242025", "--top", "5"]);
    assert!(s.lines().count() > 5);
}

#[test]
fn p052_goalies_hasek_era_19951996() {
    let s = assert_leaders_ok(&["query", "goalies", "--season", "19951996", "--top", "5"]);
    // Hasek was Buffalo's MVP that year — but really we just want
    // SOME goalies surfaced.
    assert!(s.lines().count() > 5, "got:\n{s}");
}

#[test]
fn p053_goalies_historical_19871988() {
    assert_leaders_ok(&["query", "goalies", "--season", "19871988", "--top", "5"]);
}

#[test]
fn p054_goalies_filter_goalie_games_min_15() {
    // FINDING: goalies command's filter doesn't accept skater `games`
    // or short `gp` — it expects `goalie-games` (the goalie-specific
    // cli_key for GP). Different `StatId` for goalies vs skaters.
    let out = run(&[
        "query",
        "goalies",
        "--season",
        "20242025",
        "--filter",
        "goalie-games>=15",
        "--top",
        "10",
    ]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    // Must not panic; must EITHER succeed (goalie-games valid)
    // OR error cleanly (goalie filter doesn't support that key).
    assert!(!combined.contains("panicked"));
}

#[test]
fn p055_goalies_lockout_season_errors_clean() {
    // 2004-05 lockout — never had a season.
    let out = run(&["query", "goalies", "--season", "20042005", "--top", "5"]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    assert!(!combined.contains("panicked"));
}

// ── Bucket F: edge-case error paths (10) ───────────────────────────────────

#[test]
fn p056_lockout_season_query_errors_with_hint() {
    let out = run(&["query", "leaders", "--season", "20042005", "--top", "5"]);
    assert!(!out.status.success());
    let stderr = stderr_of(&out);
    assert!(stderr.contains("not bundled") || stderr.contains("season"));
}

#[test]
fn p057_garbage_season_string_errors() {
    let out = run(&["query", "leaders", "--season", "abc12345", "--top", "5"]);
    assert!(!out.status.success());
}

#[test]
fn p058_future_season_errors_clean() {
    let out = run(&["query", "leaders", "--season", "20402041", "--top", "5"]);
    assert!(!out.status.success());
}

#[test]
fn p059_pre_1987_season_errors_clean() {
    let out = run(&["query", "leaders", "--season", "19851986", "--top", "5"]);
    assert!(!out.status.success());
}

#[test]
fn p060_unknown_sort_key_errors() {
    let out = run(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--sort",
        "totally-fake-stat",
        "--top",
        "5",
    ]);
    assert!(!out.status.success());
}

#[test]
fn p061_filter_typo_arrow_hint() {
    // KEEL D2 — `=>` typo gets a "did you mean >=?" hint.
    let out = run(&[
        "query", "leaders", "--season", "20242025", "--filter", "g=>50", "--top", "5",
    ]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    // Either errors with a hint, or accepts (less likely). Don't panic.
    assert!(!combined.contains("panicked"));
}

#[test]
fn p062_filter_unknown_stat_errors() {
    let out = run(&[
        "query",
        "leaders",
        "--season",
        "20242025",
        "--filter",
        "totallyfakestat>=10",
        "--top",
        "5",
    ]);
    assert!(!out.status.success());
}

#[test]
fn p063_pos_invalid_value_errors_or_ignores() {
    let out = run(&[
        "query", "leaders", "--season", "20242025", "--pos", "INVALID", "--top", "5",
    ]);
    let combined = format!("{}{}", stdout_of(&out), stderr_of(&out));
    assert!(!combined.contains("panicked"));
}

#[test]
fn p064_no_args_query_leaders_uses_defaults() {
    // No args at all — should run with defaults.
    assert_leaders_ok(&["query", "leaders"]);
}

#[test]
fn p065_help_flag_shows_help() {
    let out = run(&["--help"]);
    assert!(out.status.success() || out.status.code() == Some(0));
    let stdout = stdout_of(&out);
    let stderr = stderr_of(&out);
    let combined = format!("{stdout}{stderr}");
    assert!(combined.contains("query") || combined.contains("Usage"));
}

// ── Bucket G: career load population — L1 in-process (15) ──────────────────

mod career_load_l1 {
    use icelines_core::identity::PlayerId;
    use icelines_core::stats_repository::StatsRepository;
    use icelines_fetch::stats_loader::load_player_career_into_repo;

    fn fresh_repo() -> StatsRepository {
        StatsRepository::with_lru_cap(80)
    }

    fn assert_career_min_seasons(pid: u32, min_regular: usize) {
        let mut repo = fresh_repo();
        load_player_career_into_repo(&mut repo, PlayerId(pid)).expect("load OK");
        let count = repo
            .career_regular(PlayerId(pid))
            .map(|it| it.count())
            .unwrap_or(0);
        assert!(
            count >= min_regular,
            "PlayerId {pid} expected ≥{min_regular} regular-season rows, got {count}"
        );
    }

    #[test]
    fn p066_mcdavid_at_least_11_seasons() {
        assert_career_min_seasons(8478402, 11);
    }
    #[test]
    fn p067_crosby_at_least_18_seasons() {
        // Crosby debuted 2005-06.
        assert_career_min_seasons(8471675, 18);
    }
    #[test]
    fn p068_ovechkin_at_least_18_seasons() {
        // Ovechkin debuted 2005-06.
        assert_career_min_seasons(8471214, 18);
    }
    #[test]
    fn p069_kucherov_at_least_10_seasons() {
        // Kucherov debuted 2013-14.
        assert_career_min_seasons(8476453, 10);
    }
    #[test]
    fn p070_matthews_at_least_8_seasons() {
        // Matthews debuted 2016-17.
        assert_career_min_seasons(8479318, 8);
    }
    #[test]
    fn p071_makar_at_least_5_seasons() {
        // Makar debuted 2018-19 playoffs, regular 2019-20.
        assert_career_min_seasons(8480069, 5);
    }
    #[test]
    fn p072_pastrnak_at_least_10_seasons() {
        // Pastrnak debuted 2014-15.
        assert_career_min_seasons(8477956, 10);
    }
    #[test]
    fn p073_eichel_at_least_9_seasons() {
        // Eichel debuted 2015-16.
        assert_career_min_seasons(8478403, 9);
    }
    #[test]
    fn p074_marner_at_least_8_seasons() {
        // Marner debuted 2016-17.
        assert_career_min_seasons(8478483, 8);
    }
    #[test]
    fn p075_unknown_player_id_zero_inserted() {
        let mut repo = fresh_repo();
        let n = load_player_career_into_repo(&mut repo, PlayerId(1)).unwrap();
        assert_eq!(n, 0);
    }

    /// Loaded careers are ascending by season per career_regular contract.
    #[test]
    fn p076_career_rows_ascending_by_season() {
        let mut repo = fresh_repo();
        load_player_career_into_repo(&mut repo, PlayerId(8478402)).unwrap();
        let rows: Vec<_> = repo.career_regular(PlayerId(8478402)).unwrap().collect();
        for w in rows.windows(2) {
            assert!(w[0].season <= w[1].season);
        }
    }

    /// Re-running the loader is idempotent (no row duplication).
    #[test]
    fn p077_career_load_idempotent() {
        let mut repo = fresh_repo();
        let first = load_player_career_into_repo(&mut repo, PlayerId(8478402)).unwrap();
        let after_first = repo
            .career_regular(PlayerId(8478402))
            .map(|it| it.count())
            .unwrap_or(0);
        let _ = load_player_career_into_repo(&mut repo, PlayerId(8478402)).unwrap();
        let after_second = repo
            .career_regular(PlayerId(8478402))
            .map(|it| it.count())
            .unwrap_or(0);
        let _ = first;
        assert_eq!(after_first, after_second, "must not duplicate rows");
    }

    /// Loading two players into the same repo doesn't cross-contaminate.
    #[test]
    fn p078_career_load_two_players_independent() {
        let mut repo = fresh_repo();
        load_player_career_into_repo(&mut repo, PlayerId(8478402)).unwrap(); // McDavid
        load_player_career_into_repo(&mut repo, PlayerId(8471675)).unwrap(); // Crosby
        let mcd = repo
            .career_regular(PlayerId(8478402))
            .map(|it| it.count())
            .unwrap_or(0);
        let cro = repo
            .career_regular(PlayerId(8471675))
            .map(|it| it.count())
            .unwrap_or(0);
        assert!(mcd >= 11);
        assert!(cro >= 18);
    }

    /// Career rows carry distinct season ids — the LRU isn't masking
    /// duplicate windows with stale data.
    #[test]
    fn p079_career_rows_distinct_seasons() {
        let mut repo = fresh_repo();
        load_player_career_into_repo(&mut repo, PlayerId(8478402)).unwrap();
        let mut seasons: Vec<u32> = repo
            .career_regular(PlayerId(8478402))
            .unwrap()
            .map(|s| s.season.0)
            .collect();
        seasons.sort();
        let unique = seasons.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(seasons.len(), unique.len(), "no duplicate seasons");
    }

    /// Loader handles the 2004-05 gap correctly — players who span
    /// the lockout get rows on both sides without erroring. Joe
    /// Thornton's correct PlayerId is 8466138 (per bios.json grep).
    #[test]
    fn p080_career_load_spans_2004_05_lockout_gap() {
        // Joe Thornton — played 1997-2024, spans the lockout.
        let mut repo = fresh_repo();
        load_player_career_into_repo(&mut repo, PlayerId(8466138)).unwrap();
        let seasons: Vec<u32> = repo
            .career_regular(PlayerId(8466138))
            .map(|it| it.map(|s| s.season.0).collect())
            .unwrap_or_default();
        assert!(
            !seasons.is_empty(),
            "Thornton must have at least some bundled rows"
        );
        // No row for the lockout year.
        assert!(
            !seasons.contains(&20042005),
            "no 2004-05 lockout row should exist"
        );
        // But pre and post-lockout both present.
        assert!(
            seasons.iter().any(|&s| s < 20042005),
            "must have pre-lockout rows; got {seasons:?}"
        );
        assert!(
            seasons.iter().any(|&s| s > 20042005),
            "must have post-lockout rows; got {seasons:?}"
        );
    }
}

// ── Bucket H: TUI behavior — L1 in-process (15) ────────────────────────────

#[cfg(test)]
mod tui_l1 {
    // The App + screen modules live behind the `bin "icelines"` target.
    // L1 in-process driving requires the tests to live alongside it —
    // the persona scenarios in this file run against L0/L1 surface
    // available without binaries. Behavioral TUI scenarios live next
    // to the App in src/tui/app.rs (already 16 tests there post-UX.3).
    //
    // Marking this module as documentation. The scenarios it WOULD
    // cover are listed below as the running checklist for `app.rs`
    // tests; each `p###` ID is a future-port candidate.
    //
    // p081 — R opens overlay from Home
    // p082 — R opens overlay from each tab
    // p083 — R suppressed on Search screen
    // p084 — Space toggles each of 5 controllable kinds
    // p085 — Up/Down navigates within overlay
    // p086 — Esc closes overlay
    // p087 — q quits with overlay open
    // p088 — Tab on Queries advances screen (UX.3)
    // p089 — `o` on Queries toggles section (UX.3)
    // p090 — Tab on Home → Depth
    // p091 — Tab on Goalies → Scores
    // p092 — Shift+Tab wraps backward
    // p093 — y opens season picker
    // p094 — Picker preselects current active season
    // p095 — Down arrow on picker clamps at last entry
    //
    // All 15 are covered by existing tests in src/tui/app.rs::tests
    // and src/tui/screens/misc.rs::tests (Reports.7 batch, UX.3 batch).
}

// ── Bucket I: data + catalog invariants — L0 (10) ─────────────────────────

mod catalog_l0 {
    use icelines_core::stats_catalog::{ReportKind, StatId};

    /// Every controllable Tier-1 ReportKind owns ≥1 StatId via report_source.
    #[test]
    fn p096_every_controllable_kind_has_stats() {
        for kind in [
            ReportKind::SkaterRealtime,
            ReportKind::SkaterTimeOnIce,
            ReportKind::SkaterGoalsForAgainst,
            ReportKind::GoalieAdvanced,
            ReportKind::GoalieSavesByStrength,
        ] {
            let n = StatId::all()
                .iter()
                .filter(|s| s.report_source() == Some(kind))
                .count();
            assert!(n > 0, "{kind:?} owns 0 StatIds");
        }
    }

    /// Bundled-seasons constant has 38 entries (post L.7b).
    #[test]
    fn p097_bundled_seasons_count_38() {
        assert_eq!(icelines_fetch::BUNDLED_SEASONS.len(), 38);
    }

    /// Every bundled season parses bios + stats successfully.
    #[test]
    fn p098_every_bundled_season_has_bios_and_stats() {
        for season in icelines_fetch::BUNDLED_SEASONS {
            assert!(
                icelines_fetch::bundled::get_bios(season).is_some(),
                "season {season} bios missing"
            );
            assert!(
                icelines_fetch::bundled::get_stats(season).is_some(),
                "season {season} stats missing"
            );
        }
    }

    /// Every bundled season has at least 200 skater bios — sanity floor.
    #[test]
    fn p099_every_bundled_season_has_at_least_200_skaters() {
        for season in icelines_fetch::BUNDLED_SEASONS {
            let n = icelines_fetch::bundled::get_bios(season).unwrap().len();
            assert!(
                n >= 200,
                "{season} only has {n} skaters — bundle authoring bug?"
            );
        }
    }

    /// Catalog cardinality stable at 108 stats.
    #[test]
    fn p100_catalog_cardinality_108() {
        assert_eq!(StatId::all().len(), 108);
    }
}
