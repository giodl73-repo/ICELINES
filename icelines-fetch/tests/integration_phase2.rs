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
        pp_goals: 0, pp_points: 0,
        sh_goals: 0, sh_points: 0,
        gwg: 0, ot_goals: 0,
        shots: 0, shooting_pct: None,
        plus_minus: 0,
        toi_per_game_sec: None,
        faceoff_win_pct: None,
        hits: 0, blocked_shots: 0, missed_shots: 0,
        giveaways: 0, takeaways: 0, pim: 0,
        xg: None, xg_per_60: None, cf_pct_5v5: None, ff_pct_5v5: None, xgf_pct_5v5: None,
        headshot_url: None,
        sweater_number: None,
        birth_date: None,
        birth_country: None,
        nationality_code: None,
        birth_city: None,
        birth_state_province: None,
        shoots_catches: None,
        height_in_inches: None,
        weight_lbs: None,
        draft_year: None,
        draft_round: None,
        draft_overall: None,
        rookie_season: None,
        contract_expiry_year: None,
        expiry_type: None,
        salary: None,
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
    // McDavid PPG = (50+90)/82 = 1.707, Beniers = (20+30)/82 = 0.610
    // Only McDavid above 1.5 PPG
    f.ppg_min = Some(1.5);
    let result = f.apply(&players);
    assert_eq!(result.len(), 1, "only McDavid should exceed 1.5 PPG");
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

// ── L1: Fantasy scoring bridge — Player stats flow through to FantasyScore ────

fn make_player_with_realtime(name: &str, hits: u32, blocks: u32, takeaways: u32) -> Player {
    // make_player signature: (name, team, pos, g, a, gp)
    let mut p = make_player(name, "SEA", icelines_core::model::Position::Center, 20, 30, 82);
    p.hits = hits;
    p.blocked_shots = blocks;
    p.takeaways = takeaways;
    p
}

#[test]
fn l1_fantasy_score_includes_hits_and_blocks() {
    // Yahoo standard: hits=0.5, blocks=0.5
    // Player: 20G(3), 30A(2), 0pp, 100 hits(0.5), 50 blocks(0.5)
    // = 60 + 60 + 100*0.5 + 50*0.5 = 60 + 60 + 50 + 25 = 195
    let p = make_player_with_realtime("Hitter", 100, 50, 0);
    let stats = SkaterStats {
        goals: p.season_goals,
        assists: p.season_assists,
        hits: p.hits,
        blocks: p.blocked_shots,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    assert!(
        (score.total - 195.0).abs() < 0.001,
        "expected 195.0, got {}",
        score.total
    );
    // Breakdown must include hits and blocks
    assert!(score.breakdown.contains_key("hits"), "hits must appear in breakdown");
    assert!(score.breakdown.contains_key("blocks"), "blocks must appear in breakdown");
}

#[test]
fn l1_fantasy_score_includes_pp_components() {
    // Yahoo standard: ppG=1, ppA=0.5
    // 6 pp_goals = 6.0 bonus, 8 pp_assists = 4.0 bonus
    let stats = SkaterStats {
        goals: 20,
        assists: 30,
        pp_goals: 6,
        pp_assists: 8,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    // G=20*3=60, A=30*2=60, PPG=6*1=6, PPA=8*0.5=4 → 130
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
    // Yahoo standard: gwg=0.5
    let stats = SkaterStats {
        goals: 20,
        assists: 30,
        gwg: 4,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::yahoo_standard().skater, 82).unwrap();
    // G=60, A=60, GWG=4*0.5=2 → 122
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
        goals: 3.0,
        assists: 2.0,
        giveaways: -0.5,
        ..Default::default()
    };
    let stats = SkaterStats {
        goals: 10,
        assists: 20,
        giveaways: 40,
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
    // ESPN standard: shots_on_goal=1
    let stats = SkaterStats {
        goals: 20,
        assists: 30,
        shots_on_goal: 200,
        ..Default::default()
    };
    let score = compute_fantasy_score(&stats, &Scheme::espn_standard().skater, 82).unwrap();
    // G=20*6=120, A=30*4=120, SOG=200*1=200 → 440
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

// ── L1: Filter — new statistical threshold filters combined ──────────────────

fn make_player_full(
    name: &str,
    gp: u32,
    toi_sec: f32,
    plus_minus: i32,
    shots: u32,
) -> Player {
    // make_player signature: (name, team, pos, g, a, gp)
    let mut p = make_player(name, "SEA", icelines_core::model::Position::Center, 15, 25, gp);
    p.toi_per_game_sec = Some(toi_sec);
    p.plus_minus = plus_minus;
    p.shots = shots;
    p
}

#[test]
fn l1_filter_combined_toi_plus_minus_shots() {
    // Only "Elite" passes all three thresholds
    let players = vec![
        make_player_full("Elite",    82, 1400.0,  20, 200),  // all pass
        make_player_full("Low TOI",  82,  800.0,  15, 180),  // TOI fails
        make_player_full("Negative", 82, 1300.0, -10, 190),  // plus-minus fails
        make_player_full("FewShots", 82, 1250.0,   5,  60),  // shots fails
    ];
    let mut f = PlayerFilter::new();
    f.toi_min_sec     = Some(1200.0);         // ≥ 20 min/game
    f.plus_minus_min  = Some(0);              // ≥ even
    f.shots_pg_min    = Some(2.0);            // ≥ 2 shots/game
    let result = f.apply(&players);
    assert_eq!(result.len(), 1, "only Elite should pass all three filters");
    assert_eq!(result[0].full_name, "Elite");
}
