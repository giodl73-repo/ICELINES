//! Phase Art Ross — Wave 23 L1 integration tests for the TUI
//! filter overlay's evaluation pipeline.
//!
//! The TUI overlay's `run_query_views_with_pick_and_plan` helper
//! does two things in sequence:
//!   1. Apply the legacy `PlayerFilter` (sourced from the structured
//!      field editor — Position, Age, Nationality, etc.).
//!   2. Apply the new `Constraint::matches` (sourced from the
//!      free-form filter overlay's `parse_query` output).
//!
//! These tests reproduce that exact sequence end-to-end against a
//! real `StatsRepository` populated from bundled data — same data
//! the TUI sees when launched. Any regression in either stage
//! shows up here as a behavioral mismatch on the assertion list,
//! mirroring what the user would see on the Queries screen after
//! pressing `f`.
//!
//! These tests intentionally do NOT depend on the `icelines_cli`
//! library facade — the binary's `tui` module is private. We
//! reproduce the helper logic locally so the integration boundary
//! we test is the public icelines-query / icelines-core / icelines-
//! fetch surface, which is the contract that matters.

use icelines_core::filter::PlayerFilter;
use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::{PlayerView, StatsRepository};
use icelines_fetch::stats_loader::load_player_career_into_repo;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{parse_query, FilterInput, QueryPlan, StrictMode};

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

const TARGET_SEASONS: &[u32] = &[20252026, 20242025, 20232024];

fn build_repo() -> StatsRepository {
    let mut repo = StatsRepository::with_lru_cap(80);
    for (pid, _) in SAMPLE_PIDS {
        let _ = load_player_career_into_repo(&mut repo, PlayerId(*pid));
    }
    repo
}

fn build_views<'a>(repo: &'a StatsRepository) -> Vec<PlayerView<'a>> {
    let mut out = Vec::new();
    for (pid, _) in SAMPLE_PIDS {
        for s in TARGET_SEASONS {
            if let Some(view) = repo.view(PlayerId(*pid), Season(*s), SeasonType::Regular) {
                out.push(view);
                break;
            }
        }
    }
    out
}

fn fixed_today() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

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

fn parse_plan(filter: &str) -> QueryPlan {
    parse_query(FilterInput::Cli(filter.into()))
        .unwrap_or_else(|errs| panic!("test filter {filter:?} failed: {errs:?}"))
}

/// Mirrors `run_query_views_with_pick_and_plan` (post-legacy-filter,
/// post-plan-filter). The TUI helper additionally sorts + truncates;
/// for these correctness tests we only care about the matched set.
fn evaluate<'a>(
    views: &'a [PlayerView<'a>],
    legacy: &PlayerFilter,
    plan: Option<&QueryPlan>,
    season: u32,
) -> Vec<PlayerView<'a>> {
    let mut matched: Vec<PlayerView<'a>> = views
        .iter()
        .cloned()
        .filter(|v| legacy.matches_view(v))
        .collect();
    if let Some(plan) = plan {
        let provider = NoOpProvider;
        let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), season);
        matched.retain(|v| plan.root.matches(v, &ctx));
    }
    matched
}

fn names(views: &[PlayerView<'_>]) -> Vec<String> {
    views
        .iter()
        .map(|v| v.identity.full_name.clone())
        .collect()
}

// ── Bio plan ────────────────────────────────────────────────────

#[test]
fn l1_w23_country_eq_filters_to_canadians() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("country=CAN");
    let result = evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026);
    let names = names(&result);

    assert!(
        names.iter().any(|n| n == "Connor McDavid"),
        "country=CAN must include McDavid; got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "country=CAN must exclude Ovechkin (RUS); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Kirill Kaprizov"),
        "country=CAN must exclude Kaprizov (RUS); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "David Pastrnak"),
        "country=CAN must exclude Pastrnak (CZE); got: {names:?}"
    );
}

#[test]
fn l1_w23_country_in_set() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("country IN (CAN, USA)");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(names.iter().any(|n| n == "Connor McDavid"), "{names:?}");
    assert!(names.iter().any(|n| n == "Auston Matthews"), "{names:?}");
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "RUS must be excluded; got: {names:?}"
    );
}

#[test]
fn l1_w23_country_not_in_excludes_russians() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("country NOT IN (RUS)");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "NOT IN (RUS) must exclude Ovechkin; got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Kirill Kaprizov"),
        "NOT IN (RUS) must exclude Kaprizov; got: {names:?}"
    );
    assert!(!names.is_empty());
}

// ── Numeric plan ────────────────────────────────────────────────

#[test]
fn l1_w23_age_strict_lt_25() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("age<25");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(
        names.iter().any(|n| n == "Connor Bedard"),
        "age<25 must include Bedard (b. 2005); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Sidney Crosby"),
        "age<25 must exclude Crosby (b. 1987); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "age<25 must exclude Ovechkin (b. 1985); got: {names:?}"
    );
}

#[test]
fn l1_w23_age_between_22_28() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("age BETWEEN 22 AND 28");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(
        names.iter().all(|n| n != "Sidney Crosby"),
        "BETWEEN 22..28 must exclude Crosby (over 28); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "BETWEEN 22..28 must exclude Ovechkin (over 28); got: {names:?}"
    );
}

// ── Compound + boolean ──────────────────────────────────────────

#[test]
fn l1_w23_compound_canadian_under_25() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("country=CAN AND age<25");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(
        names.iter().any(|n| n == "Connor Bedard"),
        "Canadian + under 25 must include Bedard; got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Sidney Crosby"),
        "compound must exclude Crosby (over 25); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "compound must exclude Ovechkin (RUS); got: {names:?}"
    );
}

#[test]
fn l1_w23_or_canadians_or_americans() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("country=CAN OR country=USA");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(names.iter().any(|n| n == "Connor McDavid"));
    assert!(names.iter().any(|n| n == "Auston Matthews"));
    assert!(
        names.iter().all(|n| n != "Alex Ovechkin"),
        "OR-form must exclude RUS; got: {names:?}"
    );
}

#[test]
fn l1_w23_double_negation_equals_positive() {
    let repo = build_repo();
    let views = build_views(&repo);
    let positive = names(&evaluate(
        &views,
        &PlayerFilter::new(),
        Some(&parse_plan("country=CAN")),
        20252026,
    ));
    let double = names(&evaluate(
        &views,
        &PlayerFilter::new(),
        Some(&parse_plan("NOT NOT country=CAN")),
        20252026,
    ));

    let mut a = positive;
    let mut b = double;
    a.sort();
    b.sort();
    assert_eq!(a, b, "NOT NOT X must equal X");
}

// ── None plan = identity ────────────────────────────────────────

#[test]
fn l1_w23_none_plan_returns_full_unfiltered_set() {
    let repo = build_repo();
    let views = build_views(&repo);
    let result = evaluate(&views, &PlayerFilter::new(), None, 20252026);
    assert!(
        !result.is_empty(),
        "None plan + default fields must NOT filter the sample to empty"
    );
}

// ── Impossible compound = empty ─────────────────────────────────

#[test]
fn l1_w23_impossible_compound_empty() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("country=CAN AND country=RUS");
    let result = evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026);
    assert!(
        result.is_empty(),
        "country=CAN AND country=RUS must be empty; got: {:?}",
        names(&result)
    );
}

// ── Position plan ───────────────────────────────────────────────

#[test]
fn l1_w23_pos_in_excludes_defensemen() {
    let repo = build_repo();
    let views = build_views(&repo);
    let plan = parse_plan("pos IN (C, LW, RW)");
    let names = names(&evaluate(&views, &PlayerFilter::new(), Some(&plan), 20252026));

    assert!(
        names.iter().all(|n| n != "Cale Makar"),
        "pos IN (C, LW, RW) must exclude Makar (D); got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Quinn Hughes"),
        "pos IN (C, LW, RW) must exclude Quinn Hughes (D); got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Connor McDavid"),
        "pos IN (C, LW, RW) must include McDavid (C); got: {names:?}"
    );
}

// ── Plan ∩ legacy field filter ──────────────────────────────────

#[test]
fn l1_w23_plan_intersects_legacy_field_filter() {
    let repo = build_repo();
    let views = build_views(&repo);

    // Legacy field stage: defensemen only.
    let mut legacy = PlayerFilter::new();
    legacy.positions = Some(vec![Position::Defense]);

    // Plan stage: Canadians only.
    let plan = parse_plan("country=CAN");

    let names = names(&evaluate(&views, &legacy, Some(&plan), 20252026));

    assert!(
        names.iter().any(|n| n == "Cale Makar"),
        "legacy=D ∩ plan=country=CAN must include Makar; got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Quinn Hughes"),
        "Quinn Hughes is USA, plan rejects; got: {names:?}"
    );
    assert!(
        names.iter().all(|n| n != "Connor McDavid"),
        "McDavid is C, legacy rejects; got: {names:?}"
    );
}
