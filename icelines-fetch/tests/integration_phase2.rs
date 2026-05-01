/// L1 Phase 2 integration tests — scheme scoring and filter engine.
///
/// Hart.5c.4: rewritten against StatsRepository + PlayerView fixture
/// pattern. The 5 known-value Beniers / fantasy assertions
/// (179.0 / 130.0 / 122.0 / 50.0 / 440.0) are preserved — those are
/// pure scheme tests with no Player coupling. Filter tests now build a
/// small StatsRepository and exercise PlayerFilter::apply_views.
use icelines_core::{
    filter::PlayerFilter,
    identity::PlayerId,
    model::{Season, Position},
    scheme::{compute_fantasy_score, Scheme, SkaterStats},
    season_stats::{SeasonStatsBuilder, SeasonType, StatTotals, TeamStint},
    stats_repository::{PlayerView, StatsRepository},
    PaceScore, TeamAbbr,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

const SEASON: Season = Season(20242025);
const STYPE: SeasonType = SeasonType::Regular;

/// Beniers's documented 2025-26 stats line. Pure scheme input — no Player or
/// PlayerView in the path. Pinned across the migration so the
/// known-value Yahoo/ESPN/simple-pts assertions don't drift.
fn beniers_stats() -> SkaterStats {
    // Matty Beniers 2025-26: 20G, 30A, 82GP, PPG=6, PPA=5, GWG=1, HIT=31, BLK=69
    SkaterStats {
        goals: 20,
        assists: 30,
        pp_goals: 6,
        pp_assists: 5,
        gwg: 1,
        hits: 31,
        blocks: 69,
        ..Default::default()
    }
}

/// Upsert a synthetic skater into `repo` for filter-engine tests. Builds
/// a minimal SeasonStats with the given counters; pace is computed.
fn upsert_skater(
    repo: &mut StatsRepository,
    pid: u32,
    name: &str,
    team: &str,
    pos: Position,
    g: u32,
    a: u32,
    gp: u32,
) {
    let id = icelines_core::fixtures::identity(pid)
        .name(name, &name.to_lowercase())
        .build();
    let totals = StatTotals {
        gp, goals: g, assists: a, points: g + a,
        plus_minus: 0, pim: 0, shots: 0,
        shooting_pct: None, toi_per_game_sec: None,
        pp_goals: 0, pp_points: 0, sh_goals: 0, sh_points: 0,
        gwg: 0, ot_goals: 0, faceoff_win_pct: None,
        pace_score: if gp >= 10 {
            let pace_82 = (g + a) as f64 / gp as f64 * 82.0;
            let goals_per_82 = g as f64 / gp as f64 * 82.0;
            Some(PaceScore { pace_82, goals_per_82, raw_points: g + a, gp })
        } else {
            None
        },
    };
    let stint = TeamStint {
        team: TeamAbbr(team.to_owned()),
        started: Some("2024-10-15".into()),
        ended: Some("2025-04-13".into()),
        gp, goals: g, assists: a, points: g + a,
        goalie: None,
    };
    let stats = SeasonStatsBuilder::new(PlayerId(pid), SEASON, STYPE, pos)
        .with_totals(totals)
        .add_team_stint(stint)
        .build();
    repo.upsert_identity(id).unwrap();
    repo.upsert_stats(stats).unwrap();
}

fn upsert_skater_full(
    repo: &mut StatsRepository,
    pid: u32,
    name: &str,
    g: u32,
    a: u32,
    gp: u32,
    toi_sec: u32,
    plus_minus: i32,
    shots: u32,
) {
    let id = icelines_core::fixtures::identity(pid)
        .name(name, &name.to_lowercase())
        .build();
    let totals = StatTotals {
        gp, goals: g, assists: a, points: g + a,
        plus_minus, pim: 0, shots,
        shooting_pct: None, toi_per_game_sec: Some(toi_sec),
        pp_goals: 0, pp_points: 0, sh_goals: 0, sh_points: 0,
        gwg: 0, ot_goals: 0, faceoff_win_pct: None,
        pace_score: if gp >= 10 {
            let pace_82 = (g + a) as f64 / gp as f64 * 82.0;
            let goals_per_82 = g as f64 / gp as f64 * 82.0;
            Some(PaceScore { pace_82, goals_per_82, raw_points: g + a, gp })
        } else {
            None
        },
    };
    let stint = TeamStint {
        team: TeamAbbr("SEA".into()),
        started: Some("2024-10-15".into()),
        ended: Some("2025-04-13".into()),
        gp, goals: g, assists: a, points: g + a,
        goalie: None,
    };
    let stats = SeasonStatsBuilder::new(PlayerId(pid), SEASON, STYPE, Position::Center)
        .with_totals(totals)
        .add_team_stint(stint)
        .build();
    repo.upsert_identity(id).unwrap();
    repo.upsert_stats(stats).unwrap();
}

fn views<'r>(repo: &'r StatsRepository) -> Vec<PlayerView<'r>> {
    repo.skaters(SEASON, STYPE).collect()
}

// ── L1: Fantasy scheme scoring (pure — no Player/PlayerView) ─────────────────
//
// Known-value assertions pinned per spec Bench B1: 179.0, 50.0, 130.0,
// 122.0, 440.0, 195.0. These tests don't touch the migration surface,
// they exercise `compute_fantasy_score` against `SkaterStats`. Kept as-is.

#[test]
fn l1_scheme_yahoo_standard_beniers_179() {
    // G=3, A=2, PPG=1, PPA=0.5, GWG=0.5, HIT=0.5, BLK=0.5
    // 20×3 + 30×2 + 6×1 + 5×0.5 + 1×0.5 + 31×0.5 + 69×0.5
    //  60  +  60  +  6  +  2.5  +  0.5  +  15.5  +  34.5  = 179.0
    let score =
        compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
    assert!(
        (score.total - 179.0).abs() < 0.001,
        "expected 179.0, got {}",
        score.total
    );
}

#[test]
fn l1_scheme_breakdown_sum_invariant() {
    let score =
        compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
    let sum: f32 = score.breakdown.values().sum();
    assert!(
        (sum - score.total).abs() < 0.001,
        "breakdown sum {} != total {}",
        sum,
        score.total
    );
}

#[test]
fn l1_scheme_simple_pts_equals_goals_plus_assists() {
    let score = compute_fantasy_score(&beniers_stats(), &Scheme::simple_pts().skater, 82).unwrap();
    assert!(
        (score.total - 50.0).abs() < 0.001,
        "simple-pts should be G+A = 50, got {}",
        score.total
    );
}

#[test]
fn l1_scheme_espn_higher_than_yahoo_for_offensive_player() {
    let yahoo =
        compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
    let espn =
        compute_fantasy_score(&beniers_stats(), &Scheme::espn_standard().skater, 82).unwrap();
    assert!(
        espn.total > yahoo.total,
        "ESPN (heavy goals/assists) should score higher than Yahoo for a scorer"
    );
}

#[test]
fn l1_fantasy_score_includes_hits_and_blocks() {
    // Yahoo standard: hits=0.5, blocks=0.5
    // 20G(3), 30A(2), 100 hits(0.5), 50 blocks(0.5)
    // = 60 + 60 + 50 + 25 = 195
    let stats = SkaterStats {
        goals: 20, assists: 30, hits: 100, blocks: 50,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    assert!(
        (score.total - 195.0).abs() < 0.001,
        "expected 195.0, got {}",
        score.total
    );
    assert!(score.breakdown.contains_key("hits"), "hits must appear in breakdown");
    assert!(score.breakdown.contains_key("blocks"), "blocks must appear in breakdown");
}

#[test]
fn l1_fantasy_score_includes_pp_components() {
    // Yahoo standard: ppG=1, ppA=0.5
    // 6 pp_goals = 6.0 bonus, 8 pp_assists = 4.0 bonus
    let stats = SkaterStats {
        goals: 20, assists: 30, pp_goals: 6, pp_assists: 8,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    // G=60, A=60, PPG=6, PPA=4 → 130
    assert!(
        (score.total - 130.0).abs() < 0.001,
        "expected 130.0, got {}",
        score.total
    );
    assert!(score.breakdown.contains_key("pp_goals"));
    assert!(score.breakdown.contains_key("pp_assists"));
}

#[test]
fn l1_fantasy_score_includes_gwg() {
    let stats = SkaterStats {
        goals: 20, assists: 30, gwg: 4,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    // G=60, A=60, GWG=2 → 122
    assert!(
        (score.total - 122.0).abs() < 0.001,
        "expected 122.0, got {}",
        score.total
    );
    assert!(score.breakdown.contains_key("gwg"), "gwg must appear in breakdown");
}

#[test]
fn l1_fantasy_score_negative_giveaways_reduce_total() {
    // Custom scheme with giveaways penalty
    let weights = icelines_core::scheme::SkaterWeights {
        goals: 3.0, assists: 2.0, giveaways: -0.5,
        ..Default::default()
    };
    let stats = SkaterStats {
        goals: 10, assists: 20, giveaways: 40,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &weights, 82).unwrap();
    // G=30, A=40, giveaways=-20 → 50
    assert!(
        (score.total - 50.0).abs() < 0.001,
        "expected 50.0 with giveaway penalty, got {}",
        score.total
    );
}

#[test]
fn l1_fantasy_score_espn_includes_shots_on_goal() {
    let stats = SkaterStats {
        goals: 20, assists: 30, shots_on_goal: 200,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::espn_standard().skater, 82).unwrap();
    // G=120, A=120, SOG=200 → 440
    assert!(
        (score.total - 440.0).abs() < 0.001,
        "expected 440.0, got {}",
        score.total
    );
}

#[test]
fn l1_fantasy_score_per_game_is_total_over_gp() {
    let stats = beniers_stats();
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    assert!(
        (score.per_game - score.total / 82.0).abs() < 0.001,
        "per_game should be total/gp"
    );
    assert_eq!(score.gp, 82);
}

// ── L1: PlayerFilter — exercise apply_views on a real StatsRepository ────────

fn fixture_repo() -> StatsRepository {
    let mut repo = StatsRepository::new();
    upsert_skater(&mut repo, 1, "McDavid", "EDM", Position::Center, 50, 90, 82);
    upsert_skater(&mut repo, 2, "Beniers", "SEA", Position::Center, 20, 30, 82);
    upsert_skater(&mut repo, 3, "Tolvanen", "SEA", Position::LeftWing, 12, 24, 78);
    upsert_skater(&mut repo, 4, "Eberle", "SEA", Position::RightWing, 26, 29, 80);
    upsert_skater(&mut repo, 5, "Makar", "COL", Position::Defense, 21, 74, 82);
    repo
}

#[test]
fn l1_filter_by_position_centers_only() {
    let repo = fixture_repo();
    let mut f = PlayerFilter::new();
    f.positions = Some(vec![Position::Center]);
    let result = f.apply_views(views(&repo).into_iter());
    assert!(result.iter().all(|v| v.position() == Position::Center));
    assert_eq!(result.len(), 2);
}

#[test]
fn l1_filter_by_team_sea_only() {
    let repo = fixture_repo();
    let mut f = PlayerFilter::new();
    f.teams = Some(vec!["SEA".to_owned()]);
    let result = f.apply_views(views(&repo).into_iter());
    assert!(result.iter().all(|v| v.team_display() == "SEA"));
    assert_eq!(result.len(), 3);
}

#[test]
fn l1_filter_ppg_min_excludes_low_scorers() {
    let repo = fixture_repo();
    let mut f = PlayerFilter::new();
    // McDavid PPG = 1.707, Beniers = 0.610. Only McDavid above 1.5.
    f.ppg_min = Some(1.5);
    let result = f.apply_views(views(&repo).into_iter());
    assert_eq!(result.len(), 1, "only McDavid should exceed 1.5 PPG");
    assert_eq!(result[0].identity.full_name, "McDavid");
}

#[test]
fn l1_filter_combined_pos_and_team() {
    let repo = fixture_repo();
    let mut f = PlayerFilter::new();
    f.positions = Some(vec![Position::Center]);
    f.teams = Some(vec!["SEA".to_owned()]);
    let result = f.apply_views(views(&repo).into_iter());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].identity.full_name, "Beniers");
}

#[test]
fn l1_filter_no_match_returns_empty() {
    let repo = fixture_repo();
    let mut f = PlayerFilter::new();
    f.teams = Some(vec!["BOS".to_owned()]);
    let result = f.apply_views(views(&repo).into_iter());
    assert!(result.is_empty(), "no BOS players in fixture");
}

#[test]
fn l1_filter_gp_min_excludes_below_threshold() {
    let repo = fixture_repo();
    let mut f = PlayerFilter::new();
    f.gp_min = Some(80);
    let result = f.apply_views(views(&repo).into_iter());
    // Tolvanen has 78 GP — excluded
    assert!(!result.iter().any(|v| v.identity.full_name == "Tolvanen"));
    // Eberle 80, Beniers/McDavid/Makar 82 — included
    assert!(result.iter().any(|v| v.identity.full_name == "Eberle"));
}

// ── L1: Filter — combined statistical thresholds ─────────────────────────────

#[test]
fn l1_filter_combined_toi_plus_minus_shots() {
    let mut repo = StatsRepository::new();
    upsert_skater_full(&mut repo, 100, "Elite",    15, 25, 82, 1400, 20, 200);
    upsert_skater_full(&mut repo, 101, "LowTOI",   15, 25, 82,  800, 15, 180);
    upsert_skater_full(&mut repo, 102, "Negative", 15, 25, 82, 1300, -10, 190);
    upsert_skater_full(&mut repo, 103, "FewShots", 15, 25, 82, 1250,   5,  60);

    let mut f = PlayerFilter::new();
    f.toi_min_sec     = Some(1200.0);
    f.plus_minus_min  = Some(0);
    f.shots_pg_min    = Some(2.0);
    let result = f.apply_views(views(&repo).into_iter());
    assert_eq!(result.len(), 1, "only Elite should pass all three filters");
    assert_eq!(result[0].identity.full_name, "Elite");
}
