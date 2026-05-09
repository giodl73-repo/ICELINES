//! Phase Art Ross Wave 15 — mid-season trade semantics +
//! property-based monotonicity over real bundled data.
//!
//! ## Findings while planning this wave
//!
//! 1. **Bundle data squashes multi-stint into compound team
//!    names.** Tarasenko's 2022-23 (STL→NYR) loads as a single
//!    TeamStint with team="STL/NYR" — the StatsRepository sees
//!    `team_stints.len() == 1` always when reading bundled data.
//!    This is a snapshot-format finding, not an executor bug.
//!    Mid-season trade tests against real data therefore can't
//!    distinguish `team=STL` from `team=NYR` — they both match
//!    "STL/NYR" by neither.
//!
//! 2. To actually verify mid-season trade SEMANTICS in the
//!    executor, we use the same synthetic-repo pattern from
//!    `art_ross_a2_executor.rs` — hand-construct a player with
//!    explicit multi-stint TeamStint records and verify
//!    `team=` (current stint) vs `team.any=` (any stint this
//!    season) behavior.

use std::collections::HashSet;

use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::StatsRepository;
use icelines_fetch::stats_loader::load_player_career_into_repo;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{parse_query, FilterInput, StrictMode};

struct NoOpProvider;
impl DataProvider for NoOpProvider {
    fn ensure(
        &self,
        _req: &PlanRequirement,
        _events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError> {
        Ok(())
    }
}

fn fixed_today() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

// ── Synthetic multi-stint players ────────────────────────────

/// Build a synthetic StatsRepository containing one mid-season-
/// traded player. EDM → DAL trade. Stints are explicit.
fn synthetic_traded_player_repo() -> StatsRepository {
    use icelines_core::identity::{PlayerBio, PlayerIdentity};
    use icelines_core::model::{Position, TeamAbbr};
    use icelines_core::season_stats::{SeasonStats, StatTotals, TeamStint};

    let mut repo = StatsRepository::with_lru_cap(8);
    let pid = PlayerId(9999401);

    repo.upsert_identity(PlayerIdentity {
        id: pid,
        full_name: "Test Traded".to_string(),
        name_normalized: "test traded".to_string(),
        headshot_canonical_url: None,
        bio: PlayerBio {
            birth_date: Some("2000-01-15".to_string()),
            birth_country: Some("CAN".to_string()),
            nationality_code: Some("CAN".to_string()),
            birth_city: Some("Toronto".to_string()),
            birth_state_province: Some("ON".to_string()),
            height_in_inches: Some(72),
            weight_lbs: Some(190),
            draft_year: Some(2018),
            draft_round: Some(1),
            draft_overall: Some(10),
            shoots_catches: Some("L".to_string()),
            rookie_season: Some("20182019".to_string()),
        },
    })
    .unwrap();

    // Two stints: EDM first (Oct-Feb), then DAL (Feb-Apr).
    repo.upsert_stats(SeasonStats {
        player_id: pid,
        season: Season(20252026),
        season_type: SeasonType::Regular,
        position: Position::Center,
        sweater_number: Some(13),
        team_stints: vec![
            TeamStint {
                team: TeamAbbr("EDM".to_string()),
                started: Some("2025-10-08".to_string()),
                ended: Some("2026-02-15".to_string()),
                gp: 50,
                goals: 20,
                assists: 30,
                points: 50,
                goalie: None,
            },
            TeamStint {
                team: TeamAbbr("DAL".to_string()),
                started: Some("2026-02-16".to_string()),
                ended: None,
                gp: 25,
                goals: 10,
                assists: 15,
                points: 25,
                goalie: None,
            },
        ],
        totals: StatTotals::default(),
        realtime: None,
        advanced: None,
        goalie: None,
        time_on_ice: None,
        goals_for_against: None,
        goalie_advanced: None,
        goalie_saves_by_strength: None,
        goalie_bios: None,
    })
    .unwrap();

    repo
}

fn matches_filter(repo: &StatsRepository, pid: u32, season: u32, filter: &str) -> bool {
    let plan = parse_query(FilterInput::Cli(filter.into())).unwrap();
    let provider = NoOpProvider;
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), season);
    let view = repo
        .view(PlayerId(pid), Season(season), SeasonType::Regular)
        .expect("view present");
    plan.root.matches(&view, &ctx)
}

const TRADED_PID: u32 = 9999401;
const TRADED_SEASON: u32 = 20252026;

// ── Trade semantic verification ──────────────────────────────

/// `team=DAL` matches because DAL is the CURRENT (last) stint.
#[test]
fn w15_team_eq_current_stint_matches() {
    let repo = synthetic_traded_player_repo();
    assert!(
        matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team=DAL"),
        "team=DAL should match the current stint"
    );
}

/// `team=EDM` does NOT match — EDM was the OLD stint, not current.
/// This is the locked decision from the spec.
#[test]
fn w15_team_eq_old_stint_does_not_match() {
    let repo = synthetic_traded_player_repo();
    assert!(
        !matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team=EDM"),
        "team=EDM should NOT match a traded player whose current stint is DAL"
    );
}

/// `team.any=EDM` matches — `.any` modifier checks any stint
/// this season, not just the current one.
#[test]
fn w15_team_any_old_stint_matches() {
    let repo = synthetic_traded_player_repo();
    assert!(
        matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team.any=EDM"),
        "team.any=EDM should match the OLD stint of a traded player"
    );
}

#[test]
fn w15_team_any_current_stint_matches() {
    let repo = synthetic_traded_player_repo();
    assert!(
        matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team.any=DAL"),
        "team.any=DAL should also match the current stint"
    );
}

#[test]
fn w15_team_any_unrelated_team_excludes() {
    let repo = synthetic_traded_player_repo();
    assert!(
        !matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team.any=BOS"),
        "team.any=BOS should NOT match — player never played BOS"
    );
}

/// Ne semantics on team= follow the current-stint rule.
#[test]
fn w15_team_ne_current_excludes() {
    let repo = synthetic_traded_player_repo();
    assert!(
        !matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team!=DAL"),
        "team!=DAL should NOT match — DAL IS the current stint"
    );
}

#[test]
fn w15_team_ne_old_includes() {
    let repo = synthetic_traded_player_repo();
    assert!(
        matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team!=EDM"),
        "team!=EDM should match — current stint is DAL, not EDM"
    );
}

/// `team IN (DAL, BOS, NYR)` matches because DAL is the current
/// stint and is in the set.
#[test]
fn w15_team_in_set_with_current_stint_matches() {
    let repo = synthetic_traded_player_repo();
    assert!(
        matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team IN (DAL, BOS, NYR)"),
        "team IN (...) should match when current stint is in the set"
    );
}

#[test]
fn w15_team_in_set_without_current_stint_excludes() {
    let repo = synthetic_traded_player_repo();
    assert!(
        !matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team IN (BOS, NYR)"),
        "team IN (...) should NOT match when current stint isn't in the set"
    );
}

#[test]
fn w15_team_any_in_set_with_old_stint_matches() {
    let repo = synthetic_traded_player_repo();
    assert!(
        matches_filter(&repo, TRADED_PID, TRADED_SEASON, "team.any IN (EDM, BOS)"),
        "team.any IN (EDM, ...) should match the OLD stint"
    );
}

/// Compound: traded player with bio constraint.
#[test]
fn w15_traded_player_compound_with_bio() {
    let repo = synthetic_traded_player_repo();
    // Player is age ~26 in 2025-26 (born 2000-01-15, season-end
    // 2026 → 26 since pre-Feb-1).
    assert!(matches_filter(
        &repo,
        TRADED_PID,
        TRADED_SEASON,
        "team=DAL AND age<=27"
    ));
    assert!(matches_filter(
        &repo,
        TRADED_PID,
        TRADED_SEASON,
        "team.any=EDM AND country=CAN"
    ));
    // EDM (old) AND DAL (current) can't both be CURRENT —
    // logically AND should fail because team= is current-stint
    // single-valued.
    assert!(!matches_filter(
        &repo,
        TRADED_PID,
        TRADED_SEASON,
        "team=EDM AND team=DAL"
    ));
}

#[test]
fn w15_team_any_old_or_current() {
    let repo = synthetic_traded_player_repo();
    // team.any=EDM OR team.any=DAL — should match either way.
    assert!(matches_filter(
        &repo,
        TRADED_PID,
        TRADED_SEASON,
        "team.any=EDM OR team.any=DAL"
    ));
    // NOT team.any=EDM should be FALSE (he played EDM).
    assert!(!matches_filter(
        &repo,
        TRADED_PID,
        TRADED_SEASON,
        "NOT team.any=EDM"
    ));
}

// ── Property-based monotonicity over real data ────────────────

const SAMPLE_PIDS: &[(u32, &str)] = &[
    (8478402, "Connor McDavid"),
    (8471675, "Sidney Crosby"),
    (8471214, "Alex Ovechkin"),
    (8477492, "Nathan MacKinnon"),
    (8479318, "Auston Matthews"),
    (8484144, "Connor Bedard"),
    (8477956, "David Pastrnak"),
    (8480069, "Cale Makar"),
    (8481559, "Jack Hughes"),
    (8480800, "Quinn Hughes"),
    (8478864, "Kirill Kaprizov"),
    (8473419, "Brad Marchand"),
];

fn build_real_repo() -> StatsRepository {
    let mut repo = StatsRepository::with_lru_cap(80);
    for (pid, _) in SAMPLE_PIDS {
        let _ = load_player_career_into_repo(&mut repo, PlayerId(*pid));
    }
    repo
}

const REAL_SAMPLE_SEASONS: &[u32] = &[20252026, 20242025, 20232024];

fn matched_names_real(repo: &StatsRepository, filter: &str) -> HashSet<String> {
    let plan = parse_query(FilterInput::Cli(filter.into())).unwrap();
    let provider = NoOpProvider;
    let mut out = HashSet::new();
    for (pid, name) in SAMPLE_PIDS {
        for s in REAL_SAMPLE_SEASONS {
            let view = repo.view(PlayerId(*pid), Season(*s), SeasonType::Regular);
            if let Some(view) = view {
                let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), *s);
                if plan.root.matches(&view, &ctx) {
                    out.insert((*name).to_string());
                }
                break; // first season with data
            }
        }
    }
    out
}

/// **Property: tightening a constraint never increases the
/// result set.** If filter A is a subset of filter B, then
/// |A| ≤ |B|. Verify by adding an extra AND clause and
/// confirming the result is a subset.
#[test]
fn w15_tightening_monotonic_age() {
    let repo = build_real_repo();
    let loose = matched_names_real(&repo, "country=CAN");
    let tight = matched_names_real(&repo, "country=CAN AND age<25");
    for name in &tight {
        assert!(
            loose.contains(name),
            "tightening violated: {name} in tight={tight:?} but not loose={loose:?}"
        );
    }
}

#[test]
fn w15_tightening_monotonic_pos() {
    let repo = build_real_repo();
    let loose = matched_names_real(&repo, "country=CAN");
    let tight = matched_names_real(&repo, "country=CAN AND pos=C");
    for name in &tight {
        assert!(loose.contains(name), "{name} should be in {loose:?}");
    }
}

/// **Property: A AND B is the intersection of A and B.**
#[test]
fn w15_and_is_intersection() {
    let repo = build_real_repo();
    let a = matched_names_real(&repo, "country=CAN");
    let b = matched_names_real(&repo, "pos=C");
    let intersection: HashSet<String> = a.intersection(&b).cloned().collect();
    let and_query = matched_names_real(&repo, "country=CAN AND pos=C");
    assert_eq!(and_query, intersection);
}

/// **Property: A OR B is the union of A and B.**
#[test]
fn w15_or_is_union() {
    let repo = build_real_repo();
    let a = matched_names_real(&repo, "country=CAN");
    let b = matched_names_real(&repo, "country=USA");
    let union: HashSet<String> = a.union(&b).cloned().collect();
    let or_query = matched_names_real(&repo, "country=CAN OR country=USA");
    assert_eq!(or_query, union);
}

/// **Property: De Morgan's law over real data.** NOT (A AND B)
/// === NOT A OR NOT B.
#[test]
fn w15_demorgan_and_to_or_real_data() {
    let repo = build_real_repo();
    let lhs = matched_names_real(&repo, "NOT (country=CAN AND pos=C)");
    let rhs = matched_names_real(&repo, "NOT country=CAN OR NOT pos=C");
    assert_eq!(lhs, rhs);
}

#[test]
fn w15_demorgan_or_to_and_real_data() {
    let repo = build_real_repo();
    let lhs = matched_names_real(&repo, "NOT (country=CAN OR country=USA)");
    let rhs = matched_names_real(&repo, "NOT country=CAN AND NOT country=USA");
    assert_eq!(lhs, rhs);
}

/// **Property: double-NOT cancels.** NOT NOT X === X.
#[test]
fn w15_double_negation_cancels_real_data() {
    let repo = build_real_repo();
    let single = matched_names_real(&repo, "country=CAN");
    let double = matched_names_real(&repo, "NOT NOT country=CAN");
    assert_eq!(single, double);
}

/// **Property: triple-NOT == single-NOT.**
#[test]
fn w15_triple_negation_equals_single() {
    let repo = build_real_repo();
    let single = matched_names_real(&repo, "NOT country=CAN");
    let triple = matched_names_real(&repo, "NOT NOT NOT country=CAN");
    assert_eq!(single, triple);
}

/// **Property: A AND TRUE = A.** A trivially-true atom (g>=0)
/// AND'd with any predicate is equivalent to that predicate.
#[test]
fn w15_and_with_universal_is_identity() {
    let repo = build_real_repo();
    let plain = matched_names_real(&repo, "country=CAN");
    let with_universal = matched_names_real(&repo, "g>=0 AND country=CAN");
    assert_eq!(plain, with_universal);
}

/// **Property: BETWEEN x AND y === x<=key<=y.**
#[test]
fn w15_between_equals_le_ge_compound() {
    let repo = build_real_repo();
    let between = matched_names_real(&repo, "age BETWEEN 25 AND 35");
    let compound = matched_names_real(&repo, "age>=25 AND age<=35");
    assert_eq!(between, compound);
}

/// **Property: IN with one element === Eq.**
#[test]
fn w15_in_singleton_equals_eq() {
    let repo = build_real_repo();
    let in_one = matched_names_real(&repo, "country IN (CAN)");
    let eq = matched_names_real(&repo, "country=CAN");
    assert_eq!(in_one, eq);
}

/// **Property: complement = universe \ set.** NOT A combined
/// with A = empty intersection; NOT A union A = universe.
#[test]
fn w15_complement_relationships() {
    let repo = build_real_repo();
    let a = matched_names_real(&repo, "country=CAN");
    let not_a = matched_names_real(&repo, "NOT country=CAN");
    let intersection: HashSet<String> = a.intersection(&not_a).cloned().collect();
    assert!(
        intersection.is_empty(),
        "A ∩ ¬A must be empty; got: {intersection:?}"
    );
}

/// **Property: paren grouping doesn't change semantics.**
#[test]
fn w15_paren_grouping_idempotent() {
    let repo = build_real_repo();
    let bare = matched_names_real(&repo, "country=CAN AND pos=C");
    let parens = matched_names_real(&repo, "(country=CAN) AND (pos=C)");
    let nested = matched_names_real(&repo, "((country=CAN AND pos=C))");
    assert_eq!(bare, parens);
    assert_eq!(bare, nested);
}

/// **Property: case-insensitive boolean keywords.**
#[test]
fn w15_case_insensitive_booleans() {
    let repo = build_real_repo();
    let upper = matched_names_real(&repo, "country=CAN AND pos=C");
    let lower = matched_names_real(&repo, "country=CAN and pos=C");
    let mixed = matched_names_real(&repo, "country=CAN And pos=C");
    assert_eq!(upper, lower);
    assert_eq!(upper, mixed);
}

/// **Property: ordering doesn't change semantics for AND.**
#[test]
fn w15_and_commutative_real_data() {
    let repo = build_real_repo();
    let a_b = matched_names_real(&repo, "country=CAN AND pos=C");
    let b_a = matched_names_real(&repo, "pos=C AND country=CAN");
    assert_eq!(a_b, b_a);
}

/// **Property: ordering doesn't change semantics for OR.**
#[test]
fn w15_or_commutative_real_data() {
    let repo = build_real_repo();
    let a_b = matched_names_real(&repo, "country=CAN OR country=USA");
    let b_a = matched_names_real(&repo, "country=USA OR country=CAN");
    assert_eq!(a_b, b_a);
}
