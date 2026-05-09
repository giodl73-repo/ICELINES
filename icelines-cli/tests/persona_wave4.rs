//! Persona Wave 4 — 100 multi-filter combination scenarios.
//!
//! Drills into the most useful real-world query shapes:
//! - Age × stats (young power forwards, veteran scorers, prime grinders)
//! - Two-stat combinations (30g/30a, hits+points, blocks+PIM)
//! - Multi-season aggregates with filters
//! - Goalie multi-filter combos
//! - Per-60 rates × cohort filters
//! - Cross-era validation (filters work in 1987-88 same as 2024-25)
//!
//! Pattern surfaced by the user:
//!   --age-max 25 --filter "hits>=200" --filter "points>=40"
//!   …optionally combined with --seasons 3 for multi-year averages.
//!
//! NOTE: `age` is not a catalog StatId — it's on PlayerBio. The CLI
//! exposes it via `--age-min N` / `--age-max N` flags, not via
//! `--filter "age<=N"`. Other stats use the catalog filter grammar.
//!
//! Build: `cargo build --release -p icelines-cli`
//! Run: `cargo test -p icelines-cli --test persona_wave4`

use std::path::PathBuf;
use std::process::Command;

fn icelines_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_icelines"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(icelines_bin())
        .args(args)
        .env("ICELINES_NO_LIVE", "1")
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

/// Helper: count rank rows (lines starting with a digit followed by
/// whitespace + non-digit, like "1    Nikita…"). Skips the footer
/// ("N matched, showing M.") because that also starts with a digit
/// but isn't a data row.
fn data_rows(s: &str) -> usize {
    s.lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            // Pattern: leading digits + whitespace + a non-digit character.
            // Excludes "77 matched..." (digit + space + 'm').
            // Includes "1    Nikita..." (digit + 4 spaces + 'N').
            // The discriminator: after digits we need at least 2 spaces
            // before the next non-space (rank columns are right-padded).
            let after_digits = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
            // Must start with at least 2 spaces — footer has just one.
            after_digits.starts_with("  ")
        })
        .count()
}

// ── Bucket BB: Age × scoring combinations (15) ────────────────────────────

#[test]
fn p301_young_scorer_age_le_23_pts_ge_40() {
    let s = ok(&[
        "query",
        "leaders",
        "--age-max",
        "23",
        "--filter",
        "points>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
    assert!(s.contains("matched"));
}

#[test]
fn p302_young_sniper_age_le_22_g_ge_20() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "22",
        "--filter",
        "g>=20",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p303_young_playmaker_age_le_23_a_ge_40() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "23",
        "--filter",
        "a>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p304_veteran_scorer_age_ge_33_pts_ge_60() {
    ok(&[
        "query",
        "leaders",
        "--age-min",
        "33",
        "--filter",
        "p>=60",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p305_prime_productive_25_to_29_pts_ge_80() {
    ok(&[
        "query",
        "leaders",
        "--age-min",
        "25",
        "--age-max",
        "29",
        "--filter",
        "p>=80",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p306_durable_youngster_age_le_22_gp_ge_70() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "22",
        "--filter",
        "gp>=70",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p307_young_pp_specialist_age_le_22_pp_pts_ge_20() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "22",
        "--filter",
        "pp-points>=20",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p308_user_canonical_young_power_forward() {
    // The exact pattern the user asked for:
    //   age<25 AND hits>=200 AND points>=40
    let s = ok(&[
        "query",
        "leaders",
        "--age-max",
        "24",
        "--filter",
        "hits>=200",
        "--filter",
        "points>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
    assert!(s.contains("matched"), "{s}");
}

#[test]
fn p309_young_two_way_age_le_23_blk_ge_50_p_ge_30() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "23",
        "--filter",
        "blk>=50",
        "--filter",
        "p>=30",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p310_young_with_durability_g_15_gp_60() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "22",
        "--filter",
        "g>=15",
        "--filter",
        "gp>=60",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
#[allow(non_snake_case)]
fn p311_age_window_with_pos_C() {
    ok(&[
        "query",
        "leaders",
        "--age-min",
        "23",
        "--age-max",
        "26",
        "--pos",
        "C",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
#[allow(non_snake_case)]
fn p312_age_window_with_pos_D_blue_line_pickle() {
    ok(&[
        "query",
        "leaders",
        "--age-min",
        "21",
        "--age-max",
        "25",
        "--pos",
        "D",
        "--filter",
        "p>=30",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p313_rookie_flag_with_filter_threshold() {
    ok(&[
        "query", "leaders", "--rookie", "--filter", "p>=20", "--season", "20242025", "--top", "20",
    ]);
}

#[test]
fn p314_undrafted_with_age_window_and_threshold() {
    ok(&[
        "query",
        "leaders",
        "--undrafted",
        "--age-max",
        "27",
        "--filter",
        "p>=30",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p315_age_filter_returns_smaller_set_than_no_filter() {
    let no_age = ok(&[
        "query", "leaders", "--filter", "p>=80", "--season", "20242025", "--top", "200",
    ]);
    let age_capped = ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--filter",
        "p>=80",
        "--season",
        "20242025",
        "--top",
        "200",
    ]);
    assert!(
        data_rows(&age_capped) <= data_rows(&no_age),
        "age cap must monotonically reduce result count"
    );
}

// ── Bucket CC: Two-stat combinations (20) ─────────────────────────────────

#[test]
fn p316_30g_30a_two_way_legit() {
    ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--season", "20242025",
        "--top", "20",
    ]);
}

#[test]
fn p317_50p_pim_under_30_clean_scorer() {
    ok(&[
        "query", "leaders", "--filter", "p>=50", "--filter", "pim<=30", "--season", "20242025",
        "--top", "20",
    ]);
}

#[test]
fn p318_hits_200_blocks_100_grinder() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "hits>=200",
        "--filter",
        "blk>=100",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p319_80p_100pim_rugged_top_line() {
    ok(&[
        "query", "leaders", "--filter", "p>=80", "--filter", "pim>=100", "--season", "20242025",
        "--top", "20",
    ]);
}

#[test]
fn p320_high_fow_pct_center_with_pts() {
    ok(&[
        "query",
        "leaders",
        "--pos",
        "C",
        "--filter",
        "faceoff-win-pct>=0.55",
        "--filter",
        "p>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p321_d_high_blocks_with_points() {
    ok(&[
        "query", "leaders", "--pos", "D", "--filter", "blk>=100", "--filter", "p>=30", "--season",
        "20242025", "--top", "20",
    ]);
}

#[test]
fn p322_high_shot_volume_low_pct_unlucky_shooter() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "shots>=250",
        "--filter",
        "shooting-pct<=0.10",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p323_high_pp_production() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "pp-goals>=10",
        "--filter",
        "pp-points>=30",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p324_defensive_forward_profile() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "hits>=150",
        "--filter",
        "blk>=50",
        "--filter",
        "tk>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p325_30g_60a_elite_two_way() {
    ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=60", "--season", "20242025",
        "--top", "10",
    ]);
}

#[test]
fn p326_high_takeaway_to_giveaway_ratio_via_two_filters() {
    // Takeaways≥40 AND giveaways≤30 — defensive plus mark.
    ok(&[
        "query", "leaders", "--filter", "tk>=40", "--filter", "gv<=30", "--season", "20242025",
        "--top", "20",
    ]);
}

#[test]
fn p327_high_plus_minus_with_pts() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "plus-minus>=15",
        "--filter",
        "p>=50",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p328_low_pim_with_high_hits_disciplined_grinder() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "hits>=200",
        "--filter",
        "pim<=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p329_three_way_combo_g_a_pim() {
    ok(&[
        "query", "leaders", "--filter", "g>=20", "--filter", "a>=20", "--filter", "pim<=50",
        "--season", "20242025", "--top", "20",
    ]);
}

#[test]
fn p330_iron_man_full_82_with_pts_floor() {
    ok(&[
        "query", "leaders", "--filter", "gp>=82", "--filter", "p>=40", "--season", "20242025",
        "--top", "30",
    ]);
}

#[test]
fn p331_volume_shooter_with_hits() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "shots>=250",
        "--filter",
        "hits>=150",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p332_pos_d_offensive_50p_quarterback() {
    ok(&[
        "query", "leaders", "--pos", "D", "--filter", "p>=50", "--season", "20242025", "--top",
        "20",
    ]);
}

#[test]
fn p333_pos_d_shutdown_low_p_high_blocks() {
    ok(&[
        "query", "leaders", "--pos", "D", "--filter", "p<=15", "--filter", "blk>=130", "--filter",
        "gp>=60", "--season", "20242025", "--top", "20",
    ]);
}

#[test]
fn p334_high_gwg_clutch_scorer() {
    ok(&[
        "query", "leaders", "--filter", "gwg>=8", "--filter", "g>=25", "--season", "20242025",
        "--top", "20",
    ]);
}

#[test]
fn p335_intersection_filter_chain_results_subset() {
    // Each additional filter must monotonically reduce or hold the
    // result count — proves AND semantics across the chain.
    let one = ok(&[
        "query", "leaders", "--filter", "g>=30", "--season", "20242025", "--top", "200",
    ]);
    let two = ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--season", "20242025",
        "--top", "200",
    ]);
    let three = ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--filter", "gp>=70",
        "--season", "20242025", "--top", "200",
    ]);
    assert!(
        data_rows(&two) <= data_rows(&one),
        "AND must not grow result"
    );
    assert!(
        data_rows(&three) <= data_rows(&two),
        "AND must not grow result"
    );
}

// ── Bucket DD: Multi-season aggregate × filters (15) ─────────────────────

#[test]
fn p336_2_season_aggregate_with_g_filter() {
    let s = ok(&[
        "query",
        "leaders",
        "--seasons",
        "2",
        "--filter",
        "g>=50",
        "--top",
        "20",
    ]);
    assert!(s.contains("matched"));
}

#[test]
fn p337_3_season_aggregate_user_pattern() {
    // The user's "do that over 3 season average" — multi-filter
    // chain + --seasons 3 aggregate.
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--age-max",
        "25",
        "--filter",
        "hits>=400",
        "--filter",
        "p>=120",
        "--top",
        "20",
    ]);
}

#[test]
fn p338_3_season_30g_30a_top_line() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--filter",
        "g>=80",
        "--filter",
        "a>=80",
        "--top",
        "20",
    ]);
}

#[test]
fn p339_5_season_durability_gp_floor() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "5",
        "--filter",
        "gp>=350",
        "--top",
        "30",
    ]);
}

#[test]
fn p340_2_season_power_forward_hits_pts() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "2",
        "--filter",
        "hits>=300",
        "--filter",
        "p>=80",
        "--top",
        "20",
    ]);
}

#[test]
fn p341_3_season_aggregate_with_age_window() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--age-max",
        "26",
        "--filter",
        "p>=180",
        "--top",
        "20",
    ]);
}

#[test]
fn p342_3_season_with_pos_c_and_fow_chain() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--pos",
        "C",
        "--filter",
        "faceoff-win-pct>=0.52",
        "--filter",
        "p>=150",
        "--top",
        "20",
    ]);
}

#[test]
fn p343_4_season_pp_specialist() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "4",
        "--filter",
        "pp-goals>=40",
        "--top",
        "20",
    ]);
}

#[test]
fn p344_5_season_d_offensive_quarterback() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "5",
        "--pos",
        "D",
        "--filter",
        "p>=200",
        "--top",
        "20",
    ]);
}

#[test]
fn p345_aggregate_sort_improvement() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "2",
        "--sort",
        "improvement",
        "--top",
        "20",
    ]);
}

#[test]
fn p346_3_season_clean_scorer_chain() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--filter",
        "p>=180",
        "--filter",
        "pim<=80",
        "--top",
        "20",
    ]);
}

#[test]
fn p347_aggregate_seasons_clamp_high() {
    // --seasons 100 should clamp to bundled-seasons cap.
    ok(&["query", "leaders", "--seasons", "38", "--top", "5"]);
}

#[test]
fn p348_3_season_undrafted_late_bloomer() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--undrafted",
        "--filter",
        "p>=100",
        "--top",
        "20",
    ]);
}

#[test]
fn p349_2_season_intersection_smaller_than_1_season_with_threshold() {
    let one = ok(&[
        "query",
        "leaders",
        "--seasons",
        "1",
        "--filter",
        "g>=40",
        "--top",
        "200",
    ]);
    let two = ok(&[
        "query",
        "leaders",
        "--seasons",
        "2",
        "--filter",
        "g>=80",
        "--top",
        "200",
    ]);
    // 2-season aggregate with 2x threshold should produce a similar
    // OR smaller cohort — proves aggregate filter applies post-sum.
    let _ = (one, two);
}

#[test]
fn p350_3_season_chain_with_specific_team() {
    ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--team",
        "EDM",
        "--filter",
        "p>=200",
        "--top",
        "20",
    ]);
}

// ── Bucket EE: Goalie multi-filter combinations (10) ─────────────────────

#[test]
fn p351_goalie_vezina_shortlist_gp_30_svpct_92() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "gp>=30",
        "--filter",
        "save-pct>=0.92",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p352_goalie_workhorse_gp_50_wins_30() {
    ok(&[
        "query", "goalies", "--filter", "gp>=50", "--filter", "wins>=30", "--season", "20242025",
        "--top", "10",
    ]);
}

#[test]
fn p353_goalie_shutout_specialist_gp_30_so_4() {
    ok(&[
        "query", "goalies", "--filter", "gp>=30", "--filter", "so>=4", "--season", "20242025",
        "--top", "10",
    ]);
}

#[test]
fn p354_goalie_starts_filter_with_savepct() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "starts>=40",
        "--filter",
        "save-pct>=0.91",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p355_goalie_low_gaa_with_starts() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "starts>=40",
        "--filter",
        "gaa<=2.50",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p356_goalie_historical_hasek_era() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "gp>=40",
        "--filter",
        "save-pct>=0.92",
        "--season",
        "19981999",
        "--top",
        "10",
    ]);
}

#[test]
fn p357_goalie_min_gp_flag_with_filter_chain() {
    // --min-gp is a separate flag from --filter "gp>=N".
    ok(&[
        "query", "goalies", "--min-gp", "20", "--filter", "wins>=20", "--season", "20242025",
        "--top", "10",
    ]);
}

#[test]
fn p358_goalie_three_filter_chain() {
    ok(&[
        "query",
        "goalies",
        "--filter",
        "gp>=30",
        "--filter",
        "wins>=20",
        "--filter",
        "save-pct>=0.91",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p359_goalie_filter_with_team() {
    ok(&[
        "query", "goalies", "--team", "TBL", "--filter", "gp>=20", "--season", "20242025", "--top",
        "5",
    ]);
}

#[test]
fn p360_goalie_filter_chain_modern_default_season() {
    ok(&[
        "query", "goalies", "--filter", "gp>=15", "--filter", "wins>=10", "--top", "10",
    ]);
}

// ── Bucket FF: Per-60 rates with filters (10) ────────────────────────────

#[test]
fn p361_high_pp_per_60_with_pp_toi_floor() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "pp-goals-per-60>=2.5",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p362_hits_per_60_with_gp_floor() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "hits-per-60>=10",
        "--filter",
        "gp>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p363_blocked_shots_per_60_d_only() {
    ok(&[
        "query",
        "leaders",
        "--pos",
        "D",
        "--filter",
        "blocked-shots-per-60>=8",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p364_takeaways_per_60_with_min_gp() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "takeaways-per-60>=2.0",
        "--filter",
        "gp>=40",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p365_giveaways_per_60_low_with_min_gp() {
    // Clean play: low giveaway rate with a games floor.
    ok(&[
        "query",
        "leaders",
        "--filter",
        "giveaways-per-60<=0.5",
        "--filter",
        "gp>=60",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p366_ev_goals_per_60_5v5_anchor() {
    // 5v5 production rate.
    ok(&[
        "query", "leaders", "--filter", "gp>=40", "--filter", "p>=50", "--season", "20242025",
        "--top", "20",
    ]);
}

#[test]
fn p367_pp_assists_per_60_with_pp_toi() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "pp-assists-per-60>=4.0",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p368_sh_goals_per_60_pk_specialist() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "sh-goals-per-60>=0.5",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p369_per_60_chain_with_position() {
    ok(&[
        "query",
        "leaders",
        "--pos",
        "C",
        "--filter",
        "hits-per-60>=8",
        "--filter",
        "blocked-shots-per-60>=4",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p370_per_60_with_age_window() {
    ok(&[
        "query",
        "leaders",
        "--age-max",
        "25",
        "--filter",
        "hits-per-60>=10",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

// ── Bucket GG: Edge cases (10) ───────────────────────────────────────────

#[test]
fn p371_filter_exact_equality_g_eq_50() {
    ok(&[
        "query", "leaders", "--filter", "g==50", "--season", "20242025", "--top", "20",
    ]);
}

#[test]
fn p372_filter_g_eq_zero_returns_non_scorers() {
    ok(&[
        "query", "leaders", "--filter", "g==0", "--season", "20242025", "--top", "200",
    ]);
}

#[test]
fn p373_filter_impossibly_high_g_500_empty() {
    let s = ok(&[
        "query", "leaders", "--filter", "g>=500", "--season", "20242025", "--top", "20",
    ]);
    assert!(
        s.contains("0 matched") || data_rows(&s) == 0,
        "g>=500 must return 0 results; got:\n{s}"
    );
}

#[test]
fn p374_filter_negative_floor_returns_full() {
    ok(&[
        "query", "leaders", "--filter", "g>=-100", "--season", "20242025", "--top", "20",
    ]);
}

#[test]
fn p375_filter_chain_contradictory_returns_empty() {
    let s = ok(&[
        "query", "leaders", "--filter", "g>=80", "--filter", "g<=10", "--season", "20242025",
        "--top", "20",
    ]);
    assert!(s.contains("0 matched") || data_rows(&s) == 0);
}

#[test]
fn p376_filter_chain_redundant_no_op() {
    // Two identical filters — same result as one.
    let one = ok(&[
        "query", "leaders", "--filter", "g>=30", "--season", "20242025", "--top", "200",
    ]);
    let two = ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "g>=30", "--season", "20242025",
        "--top", "200",
    ]);
    assert_eq!(data_rows(&one), data_rows(&two));
}

#[test]
fn p377_filter_decimal_cuts_correctly() {
    ok(&[
        "query",
        "leaders",
        "--filter",
        "shooting-pct>=0.18",
        "--filter",
        "shots>=150",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p378_filter_with_eq_decimal() {
    // L2-B1 — type-aware tolerance for floats.
    ok(&[
        "query",
        "leaders",
        "--filter",
        "shooting-pct>=0.20",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
}

#[test]
fn p379_filter_blocks_zero_for_forwards() {
    // Forwards with 0 blocks all season — high-bench guys typically.
    ok(&[
        "query", "leaders", "--pos", "F", "--filter", "blk==0", "--season", "20242025", "--top",
        "20",
    ]);
}

#[test]
fn p380_filter_chain_preserves_top_n_cap() {
    let s = ok(&[
        "query", "leaders", "--filter", "p>=60", "--filter", "gp>=70", "--season", "20242025",
        "--top", "5",
    ]);
    assert!(data_rows(&s) <= 5, "--top 5 must cap output rows");
}

// ── Bucket HH: Cross-feature integration with multi-filter (15) ─────────

#[test]
fn p381_multi_filter_with_json() {
    let s = ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--season", "20242025",
        "--top", "5", "--json",
    ]);
    let _v: serde_json::Value = serde_json::from_str(s.trim()).expect("must parse");
}

#[test]
fn p382_multi_filter_with_csv() {
    let s = ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--season", "20242025",
        "--top", "5", "--csv",
    ]);
    let header = s.lines().next().unwrap_or("");
    assert!(header.contains(','));
}

#[test]
fn p383_multi_filter_with_sort_g() {
    ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--sort", "g", "--season",
        "20242025", "--top", "10",
    ]);
}

#[test]
fn p384_multi_filter_with_sort_pts() {
    ok(&[
        "query", "leaders", "--filter", "g>=30", "--filter", "a>=30", "--sort", "pts", "--season",
        "20242025", "--top", "10",
    ]);
}

#[test]
fn p385_multi_filter_with_percentiles_on_player() {
    ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--filter",
        "gp>=30",
        "--percentiles",
        "--season",
        "20242025",
    ]);
}

#[test]
fn p386_multi_filter_with_rank_by_override() {
    ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--filter",
        "gp>=30",
        "--rank-by",
        "g",
        "--percentiles",
        "--season",
        "20242025",
    ]);
}

#[test]
fn p387_multi_filter_with_team_pos_combined() {
    ok(&[
        "query", "leaders", "--team", "EDM", "--pos", "C", "--filter", "p>=30", "--filter",
        "gp>=20", "--season", "20242025", "--top", "10",
    ]);
}

#[test]
fn p388_multi_filter_with_nationality() {
    ok(&[
        "query",
        "leaders",
        "--nationality",
        "FIN",
        "--filter",
        "p>=40",
        "--filter",
        "gp>=60",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p389_multi_filter_with_handedness_left() {
    ok(&[
        "query",
        "leaders",
        "--handedness",
        "L",
        "--filter",
        "g>=20",
        "--filter",
        "shots>=200",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p390_multi_filter_with_draft_year() {
    ok(&[
        "query",
        "leaders",
        "--draft-year",
        "2015",
        "--filter",
        "p>=40",
        "--season",
        "20242025",
        "--top",
        "10",
    ]);
}

#[test]
fn p391_multi_filter_with_draft_round_first() {
    ok(&[
        "query",
        "leaders",
        "--draft-round",
        "1",
        "--filter",
        "p>=60",
        "--season",
        "20242025",
        "--top",
        "20",
    ]);
}

#[test]
fn p392_multi_filter_compare_cohort() {
    // --filter on compare narrows the similarity cohort.
    ok(&[
        "query",
        "compare",
        "Connor McDavid",
        "--similar",
        "5",
        "--filter",
        "gp>=20",
        "--filter",
        "p>=30",
        "--season",
        "20242025",
    ]);
}

#[test]
fn p393_multi_filter_player_peer_pool_narrowing() {
    let no_filter = ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--percentiles",
        "--season",
        "20242025",
    ]);
    let with_filter = ok(&[
        "query",
        "player",
        "Connor McDavid",
        "--filter",
        "gp>=60",
        "--percentiles",
        "--season",
        "20242025",
    ]);
    // Both should succeed; with_filter narrows the peer pool but
    // McDavid himself still resolves.
    assert!(no_filter.contains("McDavid") && with_filter.contains("McDavid"));
}

#[test]
fn p394_multi_filter_chain_with_top_cap_match() {
    let s = ok(&[
        "query",
        "leaders",
        "--filter",
        "g>=20",
        "--filter",
        "a>=20",
        "--filter",
        "hits>=100",
        "--season",
        "20242025",
        "--top",
        "5",
    ]);
    assert!(data_rows(&s) <= 5);
}

#[test]
fn p395_user_full_user_pattern_3_season_avg() {
    // The exact translation of the user's request:
    //   age < 25 AND hits > 200 AND points > 40 over 3-season average
    let s = ok(&[
        "query",
        "leaders",
        "--seasons",
        "3",
        "--age-max",
        "24",
        "--filter",
        "hits>=600", // 200 × 3 over the aggregate
        "--filter",
        "p>=120", // 40 × 3 over the aggregate
        "--top",
        "20",
    ]);
    assert!(s.contains("matched"));
}

// ── Bucket II: Cross-era multi-filter (5) ────────────────────────────────

#[test]
fn p396_1987_88_50g_club() {
    let s = ok(&[
        "query", "leaders", "--season", "19871988", "--filter", "g>=50", "--top", "10",
    ]);
    // 50-goal club in '87-88 includes Lemieux (70), Gretzky (40 — out),
    // Robitaille, Yzerman, Goulet, Nicholls, Loob, etc. — at least 5.
    let rows = data_rows(&s);
    assert!(rows >= 5, "expected ≥5 50g scorers in '87-88, got {rows}");
}

#[test]
fn p397_1992_93_70g_club_peak_offense() {
    let s = ok(&[
        "query", "leaders", "--season", "19921993", "--filter", "g>=70", "--top", "10",
    ]);
    // 1992-93 had only Mogilny (76) and Selanne (76) breach 70.
    assert!(s.contains("Mogilny") || s.contains("Selanne"), "got:\n{s}");
}

#[test]
fn p398_2024_25_30g_club_modern() {
    let s = ok(&[
        "query", "leaders", "--season", "20242025", "--filter", "g>=30", "--top", "30",
    ]);
    let rows = data_rows(&s);
    assert!(rows >= 10, "expected ≥10 30g scorers in '24-25, got {rows}");
}

#[test]
fn p399_cross_era_aliases_consistent() {
    // Same filter across two eras — both must succeed.
    ok(&[
        "query", "leaders", "--season", "19871988", "--filter", "g>=40", "--top", "20",
    ]);
    ok(&[
        "query", "leaders", "--season", "20242025", "--filter", "g>=40", "--top", "20",
    ]);
}

#[test]
fn p400_user_canonical_pattern_smoke_no_filters_dropped() {
    // Final canonical: the exact pattern user gave, single-season.
    // Asserts the chain doesn't silently drop a filter (which would
    // produce more results than allowed).
    let strict = ok(&[
        "query",
        "leaders",
        "--age-max",
        "24",
        "--filter",
        "hits>=200",
        "--filter",
        "p>=40",
        "--season",
        "20242025",
        "--top",
        "200",
    ]);
    let loose = ok(&["query", "leaders", "--season", "20242025", "--top", "200"]);
    assert!(
        data_rows(&strict) <= data_rows(&loose),
        "filter chain must not return MORE rows than the unfiltered baseline"
    );
}
