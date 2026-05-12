//! Phase Art Ross A.0.8 — Cross-pipeline parity test.
//!
//! Validates that the new pipeline (`parse_query` →
//! `Constraint::matches`) produces the same per-player boolean
//! match as the legacy pipeline (`parse_filter_expr` →
//! `FilterExpr::matches`) for a corpus of filter strings.
//!
//! Runs against bundled current-season data — no network, no
//! tempdir. The shared corpus is a curated subset of Wave 11's
//! 201 scenarios chosen to exercise every legacy code path that
//! the new pipeline must reproduce identically:
//!
//! - Simple atoms with each op (>=, <=, ==, =)
//! - Boolean compositions (AND, OR, NOT, parens)
//! - Complex chains (3+ atoms)
//! - Mixed precedence (AND-binds-tighter-than-OR)
//! - Edge cases (NOT NOT cancellation, paren grouping)
//!
//! Bio atoms are NOT in this corpus — they're handled by a
//! separate adapter layer (`try_parse_single_bio_constraint` +
//! the existing `extract_bio` extraction) that ships in A.1.

use icelines_core::identity::PlayerId;
use icelines_core::stats_catalog::parse_filter_expr;
use icelines_core::stats_repository::StatsRepository;
use icelines_fetch::stats_loader::load_player_career_into_repo;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{parse_query, FilterInput, StrictMode};

/// No-op DataProvider for the A.0 parity test corpus — the corpus
/// has no SlidingWindow atoms, so the provider is never invoked.
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

/// A representative active player (Connor McDavid). Loading his
/// career fans out across all bundled seasons; the current season
/// row is what we exercise.
const SAMPLE_PIDS: &[u32] = &[
    8478402, // McDavid
    8471675, // Crosby
    8471214, // Ovechkin
    8476453, // Kucherov
    8475158, // Bedard's class
    8483515, // Recent draftee
];

/// Pure-stat filter corpus. Every entry must produce the same
/// per-player boolean across the legacy and new pipelines.
const FILTER_CORPUS: &[&str] = &[
    // Section A — simple atoms, each op
    "g>=10",
    "g<=100",
    "g==0",
    "g=5",
    "p>=20",
    "a>=15",
    "pim>=0",
    "shots>=50",
    "shooting-pct>=0.10",
    // Section B — boolean compositions
    "g>=10 AND a>=10",
    "g>=10 OR a>=10",
    "NOT g>=100",
    "NOT NOT g>=10",
    "g>=10 AND a>=10 AND p>=20",
    "g>=10 OR a>=10 OR p>=20",
    "(g>=10 OR a>=10) AND p>=20",
    "g>=10 AND (a>=10 OR p>=20)",
    "NOT (g>=100 OR a>=100)",
    "NOT (g>=100 AND a>=100)",
    // Section C — precedence
    "g>=10 AND a>=10 OR p>=20",
    "g>=10 OR a>=10 AND p>=20",
    // Section D — bounds
    "g>=0",
    "g>=99999",
    "p<=10000",
    // Section E — short aliases (the legacy parser handles these)
    "p>=20",
    "ppg>=0.5",
    "+/->=-10",
    // Section F — case-insensitive booleans
    "g>=10 and a>=10",
    "g>=10 Or a>=10",
    "not g>=100",
    // Section G — whitespace tolerance
    "g >= 10",
    "  g>=10  AND  a>=10  ",
    "g>=10\tAND\ta>=10",
    // Section H — De Morgan equivalence (must be true for both)
    "NOT (g>=10 AND a>=10)",
    "NOT g>=10 OR NOT a>=10",
    "NOT (g>=10 OR a>=10)",
    "NOT g>=10 AND NOT a>=10",
];

fn current_season_u32() -> u32 {
    icelines_core::CURRENT_SEASON
}

/// Build a repo populated with sample players' careers. Uses the
/// existing `load_player_career_into_repo` (which fans across
/// bundled seasons via `SnapshotStore::default_root()`).
fn build_sample_repo() -> StatsRepository {
    let mut repo = StatsRepository::with_lru_cap(80);
    for pid in SAMPLE_PIDS {
        let _ = load_player_career_into_repo(&mut repo, PlayerId(*pid));
    }
    repo
}

fn fixed_today() -> chrono::NaiveDate {
    // Mid-season anchor — corpus has no calendar-window atoms, so
    // the exact date doesn't affect results, but it must be stable
    // across test runs.
    chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

/// For each filter in the corpus, parse via BOTH pipelines and
/// assert the per-player boolean agrees against every player view
/// in the active-season set.
#[test]
fn art_ross_a0_parity_corpus_agrees_on_every_player() {
    let repo = build_sample_repo();
    let season = icelines_core::model::Season(current_season_u32());
    let season_type = icelines_core::season_stats::SeasonType::Regular;

    let provider = NoOpProvider;
    let ctx = EvalCtx::new(
        &provider,
        StrictMode::Off,
        false,
        fixed_today(),
        current_season_u32(),
    );

    let mut diffs: Vec<String> = Vec::new();

    for &filter in FILTER_CORPUS {
        // Legacy pipeline: parse_filter_expr → FilterExpr::matches
        let legacy_expr = match parse_filter_expr(filter) {
            Ok(e) => e,
            Err(e) => {
                panic!("legacy parser failed on filter {filter:?} (curated corpus must parse): {e}")
            }
        };

        // New pipeline: parse_query → Constraint::matches (unified)
        let plan = match parse_query(FilterInput::Cli(filter.to_string())) {
            Ok(p) => p,
            Err(es) => panic!("new parser failed on filter {filter:?} (parity blocker): {es:?}"),
        };

        // Walk every player's active-season view. Compare match
        // booleans across pipelines.
        for pid in SAMPLE_PIDS {
            let view = match repo.view(PlayerId(*pid), season, season_type) {
                Some(v) => v,
                None => continue, // player not in active season
            };

            let legacy = legacy_expr.matches(&view);
            let new = plan.root.matches(&view, &ctx);

            if legacy != new {
                diffs.push(format!(
                    "filter {filter:?} pid={pid}: legacy={legacy} new={new}"
                ));
            }
        }
    }

    if !diffs.is_empty() {
        panic!(
            "A.0 parity violations ({} diffs):\n{}",
            diffs.len(),
            diffs.join("\n")
        );
    }
}

/// Sanity: the corpus has the diversity to actually exercise the
/// matcher (i.e. the booleans aren't trivially all-true or all-
/// false). If a filter is uniform across all players + the
/// pipeline disagreement test would silently pass on a no-op
/// match.
#[test]
fn art_ross_a0_corpus_has_actual_diversity() {
    let repo = build_sample_repo();
    let season = icelines_core::model::Season(current_season_u32());
    let season_type = icelines_core::season_stats::SeasonType::Regular;
    let provider = NoOpProvider;
    let ctx = EvalCtx::new(
        &provider,
        StrictMode::Off,
        false,
        fixed_today(),
        current_season_u32(),
    );

    let mut found_true = false;
    let mut found_false = false;

    for &filter in FILTER_CORPUS {
        let plan = parse_query(FilterInput::Cli(filter.to_string())).unwrap();
        for pid in SAMPLE_PIDS {
            if let Some(view) = repo.view(PlayerId(*pid), season, season_type) {
                if plan.root.matches(&view, &ctx) {
                    found_true = true;
                } else {
                    found_false = true;
                }
            }
        }
    }

    assert!(
        found_true && found_false,
        "corpus produced uniform results — parity test would silently pass; \
         add filter shapes that produce both true and false across the sample set"
    );
}

/// Verify the n-ary IR structure is exercised: at least one filter
/// in the corpus should produce an All with ≥3 children, and at
/// least one should produce an Any with ≥2 children. This pins the
/// IR shape contract from the 8-role review (R2).
#[test]
fn art_ross_a0_corpus_exercises_n_ary_ir() {
    let mut max_all = 0;
    let mut max_any = 0;

    for &filter in FILTER_CORPUS {
        let plan = parse_query(FilterInput::Cli(filter.to_string())).unwrap();
        match &plan.root {
            icelines_query::Constraint::All(children) if children.len() > max_all => {
                max_all = children.len();
            }
            icelines_query::Constraint::Any(children) if children.len() > max_any => {
                max_any = children.len();
            }
            _ => {}
        }
    }

    assert!(
        max_all >= 3,
        "corpus must include at least one All chain with ≥3 children to exercise the n-ary IR \
         shape (got max All-len={max_all})"
    );
    assert!(
        max_any >= 2,
        "corpus must include at least one Any chain with ≥2 children (got max Any-len={max_any})"
    );
}

/// IR roundtrip property — A.0 acceptance gate. Random
/// `Constraint` trees should serialize to canonical strings that
/// re-parse to identical trees. For A.0 we only round-trip
/// SeasonStat (Bio + sliding/career are A.1+). Manual rather than
/// proptest since the universe of legal strings is small enough
/// to enumerate.
#[test]
fn art_ross_a0_ir_roundtrip_for_corpus() {
    use icelines_query::Constraint;

    for &filter in FILTER_CORPUS {
        let plan = parse_query(FilterInput::Cli(filter.to_string())).unwrap();
        let serialized = serialize_to_canonical(&plan.root);
        let plan2 = parse_query(FilterInput::Cli(serialized.clone())).unwrap_or_else(|es| {
            panic!("round-trip {filter:?} → {serialized:?} re-parse failed: {es:?}")
        });
        assert_eq!(
            plan.root, plan2.root,
            "round-trip mismatch for {filter:?}: serialized={serialized:?}"
        );
    }

    fn serialize_to_canonical(c: &Constraint) -> String {
        use icelines_query::{Predicate, ScalarOp, ScalarValue};
        match c {
            Constraint::SeasonStat(s) => {
                let op = match &s.predicate {
                    Predicate::Scalar(ScalarOp::Ge, _) => ">=",
                    Predicate::Scalar(ScalarOp::Le, _) => "<=",
                    Predicate::Scalar(ScalarOp::Eq, _) => "==",
                    _ => unreachable!("A.0 corpus is Scalar-only"),
                };
                let val = match &s.predicate {
                    Predicate::Scalar(_, ScalarValue::Number(n)) => *n,
                    _ => unreachable!("A.0 corpus is Number-only"),
                };
                // Use the canonical cli_key + integer format when whole
                let key = s.stat.cli_key();
                if val.fract() == 0.0 && val.abs() < 1e15 {
                    format!("{key}{op}{}", val as i64)
                } else {
                    format!("{key}{op}{val}")
                }
            }
            Constraint::All(children) => {
                let pieces: Vec<String> = children.iter().map(serialize_to_canonical).collect();
                format!("({})", pieces.join(" AND "))
            }
            Constraint::Any(children) => {
                let pieces: Vec<String> = children.iter().map(serialize_to_canonical).collect();
                format!("({})", pieces.join(" OR "))
            }
            Constraint::Not(inner) => format!("NOT ({})", serialize_to_canonical(inner)),
            other => panic!("A.0 corpus shouldn't contain {other:?}"),
        }
    }
}
