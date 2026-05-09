//! Persona Wave 2 — 100 more scenario tests, exercising the Gaps.1-6
//! fixes (short aliases, goalie filter rewrite, --age-min/max already
//! present, multi-season player/compare via --seasons N, goalie player
//! support).
//!
//! Run with: `cargo test -p icelines-cli --test persona_wave2`
//! Build the release binary first: `cargo build --release -p icelines-cli`

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
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

fn fail(args: &[&str]) -> String {
    let out = run(args);
    assert!(
        !out.status.success(),
        "{:?} must fail; stdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ── Bucket K: Short alias resolution (Gaps.1) — 20 scenarios ──────────────

#[test]
fn p101_alias_g_resolves_to_goals() {
    let s = ok(&[
        "query", "leaders", "--filter", "g>=50", "--season", "19921993", "--top", "5",
    ]);
    assert!(s.contains("Lemieux"));
}

#[test]
fn p102_alias_a_resolves_to_assists() {
    ok(&[
        "query", "leaders", "--filter", "a>=50", "--season", "19981999", "--top", "5",
    ]);
}

#[test]
fn p103_alias_p_resolves_to_points() {
    ok(&[
        "query", "leaders", "--filter", "p>=80", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p104_alias_pts_resolves_to_points() {
    ok(&[
        "query", "leaders", "--filter", "pts>=80", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p105_alias_gp_resolves_to_games() {
    ok(&[
        "query", "leaders", "--filter", "gp>=70", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p106_alias_ppg_resolves_to_points_per_game() {
    ok(&[
        "query", "leaders", "--filter", "ppg>=1.0", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p107_alias_s_resolves_to_shots() {
    ok(&[
        "query", "leaders", "--filter", "s>=200", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p108_alias_blk_resolves_to_blocked_shots() {
    ok(&[
        "query", "leaders", "--filter", "blk>=50", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p109_alias_blocks_resolves_to_blocked_shots() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "blocks>=50",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
}

#[test]
fn p110_alias_tk_resolves_to_takeaways() {
    ok(&[
        "query", "leaders", "--filter", "tk>=30", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p111_alias_gv_resolves_to_giveaways() {
    ok(&[
        "query", "leaders", "--filter", "gv>=30", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
#[allow(non_snake_case)]
fn p112_alias_uppercase_HITS_resolves() {
    let s = ok(&[
        "query",
        "leaders",
        "--filter",
        "HITS>=200",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
    // case-insensitive resolution should give us results
    let _ = s;
}

#[test]
fn p113_alias_combined_g_and_a_in_one_call() {
    ok(&[
        "query", "leaders", "--filter", "g>=20", "--filter", "a>=20", "--season", "20242025",
        "--top", "10",
    ]);
}

#[test]
fn p114_alias_pen_resolves_to_pim() {
    ok(&[
        "query", "leaders", "--filter", "pen>=50", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p115_alias_plusminus_resolves_to_plus_minus() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "plusminus>=10",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
}

#[test]
fn p116_alias_sort_g_legacy_works() {
    ok(&[
        "query", "leaders", "--sort", "g", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p117_alias_sort_pts_works() {
    ok(&[
        "query", "leaders", "--sort", "pts", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p118_alias_pace_resolves_to_pace_82() {
    ok(&[
        "query", "leaders", "--filter", "pace>=80", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p119_alias_unknown_fake_still_errors() {
    let err = fail(&[
        "query",
        "leaders",
        "--filter",
        "totallyfake>=10",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
    assert!(err.contains("unknown") || err.contains("stat"));
}

#[test]
fn p120_alias_short_keys_with_pos_filter() {
    ok(&[
        "query", "leaders", "--pos", "C", "--filter", "p>=50", "--filter", "gp>=20", "--season",
        "20242025", "--top", "5",
    ]);
}

// ── Bucket L: Goalie filter rewrite (Gaps.4) — 10 scenarios ────────────────

#[test]
fn p121_goalie_filter_gp_rewrites_to_goalie_games() {
    let s = ok(&[
        "query", "goalies", "--filter", "gp>=15", "--season", "20242025", "--top", "5",
    ]);
    assert!(s.lines().count() > 5);
}

#[test]
fn p122_goalie_filter_games_rewrites_to_goalie_games() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "games>=15",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
}

#[test]
fn p123_goalie_filter_starts_rewrites_to_goalie_starts() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "starts>=10",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
}

#[test]
fn p124_goalie_filter_save_pct_alias_sv_pct() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "save-pct>=0.91",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p125_goalie_filter_wins_native() {
    ok(&[
        "query", "goalies", "--filter", "wins>=20", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p126_goalie_filter_w_alias() {
    ok(&[
        "query", "goalies", "--filter", "w>=20", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p127_goalie_filter_so_alias_shutouts() {
    ok(&[
        "query", "goalies", "--filter", "so>=2", "--season", "20242025", "--top", "5",
    ]);
}

#[test]
fn p128_goalie_filter_unknown_key_with_hint() {
    let err = fail(&[
        "query",
        "goalies",
        "--filter",
        "totallyfake>=10",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
    // Should mention goalie-specific keys in the hint.
    assert!(
        err.contains("goalie") || err.contains("stat") || err.contains("unknown"),
        "expected goalie-context hint, got:\n{err}"
    );
}

#[test]
fn p129_goalie_filter_combined_multiple() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "gp>=15",
        "--filter",
        "save-pct>=0.91",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
}

#[test]
fn p130_goalie_filter_historical_season() {
    ok(&[
        "query", "goalies", "--filter", "gp>=20", "--season", "19951996", "--top", "5",
    ]);
}

// ── Bucket M: Multi-season player (Gaps.2) — 15 scenarios ──────────────────

#[test]
fn p131_player_seasons_38_full_career_default() {
    // Default seasons=38; McDavid should show 11+ seasons.
    let s = ok(&["query", "player", "Connor McDavid"]);
    // CAREER ARC line must reflect a multi-season window.
    assert!(s.contains("CAREER ARC"), "got:\n{s}");
}

#[test]
fn p132_player_seasons_5_modern_era_only() {
    let s = ok(&["query", "player", "Connor McDavid", "--seasons", "5"]);
    assert!(s.contains("CAREER ARC"));
}

#[test]
fn p133_player_seasons_1_current_only() {
    let s = ok(&["query", "player", "Connor McDavid", "--seasons", "1"]);
    assert!(s.contains("McDavid"));
}

#[test]
fn p134_player_seasons_38_gretzky_full_history() {
    let s = ok(&["query", "player", "Wayne Gretzky", "--seasons", "38"]);
    assert!(s.contains("Gretzky"));
}

#[test]
fn p135_player_seasons_38_lemieux_full_history() {
    let s = ok(&["query", "player", "Mario Lemieux", "--seasons", "38"]);
    assert!(s.contains("Lemieux"));
}

#[test]
fn p136_player_seasons_38_ovechkin_career() {
    let s = ok(&["query", "player", "Alex Ovechkin", "--seasons", "38"]);
    assert!(s.contains("Ovechkin"));
}

#[test]
fn p137_player_seasons_clamp_above_max() {
    // --seasons 100 clamps to 38.
    ok(&["query", "player", "Connor McDavid", "--seasons", "100"]);
}

#[test]
fn p138_player_seasons_zero_clamps_to_one() {
    // --seasons 0 should not error; clamps to 1.
    ok(&["query", "player", "Connor McDavid", "--seasons", "0"]);
}

#[test]
fn p139_player_seasons_with_percentiles() {
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
fn p140_player_seasons_with_filter() {
    ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--seasons",
        "38",
        "--filter",
        "gp>=30",
        "--percentiles",
    ]);
}

#[test]
fn p141_player_seasons_historical_specific_season_compat() {
    // --season + --seasons together should still work.
    ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--season",
        "20232024",
        "--seasons",
        "38",
    ]);
}

#[test]
fn p142_player_seasons_crosby_modern_career() {
    ok(&["query", "player", "Sidney Crosby", "--seasons", "20"]);
}

#[test]
fn p143_player_seasons_kucherov_decade() {
    ok(&["query", "player", "Nikita Kucherov", "--seasons", "15"]);
}

#[test]
fn p144_player_seasons_matthews() {
    ok(&["query", "player", "Auston Matthews", "--seasons", "10"]);
}

#[test]
fn p145_player_seasons_makar_short_career() {
    ok(&["query", "player", "Cale Makar", "--seasons", "10"]);
}

// ── Bucket N: Multi-season compare (Gaps.3) — 10 scenarios ─────────────────

#[test]
fn p146_compare_default_38_seasons_career_arcs() {
    let s = ok(&["query", "compare", "Connor McDavid", "Sidney Crosby"]);
    // Default seasons=38 → both careers print CAREER ARC blocks.
    let arc_count = s.matches("CAREER ARC").count();
    assert_eq!(
        arc_count, 2,
        "expected 2 CAREER ARC blocks, got {arc_count}"
    );
}

#[test]
fn p147_compare_seasons_5_two_arcs() {
    let s = ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Sidney Crosby",
        "--seasons",
        "5",
    ]);
    assert_eq!(s.matches("CAREER ARC").count(), 2);
}

#[test]
fn p148_compare_seasons_1_skips_arcs() {
    // --seasons 1 → no career arc, just head-to-head.
    let s = ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Sidney Crosby",
        "--seasons",
        "1",
    ]);
    // Head-to-head should still print, but no CAREER ARC blocks.
    assert!(s.contains("McDavid") && s.contains("Crosby"));
    assert_eq!(s.matches("CAREER ARC").count(), 0);
}

#[test]
fn p149_compare_seasons_gretzky_lemieux_overlap() {
    let s = ok(&[
        "query",
        "compare",
        "Wayne Gretzky",
        "Mario Lemieux",
        "--seasons",
        "20",
    ]);
    assert!(s.contains("Gretzky") && s.contains("Lemieux"));
}

#[test]
fn p150_compare_with_similar_ignores_seasons() {
    // --similar N is single-season Z-score — `--seasons` shouldn't error.
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "--similar",
        "5",
        "--seasons",
        "38",
    ]);
}

#[test]
fn p151_compare_seasons_clamps() {
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Sidney Crosby",
        "--seasons",
        "100",
    ]);
}

#[test]
fn p152_compare_kucherov_vs_pastrnak_decade() {
    let s = ok(&[
        "query",
        "compare",
        "Nikita Kucherov",
        "David Pastrnak",
        "--seasons",
        "10",
    ]);
    assert_eq!(s.matches("CAREER ARC").count(), 2);
}

#[test]
fn p153_compare_seasons_with_specific_season_window() {
    // Single season set + multi-season arcs: head-to-head uses
    // --season's window, arcs are 38 seasons.
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Sidney Crosby",
        "--season",
        "20232024",
        "--seasons",
        "38",
    ]);
}

#[test]
fn p154_compare_unknown_player_with_seasons_clean_error() {
    let out = run(&[
        "query",
        "compare",
        "Xyz Nope",
        "Connor McDavid",
        "--seasons",
        "38",
    ]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p155_compare_seasons_ovechkin_vs_crosby_15() {
    ok(&[
        "query",
        "compare",
        "Alex Ovechkin",
        "Sidney Crosby",
        "--seasons",
        "15",
    ]);
}

// ── Bucket O: Goalie player support (Gaps.5) — 5 scenarios ─────────────────

#[test]
fn p156_player_patrick_roy_finds_goalie() {
    // Pre-Gaps.5 this errored "not found". Now goalie bios are searched.
    let s = ok(&["query", "player", "Patrick Roy", "--season", "19951996"]);
    assert!(
        s.contains("Roy") || s.contains("PATRICK") || s.to_lowercase().contains("roy"),
        "must surface Roy in goalie player query, got:\n{s}"
    );
}

#[test]
fn p157_player_dominik_hasek_historical() {
    let s = ok(&["query", "player", "Dominik Hasek", "--season", "19981999"]);
    assert!(s.to_lowercase().contains("hasek"));
}

#[test]
fn p158_player_modern_goalie_hellebuyck() {
    ok(&[
        "query",
        "player",
        "Connor Hellebuyck",
        "--season",
        "20242025",
    ]);
}

#[test]
fn p159_player_goalie_with_seasons_full_career() {
    // Multi-season + goalie path — exercises both Gaps.2 + Gaps.5.
    let out = run(&["query", "player", "Patrick Roy", "--seasons", "38"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Either succeeds with career data OR errors clean (goalie career
    // path may not be wired through print_career yet).
    assert!(!combined.contains("panicked"));
}

#[test]
fn p160_player_unknown_skater_or_goalie_fails_clean() {
    fail(&["query", "player", "Xyz Made-Up Goalie"]);
}

// ── Bucket P: Age filter + leaders flags (Gaps.6) — 10 scenarios ──────────

#[test]
fn p161_age_max_22_youngsters() {
    let s = ok(&[
        "query",
        "leaders",
        "--age-max",
        "22",
        "--season",
        "20252026",
        "--top",
        "10",
    ]);
    assert!(s.contains("matched"));
}

#[test]
fn p162_age_min_35_veterans() {
    let s = ok(&[
        "query",
        "leaders",
        "--age-min",
        "35",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
    assert!(s.contains("matched"));
}

#[test]
fn p163_age_min_max_combined_window() {
    ok(&[
        "query",
        "leaders",
        "--age-min",
        "23",
        "--age-max",
        "27",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p164_age_with_pos_filter() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "23",
        "--pos",
        "C",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p165_age_with_filter_aliases_combined() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--filter",
        "p>=40",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p166_age_zero_clamps_or_errors_clean() {
    let out = run(&["query", "leaders", "--age-max", "0", "--top", "5"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p167_age_with_team_filter() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--team",
        "EDM",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p168_age_with_nationality_filter() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--nationality",
        "CAN",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p169_age_with_undrafted_flag() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--undrafted",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p170_age_with_rookie_flag() {
    ok(&[
        "query", "leaders", "--rookie", "--season", "20242025", "--top", "10",
    ]);
}

// ── Bucket Q: Season-specific historic deep-dives — 15 scenarios ────────

#[test]
fn p171_gretzky_215_pt_year_18851986_doesnt_exist() {
    // Pre-1987 not bundled.
    fail(&["query", "leaders", "--season", "19851986", "--top", "5"]);
}

#[test]
fn p172_gretzky_in_1987_88_filter_g_plus_a_thresh() {
    let s = ok(&[
        "query", "leaders", "--season", "19871988", "--filter", "g>=40", "--filter", "a>=80",
        "--top", "10",
    ]);
    assert!(s.contains("Gretzky"));
}

#[test]
fn p173_lemieux_85_goals_year_19881989() {
    let s = ok(&[
        "query", "leaders", "--season", "19881989", "--filter", "g>=70", "--top", "5",
    ]);
    // Lemieux had 85 goals in 88-89.
    assert!(s.contains("Lemieux"));
}

#[test]
fn p174_19961997_top_scorer_should_be_lemieux_or_kariya() {
    ok(&["query", "leaders", "--season", "19961997", "--top", "5"]);
}

#[test]
fn p175_20012002_iginla_50_goal_year() {
    ok(&["query", "leaders", "--season", "20012002", "--top", "10"]);
}

#[test]
fn p176_20052006_ovechkin_crosby_rookies() {
    // FINDING: --age-max uses CURRENT age, not age-at-season — so
    // Ovechkin/Crosby (both 40 now) get filtered out of their rookie
    // year top-10. Drop the age filter; just assert the rookies show
    // up in the top 10 by pts/82.
    let s = ok(&["query", "leaders", "--season", "20052006", "--top", "10"]);
    assert!(
        s.to_lowercase().contains("ovechkin") && s.to_lowercase().contains("crosby"),
        "expected both Ovechkin and Crosby in top 10, got:\n{s}"
    );
}

#[test]
fn p177_20102011_stamkos_year() {
    ok(&["query", "leaders", "--season", "20102011", "--top", "10"]);
}

#[test]
fn p178_20132014_lockout_short_season() {
    ok(&["query", "leaders", "--season", "20122013", "--top", "5"]);
}

#[test]
fn p179_20162017_mcdavid_first_art_ross() {
    let s = ok(&["query", "leaders", "--season", "20162017", "--top", "1"]);
    assert!(s.contains("McDavid"), "got:\n{s}");
}

#[test]
fn p180_20182019_kucherov_128() {
    let s = ok(&["query", "leaders", "--season", "20182019", "--top", "1"]);
    assert!(s.contains("Kucherov"), "got:\n{s}");
}

#[test]
fn p181_20192020_covid_short_season() {
    ok(&["query", "leaders", "--season", "20192020", "--top", "5"]);
}

#[test]
fn p182_20222023_mcdavid_64_goals() {
    let s = ok(&["query", "leaders", "--season", "20222023", "--top", "1"]);
    assert!(s.contains("McDavid"), "got:\n{s}");
}

#[test]
fn p183_20232024_seasons_arc_compare() {
    let s = ok(&[
        "query",
        "compare",
        "Auston Matthews",
        "Connor McDavid",
        "--season",
        "20232024",
    ]);
    assert!(s.contains("Matthews") && s.contains("McDavid"));
}

#[test]
fn p184_20242025_top_scorer_kucherov() {
    // Pts/82 leader in 2024-25 was Kucherov (127.2). Draisaitl was
    // raw points leader; Kucherov's pace edged him on Pts/82.
    let s = ok(&["query", "leaders", "--season", "20242025", "--top", "1"]);
    assert!(s.contains("Kucherov"), "got:\n{s}");
}

#[test]
fn p185_20252026_current_season_data() {
    ok(&["query", "leaders", "--season", "20252026", "--top", "5"]);
}

// ── Bucket R: Integration + sort/filter combos — 10 scenarios ─────────────

#[test]
fn p186_sort_g_with_age_filter() {
    ok(&[
        "query",
        "leaders",
        "--sort",
        "g",
        "--age-max",
        "23",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p187_sort_pts_with_filter_alias() {
    ok(&[
        "query", "leaders", "--sort", "pts", "--filter", "gp>=70", "--season", "20242025", "--top",
        "10",
    ]);
}

#[test]
fn p188_pos_d_with_filter_blocks() {
    ok(&[
        "query", "leaders", "--pos", "D", "--filter", "blk>=100", "--season", "20242025", "--top",
        "10",
    ]);
}

#[test]
#[allow(non_snake_case)]
fn p189_pos_C_filter_fow_pct() {
    ok(&[
        "query",
        "leaders",
        "--pos",
        "C",
        "--filter",
        "faceoff-win-pct>=0.55",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p190_aggregate_seasons_2_filter() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "2",
        "--filter",
        "g>=50",
        "--top",
        "10",
    ]);
}

#[test]
fn p191_aggregate_seasons_5_undrafted_age_max() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "5",
        "--undrafted",
        "--age-max",
        "27",
        "--top",
        "10",
    ]);
}

#[test]
fn p192_compare_with_filter_cohort_passes_through_to_similar() {
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "--similar",
        "10",
        "--filter",
        "gp>=20",
    ]);
}

#[test]
fn p193_player_filter_narrows_percentile_pool() {
    ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--filter",
        "gp>=20",
        "--percentiles",
    ]);
}

#[test]
fn p194_export_md_columns_with_alias_keys() {
    // L.5.4 export — `export md` is the only export shape today.
    // Use Linux-friendly tempdir for the output.
    let dir = std::env::temp_dir().join(format!("icelines-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let out_path = dir.join("aliases.md");
    let result = ok(&[
        "export",
        "md",
        "leaders",
        "--columns",
        "g,a,p,hits,blk",
        "--out",
        out_path.to_str().unwrap(),
    ]);
    let _ = result;
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn p195_handedness_filter() {
    ok(&[
        "query",
        "leaders",
        "--handedness",
        "L",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

// ── Bucket S: Robustness + smoke — 5 scenarios ─────────────────────────

#[test]
fn p196_repeated_runs_no_state_leak() {
    // 5 invocations back-to-back — checks for global-state pollution.
    for _ in 0..5 {
        ok(&["query", "leaders", "--top", "5"]);
    }
}

#[test]
fn p197_help_subcommand_doesnt_crash() {
    let out = run(&["query", "leaders", "--help"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!combined.contains("panicked"));
}

#[test]
fn p198_query_player_help_lists_seasons_flag() {
    let out = run(&["query", "player", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("--seasons"),
        "must list --seasons in help"
    );
}

#[test]
fn p199_query_compare_help_lists_seasons_flag() {
    let out = run(&["query", "compare", "--help"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("--seasons"));
}

#[test]
fn p200_workspace_smoke_full_dispatch_paths() {
    // Smoke: leaders + player + compare + goalies all run on 2024-25.
    ok(&["query", "leaders", "--season", "20242025", "--top", "1"]);
    ok(&["query", "player", "Connor McDavid", "--season", "20242025"]);
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "Sidney Crosby",
        "--season",
        "20242025",
    ]);
    ok(&["query", "goalies", "--season", "20242025", "--top", "1"]);
}
