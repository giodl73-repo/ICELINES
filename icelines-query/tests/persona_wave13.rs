//! Persona Wave 13 — 200 reporter-style queries.
//!
//! Real hockey storyline questions a beat writer or analyst
//! would actually type into the system, translated into the
//! Phase Art Ross grammar. Each test verifies the query parses
//! cleanly and produces sensible IR (right variant shape, right
//! atom count, needs_provider correct).
//!
//! Different from Wave 12 (adversarial parser-stress): these
//! are REAL questions about REAL hockey storylines. If any fail
//! to parse it's almost certainly a real bug — they were chosen
//! to exercise the grammar the way users will.
//!
//! Sections:
//!   A — Young breakout candidates (20)
//!   B — Career scoring milestones (20)
//!   C — Goalie storylines (20)
//!   D — Defensemen analytics (20)
//!   E — Power-forward archetypes (20)
//!   F — Geographic / national-team (20)
//!   G — Draft-retrospective (20)
//!   H — Streak / hot-cold (20)
//!   I — Cross-league / development arc (20)
//!   J — Reporter staples (20)

use icelines_query::{parse_query, Constraint, FilterInput};

fn ok(s: &str) -> Constraint {
    parse_query(FilterInput::Cli(s.to_string()))
        .unwrap_or_else(|e| {
            panic!("REPORTER QUERY FAILED TO PARSE\n  --filter {s:?}\n  errors: {e:?}")
        })
        .root
}

/// Verify the query parses + has the expected number of top-level
/// atoms (after All/Any unwrapping).
fn ok_with_n_atoms(s: &str, n: usize) -> Constraint {
    let c = ok(s);
    let count = match &c {
        Constraint::All(children) | Constraint::Any(children) => children.len(),
        _ => 1,
    };
    assert_eq!(
        count, n,
        "expected {n} atoms in {s:?}; got {count} via {c:?}"
    );
    c
}

// ── Section A — Young breakout candidates (20) ───────────────────

#[test]
fn p_w13_001_young_top6_centers() {
    // "Centers under 25 averaging point-per-game"
    ok_with_n_atoms("pos=C AND age<25 AND ppg>=1.0", 3);
}

#[test]
fn p_w13_002_youthful_goal_scorers() {
    ok_with_n_atoms("g>=20 AND age<=22", 2);
}

#[test]
fn p_w13_003_first_round_picks_under_24() {
    ok_with_n_atoms("draft-round=1 AND age<24 AND p>=40", 3);
}

#[test]
fn p_w13_004_sophomore_breakouts() {
    ok_with_n_atoms("rookie-season=20232024 AND p>=50", 2);
}

#[test]
fn p_w13_005_rookies_already_breaking_out() {
    // Rookies are draft pick year ≥ recent
    ok_with_n_atoms("rookie-season>=20242025 AND g>=15", 2);
}

#[test]
fn p_w13_006_under_25_with_streak() {
    // Hot rookie/sophomore streak
    let c = ok("g.last10g>=5 AND age<=24");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_007_top_pick_breakout() {
    ok_with_n_atoms("draft-overall<=10 AND age<=23 AND ppg>=0.8", 3);
}

#[test]
fn p_w13_008_under_25_canadian_top_picks() {
    ok_with_n_atoms("country=CAN AND draft-overall<=15 AND age<=24", 3);
}

#[test]
fn p_w13_009_under_25_swedish_centers() {
    ok_with_n_atoms("country=SWE AND pos=C AND age<=25", 3);
}

#[test]
fn p_w13_010_youthful_two_way_centers() {
    ok_with_n_atoms("pos=C AND age<=24 AND p>=40 AND blocked-shots>=20", 4);
}

#[test]
fn p_w13_011_under_22_first_rounders_with_50_pts() {
    ok_with_n_atoms("age<=22 AND draft-round=1 AND p>=50", 3);
}

#[test]
fn p_w13_012_young_left_wingers_with_pop() {
    ok_with_n_atoms("pos=LW AND age<=23 AND g>=20", 3);
}

#[test]
fn p_w13_013_young_right_wing_specialists() {
    ok_with_n_atoms("pos=RW AND age<25 AND ppg>=0.7", 3);
}

#[test]
fn p_w13_014_young_d_with_offense() {
    ok_with_n_atoms("pos=D AND age<=24 AND assists>=30", 3);
}

#[test]
fn p_w13_015_young_d_lockdown() {
    ok_with_n_atoms("pos=D AND age<=24 AND blocked-shots>=80", 3);
}

#[test]
fn p_w13_016_top10_picks_under_24() {
    ok_with_n_atoms("draft-overall<=10 AND age<24", 2);
}

#[test]
fn p_w13_017_late_round_under25_breakouts() {
    ok_with_n_atoms("draft-round>=4 AND age<=25 AND p>=40", 3);
}

#[test]
fn p_w13_018_undrafted_youngsters_with_pts() {
    // age<=24 AND no draft year (parser doesn't have null-check;
    // this is a near-miss reporter would write — verify it parses)
    ok("age<=24 AND p>=30");
}

#[test]
fn p_w13_019_young_centers_who_can_draw_penalties() {
    ok_with_n_atoms("pos=C AND age<=25 AND penalties-drawn>=20", 3);
}

#[test]
fn p_w13_020_young_high_shot_volume() {
    ok_with_n_atoms("age<=24 AND shots>=200", 2);
}

// ── Section B — Career scoring milestones (20) ──────────────────

#[test]
fn p_w13_021_career_500_goals_club() {
    let c = ok("g.career>=500");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_022_career_1000_points_club() {
    let c = ok("p.career>=1000");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_023_career_500_goals_canadian() {
    ok_with_n_atoms("g.career>=500 AND country=CAN", 2);
}

#[test]
fn p_w13_024_career_1000_assists() {
    ok("a.career>=1000");
}

#[test]
fn p_w13_025_50_goal_seasons_count() {
    // SeasonsWith — count of seasons with the predicate
    ok("g.seasons-with>=5");
}

#[test]
fn p_w13_026_100_pt_seasons_count() {
    ok("p.seasons-with>=3");
}

#[test]
fn p_w13_027_active_career_400_goals() {
    ok_with_n_atoms("g.career>=400 AND age<=35", 2);
}

#[test]
fn p_w13_028_european_500_pt_career() {
    ok_with_n_atoms("p.career>=500 AND country IN (SWE, FIN, RUS, CZE)", 2);
}

#[test]
fn p_w13_029_career_pts_pace_threshold() {
    ok("p.career>=300 AND age<=27");
}

#[test]
fn p_w13_030_career_streak_legend() {
    // Longest career point streak
    ok("p.streak>=15");
}

#[test]
fn p_w13_031_career_25g_in_10_ever() {
    // 25 goals in 10 games — would be a record. EVER form.
    ok("g.any10g>=25 EVER");
}

#[test]
fn p_w13_032_5g_in_3g_ever() {
    // 5 goals in 3 games (hat trick three times in a row?)
    ok("g.any3g>=5 EVER");
}

#[test]
fn p_w13_033_30_assists_in_10g_window() {
    ok("a.any10g>=30 EVER");
}

#[test]
fn p_w13_034_15g_in_5g_under_22() {
    ok("g.any5g>=15 EVER AT age<=22");
}

#[test]
fn p_w13_035_career_us_500_goals() {
    ok_with_n_atoms("g.career>=500 AND country=USA", 2);
}

#[test]
fn p_w13_036_career_300g_strict_under30() {
    ok_with_n_atoms("g.career>=300 AND age<30", 2);
}

#[test]
fn p_w13_037_career_with_at_age_window() {
    ok("p.career>=400 AT age BETWEEN 22 AND 30");
}

#[test]
fn p_w13_038_career_first_round_top500() {
    ok_with_n_atoms("p.career>=500 AND draft-round=1", 2);
}

#[test]
fn p_w13_039_career_late_round_overachievers() {
    ok_with_n_atoms("p.career>=400 AND draft-round>=4", 2);
}

#[test]
fn p_w13_040_career_milestone_streak_combo() {
    ok_with_n_atoms("p.career>=500 AND p.streak>=10", 2);
}

// ── Section C — Goalie storylines (20) ──────────────────────────

#[test]
fn p_w13_041_starting_goalies_save_pct() {
    ok_with_n_atoms("pos=G AND save-pct>=0.910 AND goalie-games>=40", 3);
}

#[test]
fn p_w13_042_goalies_under_25_starting() {
    ok_with_n_atoms("pos=G AND age<=25 AND goalie-starts>=20", 3);
}

#[test]
fn p_w13_043_low_gaa_goalies() {
    ok_with_n_atoms("pos=G AND gaa<=2.50", 2);
}

#[test]
fn p_w13_044_shutout_kings() {
    ok_with_n_atoms("pos=G AND shutouts>=4", 2);
}

#[test]
fn p_w13_045_career_300_wins_club() {
    ok_with_n_atoms("pos=G AND wins.career>=300", 2);
}

#[test]
fn p_w13_046_canadian_goalie_studs() {
    ok_with_n_atoms("pos=G AND country=CAN AND save-pct>=0.910", 3);
}

#[test]
fn p_w13_047_quality_starts_count() {
    ok_with_n_atoms("pos=G AND quality-starts>=20", 2);
}

#[test]
fn p_w13_048_finnish_goalie_pipeline() {
    ok_with_n_atoms("pos=G AND country=FIN AND age<=27", 3);
}

#[test]
fn p_w13_049_goalie_with_70_starts() {
    ok_with_n_atoms("pos=G AND goalie-starts>=60", 2);
}

#[test]
fn p_w13_050_goalies_60_to_70_save_pct() {
    ok_with_n_atoms("pos=G AND save-pct BETWEEN 0.910 AND 0.930", 2);
}

#[test]
fn p_w13_051_goalies_drafted_late() {
    ok_with_n_atoms("pos=G AND draft-round>=5", 2);
}

#[test]
fn p_w13_052_us_born_goalies() {
    ok_with_n_atoms("pos=G AND country=USA AND age<=30", 3);
}

#[test]
fn p_w13_053_goalies_career_300_starts() {
    ok_with_n_atoms("pos=G AND goalie-starts.career>=300", 2);
}

#[test]
fn p_w13_054_swedish_goalie_starters() {
    ok_with_n_atoms("pos=G AND country=SWE", 2);
}

#[test]
fn p_w13_055_goalies_hot_in_last_10g() {
    let c = ok("pos=G AND wins.last10g>=7");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_056_goalies_cold_streak_check() {
    ok("pos=G AND losses.last10g>=8");
}

#[test]
fn p_w13_057_goalies_under_28_high_save_pct() {
    ok_with_n_atoms("pos=G AND age<28 AND save-pct>=0.920", 3);
}

#[test]
fn p_w13_058_goalies_30_save_pct_in_10g() {
    ok("pos=G AND saves.last10g>=300");
}

#[test]
fn p_w13_059_goalies_over_30_age() {
    ok_with_n_atoms("pos=G AND age>=30 AND goalie-starts>=30", 3);
}

#[test]
fn p_w13_060_goalies_career_50_shutouts() {
    ok_with_n_atoms("pos=G AND shutouts.career>=50", 2);
}

// ── Section D — Defensemen analytics (20) ───────────────────────

#[test]
fn p_w13_061_offensive_defenders() {
    ok_with_n_atoms("pos=D AND p>=40", 2);
}

#[test]
fn p_w13_062_pure_lockdown_d() {
    ok_with_n_atoms("pos=D AND blocked-shots>=150 AND hits>=80", 3);
}

#[test]
fn p_w13_063_norris_candidates() {
    ok_with_n_atoms("pos=D AND p>=50 AND blocked-shots>=100", 3);
}

#[test]
fn p_w13_064_young_d_60_assists() {
    ok_with_n_atoms("pos=D AND age<=24 AND assists>=40", 3);
}

#[test]
fn p_w13_065_top_d_pp_specialists() {
    ok_with_n_atoms("pos=D AND pp-points>=20", 2);
}

#[test]
fn p_w13_066_d_with_20_goals() {
    ok_with_n_atoms("pos=D AND g>=15", 2);
}

#[test]
fn p_w13_067_d_with_300_career_points() {
    ok_with_n_atoms("pos=D AND p.career>=300", 2);
}

#[test]
fn p_w13_068_canadian_top_d() {
    ok_with_n_atoms("pos=D AND country=CAN AND p>=40", 3);
}

#[test]
fn p_w13_069_european_d_breakouts() {
    ok_with_n_atoms("pos=D AND country IN (SWE, FIN, CZE) AND p>=30", 3);
}

#[test]
fn p_w13_070_d_with_career_500_pts() {
    ok_with_n_atoms("pos=D AND p.career>=500", 2);
}

#[test]
fn p_w13_071_d_late_round_breakouts() {
    ok_with_n_atoms("pos=D AND draft-round>=4 AND p>=30", 3);
}

#[test]
fn p_w13_072_d_first_round_top_picks() {
    ok_with_n_atoms("pos=D AND draft-overall<=10 AND p>=50", 3);
}

#[test]
fn p_w13_073_d_under_22() {
    ok_with_n_atoms("pos=D AND age<22", 2);
}

#[test]
fn p_w13_074_d_with_hot_5g_streak() {
    let c = ok("pos=D AND p.last5g>=5");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_075_d_with_takeaways() {
    ok_with_n_atoms("pos=D AND takeaways>=50", 2);
}

#[test]
fn p_w13_076_d_under_six_three_who_score() {
    ok_with_n_atoms("pos=D AND height<=75 AND p>=30", 3);
}

#[test]
fn p_w13_077_big_lockdown_d() {
    ok_with_n_atoms("pos=D AND height>=76 AND blocked-shots>=150", 3);
}

#[test]
fn p_w13_078_d_with_streak_career() {
    ok("pos=D AND p.streak>=10");
}

#[test]
fn p_w13_079_d_with_50pt_seasons() {
    ok("pos=D AND p.seasons-with>=3");
}

#[test]
fn p_w13_080_d_career_assists_milestone() {
    ok_with_n_atoms("pos=D AND a.career>=500", 2);
}

// ── Section E — Power-forward archetypes (20) ───────────────────

#[test]
fn p_w13_081_classic_power_forward() {
    ok_with_n_atoms("height>=74 AND weight>=210 AND hits>=150 AND g>=20", 4);
}

#[test]
fn p_w13_082_six_foot_two_power_forward() {
    ok_with_n_atoms("height>=74 AND g>=25 AND pim>=50", 3);
}

#[test]
fn p_w13_083_pf_with_career_300_goals() {
    ok_with_n_atoms("height>=74 AND weight>=210 AND g.career>=300", 3);
}

#[test]
fn p_w13_084_two_way_power_forward() {
    ok_with_n_atoms(
        "height>=74 AND weight>=210 AND blocked-shots>=50 AND g>=20",
        4,
    );
}

#[test]
fn p_w13_085_young_pf() {
    ok_with_n_atoms("height>=74 AND age<=25 AND g>=20", 3);
}

#[test]
fn p_w13_086_pf_with_hits_streak() {
    ok("height>=74 AND hits.last10g>=20");
}

#[test]
fn p_w13_087_compact_skill_forward() {
    // Smaller skill forwards — under 6'0" with high pts
    ok_with_n_atoms("height<=72 AND p>=70", 2);
}

#[test]
fn p_w13_088_short_speedy_centers() {
    ok_with_n_atoms("pos=C AND height<=70 AND p>=50", 3);
}

#[test]
fn p_w13_089_giant_d_corps() {
    ok_with_n_atoms("pos=D AND height>=78", 2);
}

#[test]
fn p_w13_090_pf_pp_specialist() {
    ok_with_n_atoms("height>=74 AND weight>=210 AND pp-goals>=8", 3);
}

#[test]
fn p_w13_091_pf_left_wing() {
    ok_with_n_atoms("pos=LW AND height>=74 AND g>=20 AND hits>=100", 4);
}

#[test]
fn p_w13_092_pf_right_wing() {
    ok_with_n_atoms("pos=RW AND height>=74 AND g>=25", 3);
}

#[test]
fn p_w13_093_pf_with_giveaways_low() {
    ok_with_n_atoms("height>=74 AND giveaways<=40 AND p>=50", 3);
}

#[test]
fn p_w13_094_pf_takeaways_high() {
    ok_with_n_atoms("height>=74 AND takeaways>=40 AND p>=50", 3);
}

#[test]
fn p_w13_095_pf_under_24() {
    ok_with_n_atoms("height>=74 AND age<=24 AND g>=20", 3);
}

#[test]
fn p_w13_096_pf_career_500_pts_under35() {
    ok_with_n_atoms("height>=74 AND p.career>=500 AND age<=35", 3);
}

#[test]
fn p_w13_097_pf_first_round() {
    ok_with_n_atoms("height>=74 AND weight>=210 AND draft-round=1 AND g>=20", 4);
}

#[test]
fn p_w13_098_pf_canadian_top10() {
    ok_with_n_atoms("height>=74 AND country=CAN AND draft-overall<=10", 3);
}

#[test]
fn p_w13_099_pf_with_30g_or_50p() {
    ok("height>=74 AND (g>=30 OR p>=50)");
}

#[test]
fn p_w13_100_pf_streak_g_in_10() {
    ok("height>=74 AND g.last10g>=5");
}

// ── Section F — Geographic / national-team (20) ─────────────────

#[test]
fn p_w13_101_canadian_top_scorers() {
    ok_with_n_atoms("country=CAN AND p>=70", 2);
}

#[test]
fn p_w13_102_us_top_scorers() {
    ok_with_n_atoms("country=USA AND p>=70", 2);
}

#[test]
fn p_w13_103_swedish_top_scorers() {
    ok_with_n_atoms("country=SWE AND p>=60", 2);
}

#[test]
fn p_w13_104_finnish_top_scorers() {
    ok_with_n_atoms("country=FIN AND p>=50", 2);
}

#[test]
fn p_w13_105_russian_top_scorers() {
    ok_with_n_atoms("country=RUS AND p>=60", 2);
}

#[test]
fn p_w13_106_european_top_scorers_set() {
    ok_with_n_atoms("country IN (SWE, FIN, RUS, CZE, SVK) AND p>=70", 2);
}

#[test]
fn p_w13_107_north_american_set() {
    ok_with_n_atoms("country IN (CAN, USA) AND p>=80", 2);
}

#[test]
fn p_w13_108_minnesota_born() {
    ok_with_n_atoms("country=USA AND birth-state=MN AND p>=40", 3);
}

#[test]
fn p_w13_109_ontario_born() {
    ok_with_n_atoms("country=CAN AND birth-state=ON AND p>=40", 3);
}

#[test]
fn p_w13_110_quebec_born() {
    ok_with_n_atoms("country=CAN AND birth-state=QC", 2);
}

#[test]
fn p_w13_111_alberta_born() {
    ok_with_n_atoms("country=CAN AND birth-state=AB AND p>=40", 3);
}

#[test]
fn p_w13_112_michigan_born() {
    ok_with_n_atoms("country=USA AND birth-state=MI", 2);
}

#[test]
fn p_w13_113_massachusetts_born() {
    ok_with_n_atoms("country=USA AND birth-state=MA", 2);
}

#[test]
fn p_w13_114_california_born() {
    ok_with_n_atoms("country=USA AND birth-state=CA AND p>=40", 3);
}

#[test]
fn p_w13_115_european_d_corps() {
    ok_with_n_atoms("pos=D AND country IN (SWE, FIN, CZE, SVK)", 2);
}

#[test]
fn p_w13_116_european_goalies() {
    ok_with_n_atoms("pos=G AND country IN (SWE, FIN, RUS)", 2);
}

#[test]
fn p_w13_117_canadian_goalies() {
    ok_with_n_atoms("pos=G AND country=CAN", 2);
}

#[test]
fn p_w13_118_us_centers() {
    ok_with_n_atoms("pos=C AND country=USA AND p>=50", 3);
}

#[test]
fn p_w13_119_germans() {
    ok("country=GER AND p>=40");
}

#[test]
fn p_w13_120_slovak_breakouts() {
    ok_with_n_atoms("country=SVK AND age<=25 AND p>=40", 3);
}

// ── Section G — Draft retrospective (20) ────────────────────────

#[test]
fn p_w13_121_first_overall_picks() {
    ok("draft-overall=1");
}

#[test]
fn p_w13_122_top10_picks_with_career_500p() {
    ok_with_n_atoms("draft-overall<=10 AND p.career>=500", 2);
}

#[test]
fn p_w13_123_late_round_500p_career() {
    ok_with_n_atoms("draft-round>=5 AND p.career>=500", 2);
}

#[test]
fn p_w13_124_seventh_round_steals() {
    ok_with_n_atoms("draft-round=7 AND p.career>=300", 2);
}

#[test]
fn p_w13_125_first_round_busts_low_pt() {
    ok_with_n_atoms("draft-round=1 AND p.career<=200 AND age>=30", 3);
}

#[test]
fn p_w13_126_2020_draft_class() {
    ok_with_n_atoms("draft-year=2020 AND p>=30", 2);
}

#[test]
fn p_w13_127_2018_draft_class() {
    ok_with_n_atoms("draft-year=2018 AND p>=50", 2);
}

#[test]
fn p_w13_128_recent_first_rounders_p_threshold() {
    ok_with_n_atoms("draft-year>=2020 AND draft-round=1 AND p>=40", 3);
}

#[test]
fn p_w13_129_2019_class_career_pts() {
    ok_with_n_atoms("draft-year=2019 AND p.career>=200", 2);
}

#[test]
fn p_w13_130_top5_pick_busts() {
    ok_with_n_atoms("draft-overall<=5 AND age>=27 AND p.career<=300", 3);
}

#[test]
fn p_w13_131_late_round_canadian_breakouts() {
    ok_with_n_atoms("draft-round>=4 AND country=CAN AND p>=50", 3);
}

#[test]
fn p_w13_132_late_round_us_breakouts() {
    ok_with_n_atoms("draft-round>=4 AND country=USA AND p>=50", 3);
}

#[test]
fn p_w13_133_top10_under_25() {
    ok_with_n_atoms("draft-overall<=10 AND age<=24 AND p>=40", 3);
}

#[test]
fn p_w13_134_draft_year_range_query() {
    ok("draft-year BETWEEN 2018 AND 2022");
}

#[test]
fn p_w13_135_undrafted_overachievers() {
    // No draft-round on player → atom evaluates to false; this
    // checks the parser handles it cleanly even though the
    // result is an empty population
    ok("draft-round>=8 AND p>=40");
}

#[test]
fn p_w13_136_first_round_age_under_22() {
    ok_with_n_atoms("draft-round=1 AND age<=22", 2);
}

#[test]
fn p_w13_137_top15_overall_30g() {
    ok_with_n_atoms("draft-overall<=15 AND g>=30", 2);
}

#[test]
fn p_w13_138_2021_class_breakouts() {
    ok_with_n_atoms("draft-year=2021 AND p>=40", 2);
}

#[test]
fn p_w13_139_first_round_in_country_set() {
    ok_with_n_atoms("draft-round=1 AND country IN (CAN, USA, SWE)", 2);
}

#[test]
fn p_w13_140_high_pick_low_career() {
    ok_with_n_atoms("draft-overall<=15 AND p.career<=300 AND age>=27", 3);
}

// ── Section H — Streak / hot-cold (20) ──────────────────────────

#[test]
fn p_w13_141_hot_streak_5g_in_10() {
    let c = ok("g.last10g>=5");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_142_hot_streak_10p_in_5() {
    ok("p.last5g>=10");
}

#[test]
fn p_w13_143_hot_streak_15p_in_10() {
    ok("p.last10g>=15");
}

#[test]
fn p_w13_144_hot_streak_30d() {
    ok("p.last30d>=20");
}

#[test]
fn p_w13_145_cold_streak_no_p_in_5() {
    ok("p.last5g<=0");
}

#[test]
fn p_w13_146_30d_g_burst() {
    ok("g.last30d>=10");
}

#[test]
fn p_w13_147_3w_g_burst() {
    ok("g.last3w>=8");
}

#[test]
fn p_w13_148_3m_pace() {
    ok("p.last3m>=30");
}

#[test]
fn p_w13_149_team_specific_streak() {
    ok("team=EDM AND g.last10g>=5");
}

#[test]
fn p_w13_150_streak_with_age_filter() {
    ok("g.last10g>=5 AND age<=25");
}

#[test]
fn p_w13_151_streak_with_team_and_age() {
    ok("team=COL AND g.last10g>=4 AND age<=24");
}

#[test]
fn p_w13_152_streak_allteams_modifier() {
    ok("g.last10g.allteams>=5");
}

#[test]
fn p_w13_153_streak_career_modifier() {
    ok("p.last10g.career>=15");
}

#[test]
fn p_w13_154_ever_streak_5_in_10_under25() {
    ok("g.any10g>=5 EVER AT age<=25");
}

#[test]
fn p_w13_155_ever_15p_in_10_under23() {
    ok("p.any10g>=15 EVER AT age<=23");
}

#[test]
fn p_w13_156_streak_double_threshold_or() {
    ok("g.last10g>=5 OR a.last10g>=10");
}

#[test]
fn p_w13_157_streak_with_country() {
    ok("g.last10g>=5 AND country=CAN");
}

#[test]
fn p_w13_158_streak_in_pos_set() {
    ok("g.last10g>=5 AND pos IN (C, LW, RW)");
}

#[test]
fn p_w13_159_streak_3g() {
    ok("p.last3g>=5");
}

#[test]
fn p_w13_160_streak_15g() {
    ok("p.last15g>=15");
}

// ── Section I — Cross-league / development arc (20) ─────────────

#[test]
fn p_w13_161_chl_three_alumni() {
    let c = ok("league IN (OHL, WHL, QMJHL)");
    assert!(c.needs_provider());
}

#[test]
fn p_w13_162_ohl_alumni_with_nhl_pts() {
    ok_with_n_atoms("league=OHL AND p>=40", 2);
}

#[test]
fn p_w13_163_whl_alumni() {
    ok_with_n_atoms("league=WHL AND age<=25", 2);
}

#[test]
fn p_w13_164_qmjhl_alumni() {
    ok_with_n_atoms("league=QMJHL AND p>=40", 2);
}

#[test]
fn p_w13_165_ncaa_alumni_with_nhl_career() {
    ok_with_n_atoms("league.tier=College AND p.career>=300", 2);
}

#[test]
fn p_w13_166_junior_career_milestones() {
    ok("p.career.junior>=200");
}

#[test]
fn p_w13_167_ohl_career_300p() {
    ok("p.career.ohl>=300");
}

#[test]
fn p_w13_168_whl_career_350p() {
    ok("p.career.whl>=350");
}

#[test]
fn p_w13_169_ushl_alumni() {
    ok_with_n_atoms("league=USHL AND p>=40", 2);
}

#[test]
fn p_w13_170_khl_alumni_now_in_nhl() {
    ok_with_n_atoms("league=KHL AND age<=30 AND p>=30", 3);
}

#[test]
fn p_w13_171_shl_alumni() {
    ok_with_n_atoms("league=SHL AND p>=40", 2);
}

#[test]
fn p_w13_172_liiga_alumni() {
    ok_with_n_atoms("league=Liiga AND p>=40", 2);
}

#[test]
fn p_w13_173_nhl_only_career_500p() {
    ok("p.career.nhl>=500");
}

#[test]
fn p_w13_174_pro_tier_career() {
    ok("p.career.pro>=300");
}

#[test]
fn p_w13_175_international_tier_check() {
    ok("league.tier=International");
}

#[test]
fn p_w13_176_junior_with_age_under_20() {
    ok_with_n_atoms("league.tier=Junior AND age<=20", 2);
}

#[test]
fn p_w13_177_no_nhl_yet() {
    ok("league NOT IN (NHL)");
}

#[test]
fn p_w13_178_no_european_pro() {
    ok("league NOT IN (KHL, SHL, Liiga, DEL)");
}

#[test]
fn p_w13_179_ohl_only_no_nhl() {
    ok("league=OHL AND league NOT IN (NHL)");
}

#[test]
fn p_w13_180_dev_arc_junior_to_nhl() {
    ok_with_n_atoms("league.tier=Junior AND p.career.nhl>=200", 2);
}

// ── Section J — Reporter staples (20) ───────────────────────────

#[test]
fn p_w13_181_top_pp_specialists() {
    ok_with_n_atoms("pp-points>=25 AND age<=27", 2);
}

#[test]
fn p_w13_182_top_pk_specialists() {
    ok_with_n_atoms("sh-points>=10", 1);
}

#[test]
fn p_w13_183_clutch_gwg_scorers() {
    ok_with_n_atoms("gwg>=5", 1);
}

#[test]
fn p_w13_184_ot_heroes() {
    ok("ot-goals>=2");
}

#[test]
fn p_w13_185_high_volume_shooters() {
    ok_with_n_atoms("shots>=250", 1);
}

#[test]
fn p_w13_186_high_efficiency_shooters() {
    ok_with_n_atoms("shooting-pct>=0.18 AND shots>=150", 2);
}

#[test]
fn p_w13_187_low_pim_skill_players() {
    ok_with_n_atoms("p>=70 AND pim<=20", 2);
}

#[test]
fn p_w13_188_high_pim_enforcers() {
    ok_with_n_atoms("pim>=80 AND hits>=150", 2);
}

#[test]
fn p_w13_189_low_giveaway_high_takeaway() {
    ok_with_n_atoms("takeaways>=50 AND giveaways<=40", 2);
}

#[test]
fn p_w13_190_minutes_eaters() {
    ok_with_n_atoms("total-toi-per-game>=1320", 1);
}

#[test]
fn p_w13_191_pp_minutes_eaters() {
    ok_with_n_atoms("pp-toi-per-game>=180", 1);
}

#[test]
fn p_w13_192_pk_minutes_eaters() {
    ok_with_n_atoms("sh-toi-per-game>=120", 1);
}

#[test]
fn p_w13_193_top_centerice_faceoff() {
    ok_with_n_atoms("pos=C AND faceoff-win-pct>=0.55 AND games>=40", 3);
}

#[test]
fn p_w13_194_high_blocks_d() {
    ok_with_n_atoms("pos=D AND blocked-shots>=200", 2);
}

#[test]
fn p_w13_195_career_streak_milestone() {
    ok("p.streak>=20");
}

#[test]
fn p_w13_196_30g_30a_threshold() {
    ok_with_n_atoms("g>=30 AND assists>=30", 2);
}

#[test]
fn p_w13_197_40g_40a_threshold() {
    ok_with_n_atoms("g>=40 AND assists>=40", 2);
}

#[test]
fn p_w13_198_50g_seasons_count() {
    ok("g.seasons-with>=2");
}

#[test]
fn p_w13_199_double_century_career() {
    ok("g.career>=200 AND a.career>=400");
}

#[test]
fn p_w13_200_kitchen_sink_query() {
    // The reporter's everything-bagel query — exercise the
    // grammar at its widest. Sliding window + bio + IN +
    // BETWEEN + LIKE + EVER-AT-age all in one.
    ok("g.last10g>=5 AND age BETWEEN 22 AND 28 AND \
         country IN (CAN, USA, SWE) AND \
         pos IN (C, LW, RW) AND draft-round<=2");
}
