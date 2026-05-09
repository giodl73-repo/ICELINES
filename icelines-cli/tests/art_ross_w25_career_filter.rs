//! Phase Art Ross — Wave 25 L1 integration tests for
//! `query career --filter`.
//!
//! `filter_cohort_with_plan` (private to `commands::query_career`)
//! does the actual narrowing; from outside the crate we can't call
//! it directly, so the L1 tier reproduces the same evaluator
//! sequence: load bundled bios into a `StatsRepository`, build a
//! `PlayerView`, run `Constraint::matches` against it. This proves
//! the same filter expressions the cli surface accepts evaluate
//! correctly against real bundled bios — the surface contract.
//!
//! The cohort fixture pids are real bundled NHL players; the
//! filter expressions exercise bio atoms that work on the
//! `query career` surface (country, pos, age@stint-year).

use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::{PlayerView, StatsRepository};
use icelines_fetch::stats_loader::load_player_career_into_repo;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{parse_query, FilterInput, QueryPlan, StrictMode};

/// Sample of currently-active players who definitely have bundled
/// NHL bios (filter eval needs a `PlayerView` with bio fields). The
/// test treats these pids as members of a synthetic OHL/WHL cohort.
const SAMPLE_PIDS: &[(u32, &str, &str)] = &[
    // (nhl_id, full_name, country)
    (8478402, "Connor McDavid", "CAN"),
    (8471675, "Sidney Crosby", "CAN"),
    (8479318, "Auston Matthews", "USA"),
    (8484144, "Connor Bedard", "CAN"),
    (8481559, "Jack Hughes", "USA"),
    (8480069, "Cale Makar", "CAN"),
    (8478864, "Kirill Kaprizov", "RUS"),
    (8477956, "David Pastrnak", "CZE"),
];

const BUNDLED_FALLBACK_SEASONS: &[u32] = &[20252026, 20242025, 20232024, 20222023, 20212022];

fn build_repo() -> StatsRepository {
    let mut repo = StatsRepository::with_lru_cap(80);
    for (pid, _, _) in SAMPLE_PIDS {
        let _ = load_player_career_into_repo(&mut repo, PlayerId(*pid));
    }
    repo
}

/// Look up the most-recent skater view for a pid — the same shape
/// `filter_cohort_with_plan` produces.
fn most_recent_view<'a>(repo: &'a StatsRepository, pid: u32) -> Option<PlayerView<'a>> {
    for s in BUNDLED_FALLBACK_SEASONS {
        if let Some(v) = repo.view(PlayerId(pid), Season(*s), SeasonType::Regular) {
            return Some(v);
        }
        if let Some(v) = repo.view(PlayerId(pid), Season(*s), SeasonType::Playoff) {
            return Some(v);
        }
    }
    None
}

struct NoOp;
impl DataProvider for NoOp {
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

/// Evaluate `plan` against each sample pid using `stint_season` as
/// the EvalCtx anchor (same path `filter_cohort_with_plan` takes).
/// Returns the names that pass.
fn evaluate(repo: &StatsRepository, plan: &QueryPlan, stint_season: u32) -> Vec<String> {
    let provider = NoOp;
    let mut out = Vec::new();
    for (pid, name, _) in SAMPLE_PIDS {
        let Some(view) = most_recent_view(repo, *pid) else {
            continue;
        };
        let ctx = EvalCtx::new(
            &provider,
            StrictMode::Off,
            false,
            fixed_today(),
            stint_season,
        );
        if plan.root.matches(&view, &ctx) {
            out.push((*name).to_owned());
        }
    }
    out
}

fn parse_plan(filter: &str) -> QueryPlan {
    parse_query(FilterInput::Cli(filter.into()))
        .unwrap_or_else(|errs| panic!("test filter {filter:?} failed to parse: {errs:?}"))
}

// ── Bio atoms work on cohort views ──────────────────────────────

#[test]
fn l1_w25_country_eq_filters_canadians() {
    let repo = build_repo();
    let names = evaluate(&repo, &parse_plan("country=CAN"), 20142015);
    assert!(
        names.iter().any(|n| n == "Connor McDavid"),
        "country=CAN must include McDavid; got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Auston Matthews"),
        "country=CAN must exclude Matthews (USA); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "country=CAN must exclude RUS players; got: {names:?}"
    );
}

#[test]
fn l1_w25_country_in_set() {
    let repo = build_repo();
    let names = evaluate(&repo, &parse_plan("country IN (CAN, USA)"), 20142015);
    assert!(names.iter().any(|n| n == "Connor McDavid"));
    assert!(names.iter().any(|n| n == "Auston Matthews"));
    assert!(
        names.iter().all(|n| n != "Kirill Kaprizov"),
        "RUS must be excluded; got: {names:?}"
    );
}

#[test]
fn l1_w25_pos_in_excludes_defensemen() {
    let repo = build_repo();
    let names = evaluate(&repo, &parse_plan("pos IN (C, LW, RW)"), 20142015);
    assert!(
        names.iter().all(|n| n != "Cale Makar"),
        "pos IN (C, LW, RW) must exclude Makar (D); got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Connor McDavid"),
        "must include McDavid (C); got: {names:?}"
    );
}

// ── Age atom anchors on the stint year ──────────────────────────

#[test]
fn l1_w25_age_at_stint_year_uses_anchor_season() {
    let repo = build_repo();
    // 2014-15 cohort: McDavid (b. 1997) was 17 → matches age<=18.
    let early = evaluate(&repo, &parse_plan("age<=18"), 20142015);
    assert!(
        early.iter().any(|n| n == "Connor McDavid"),
        "age<=18 anchored to 2014-15 must include McDavid; got: {early:?}"
    );
    // 2024-25 cohort: McDavid is 27 → must NOT match age<=18.
    let recent = evaluate(&repo, &parse_plan("age<=18"), 20242025);
    assert!(
        recent.iter().all(|n| n != "Connor McDavid"),
        "age<=18 anchored to 2024-25 must exclude McDavid; got: {recent:?}"
    );
}

// ── Compound bio + double negation ──────────────────────────────

#[test]
fn l1_w25_compound_canadian_centers_age_under_25() {
    let repo = build_repo();
    let names = evaluate(
        &repo,
        &parse_plan("country=CAN AND pos=C AND age<25"),
        20142015,
    );
    // Bedard: CAN, C, was 9 in 2014-15 → matches.
    assert!(
        names.iter().any(|n| n == "Connor Bedard"),
        "compound must include Bedard (CAN, C, age 9 in 2014-15); got: {names:?}"
    );
    // Crosby: CAN, C, was 27 in 2014-15 → too old, excluded.
    assert!(
        names.iter().all(|n| n != "Sidney Crosby"),
        "compound must exclude Crosby (over 25 in 2014-15); got: {names:?}"
    );
    // Matthews: USA, excluded by country.
    assert!(
        names.iter().all(|n| n != "Auston Matthews"),
        "compound must exclude Matthews (USA); got: {names:?}"
    );
}

#[test]
fn l1_w25_double_negation_equals_positive() {
    let repo = build_repo();
    let positive = evaluate(&repo, &parse_plan("country=CAN"), 20142015);
    let double = evaluate(&repo, &parse_plan("NOT NOT country=CAN"), 20142015);
    let mut a = positive;
    let mut b = double;
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

// ── Empty cohort intersection ───────────────────────────────────

#[test]
fn l1_w25_impossible_compound_empty() {
    let repo = build_repo();
    let names = evaluate(&repo, &parse_plan("country=CAN AND country=RUS"), 20142015);
    assert!(
        names.is_empty(),
        "country=CAN AND country=RUS must yield empty; got: {names:?}"
    );
}

// ── OR boolean ──────────────────────────────────────────────────

#[test]
fn l1_w25_or_canadians_or_americans() {
    let repo = build_repo();
    let names = evaluate(&repo, &parse_plan("country=CAN OR country=USA"), 20142015);
    assert!(names.iter().any(|n| n == "Connor McDavid"));
    assert!(names.iter().any(|n| n == "Auston Matthews"));
    assert!(
        names.iter().all(|n| n != "Kirill Kaprizov"),
        "RUS excluded from CAN OR USA; got: {names:?}"
    );
}
