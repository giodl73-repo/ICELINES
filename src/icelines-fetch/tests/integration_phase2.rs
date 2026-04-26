/// L1 Phase 2 integration tests — scheme scoring and filter engine.
/// No live network. Uses fixture data and computed values.
use icelines_core::{
    filter::PlayerFilter,
    model::{GpStatus, Player, Position},
    name::normalize_name,
    scheme::{compute_fantasy_score, Scheme, SkaterStats},
    scoring::compute_pace_score,
    TeamAbbr,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_player(name: &str, team: &str, pos: Position, g: u32, a: u32, gp: u32) -> Player {
    Player {
        nhl_id: None,
        full_name: name.to_owned(),
        name_normalized: normalize_name(name),
        team: TeamAbbr(team.to_owned()),
        position: pos,
        eligible_pos: vec![pos],
        gp_status: GpStatus::from_gp(gp),
        season_goals: g,
        season_assists: a,
        season_points: g + a,
        pace_score: compute_pace_score(g, a, gp),
        headshot_url: None,
        birth_date: None,
        birth_country: None,
        nationality_code: None,
        shoots_catches: None,
        draft_year: None,
        draft_round: None,
        draft_overall: None,
        rookie_season: None,
    }
}

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

// ── L1: Fantasy scheme scoring ────────────────────────────────────────────────

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
    // simple-pts: G=1, A=1 → 20+30 = 50
    let score = compute_fantasy_score(&beniers_stats(), &Scheme::simple_pts().skater, 82).unwrap();
    assert!(
        (score.total - 50.0).abs() < 0.001,
        "simple-pts should be G+A = 50, got {}",
        score.total
    );
}

#[test]
fn l1_scheme_espn_higher_than_yahoo_for_offensive_player() {
    // ESPN weights goals(6) and assists(4) higher — total should be larger
    let yahoo =
        compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
    let espn =
        compute_fantasy_score(&beniers_stats(), &Scheme::espn_standard().skater, 82).unwrap();
    assert!(
        espn.total > yahoo.total,
        "ESPN (heavy goals/assists) should score higher than Yahoo for a scorer"
    );
}

// ── L1: PlayerFilter engine ───────────────────────────────────────────────────

fn players_fixture() -> Vec<Player> {
    vec![
        make_player("McDavid", "EDM", Position::Center, 50, 90, 82),
        make_player("Beniers", "SEA", Position::Center, 20, 30, 82),
        make_player("Tolvanen", "SEA", Position::LeftWing, 12, 24, 78),
        make_player("Eberle", "SEA", Position::RightWing, 26, 29, 80),
        make_player("Makar", "COL", Position::Defense, 21, 74, 82),
    ]
}

#[test]
fn l1_filter_by_position_centers_only() {
    let players = players_fixture();
    let mut f = PlayerFilter::new();
    f.positions = Some(vec![Position::Center]);
    let result = f.apply(&players);
    assert!(result.iter().all(|p| p.position == Position::Center));
    assert_eq!(result.len(), 2);
}

#[test]
fn l1_filter_by_team_sea_only() {
    let players = players_fixture();
    let mut f = PlayerFilter::new();
    f.teams = Some(vec!["SEA".to_owned()]);
    let result = f.apply(&players);
    assert!(result.iter().all(|p| p.team.as_str() == "SEA"));
    assert_eq!(result.len(), 3);
}

#[test]
fn l1_filter_ppg_min_excludes_low_scorers() {
    let players = players_fixture();
    let mut f = PlayerFilter::new();
    // McDavid pace = 140, Beniers = 59.5 — only McDavid above 100
    f.ppg_min = Some(100.0);
    let result = f.apply(&players);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].full_name, "McDavid");
}

#[test]
fn l1_filter_combined_pos_and_team() {
    let players = players_fixture();
    let mut f = PlayerFilter::new();
    f.positions = Some(vec![Position::Center]);
    f.teams = Some(vec!["SEA".to_owned()]);
    let result = f.apply(&players);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].full_name, "Beniers");
}

#[test]
fn l1_filter_no_match_returns_empty() {
    let players = players_fixture();
    let mut f = PlayerFilter::new();
    f.teams = Some(vec!["BOS".to_owned()]);
    let result = f.apply(&players);
    assert!(result.is_empty(), "no BOS players in fixture");
}

#[test]
fn l1_filter_gp_min_excludes_below_threshold() {
    let players = players_fixture();
    let mut f = PlayerFilter::new();
    f.gp_min = Some(80);
    let result = f.apply(&players);
    // Tolvanen has 78 GP — excluded
    assert!(!result.iter().any(|p| p.full_name == "Tolvanen"));
    // Eberle 80, Beniers/McDavid/Makar 82 — included
    assert!(result.iter().any(|p| p.full_name == "Eberle"));
}
