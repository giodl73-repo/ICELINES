//! Persona Wave 21 — cross-surface result parity. Same filter
//! against the same repo, run via the web `/api/v1/leaders`
//! API AND directly via the icelines-query library, must
//! return the same matched player set.
//!
//! Catches divergence between web's bio-extraction +
//! filter_expr + new_plans evaluation order vs the library's
//! pure `Constraint::matches` over the same data.

use std::collections::HashSet;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::StatsRepository;
use icelines_fetch::stats_loader::load_player_career_into_repo;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{parse_query, FilterInput, StrictMode};
use icelines_web::{router, WebState};
use tower::util::ServiceExt;

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

fn build_repo() -> StatsRepository {
    let mut repo = StatsRepository::with_lru_cap(80);
    for (pid, _) in SAMPLE_PIDS {
        let _ = load_player_career_into_repo(&mut repo, PlayerId(*pid));
    }
    repo
}

fn fixed_today() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

const SAMPLE_SEASONS: &[u32] = &[20252026, 20242025, 20232024];

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

/// Run a filter via the icelines-query LIBRARY directly. Returns
/// the set of matched player names from the sample.
fn library_match(repo: &StatsRepository, filter: &str) -> HashSet<String> {
    let plan = parse_query(FilterInput::Cli(filter.into())).unwrap();
    let provider = NoOpProvider;

    let mut out = HashSet::new();
    for (pid, name) in SAMPLE_PIDS {
        for s in SAMPLE_SEASONS {
            if let Some(view) =
                repo.view(PlayerId(*pid), Season(*s), SeasonType::Regular)
            {
                let ctx =
                    EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), *s);
                if plan.root.matches(&view, &ctx) {
                    out.insert((*name).to_string());
                }
                break;
            }
        }
    }
    out
}

/// Run a filter via the web API. Builds a router pointed at the
/// SAME repo (via `WebState::with_repo`) so we can compare apples-
/// to-apples.
async fn web_match(repo: StatsRepository, filter: &str) -> HashSet<String> {
    let state = WebState::with_repo(repo);
    let app = router(state);
    let url = format!("/api/v1/leaders?filter={}&top=500", urlencode(filter));
    let response = app
        .oneshot(
            Request::builder()
                .uri(url)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    if response.status() != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        panic!("web /api/v1/leaders returned non-200 for {filter:?}: body:\n{body}");
    }

    let body = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .expect("body");
    let v: serde_json::Value =
        serde_json::from_slice(&body).expect("API response must be valid JSON");

    // Find the data array (envelope shape: data.rows or top-level array).
    let arr = v
        .get("data")
        .and_then(|d| d.get("rows"))
        .and_then(|r| r.as_array())
        .or_else(|| v.get("data").and_then(|d| d.as_array()))
        .or_else(|| v.as_array())
        .unwrap_or_else(|| panic!("unrecognized API response shape: {v}"))
        .clone();

    let mut out = HashSet::new();
    for row in arr {
        if let Some(name) = row.get("name").and_then(|n| n.as_str()) {
            // Only count names from our sample set (the API may
            // return many other players whose data is in the
            // sample-loaded repo).
            for (_, sample_name) in SAMPLE_PIDS {
                if name == *sample_name {
                    out.insert((*sample_name).to_string());
                }
            }
        }
    }
    out
}

fn urlencode(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('"', "%22")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace(',', "%2C")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('!', "%21")
        .replace('&', "%26")
        .replace('+', "%2B")
}

async fn assert_parity(filter: &str) {
    let repo = build_repo();
    let lib_set = library_match(&repo, filter);
    let web_set = web_match(repo, filter).await;

    let only_lib: HashSet<&String> = lib_set.difference(&web_set).collect();
    let only_web: HashSet<&String> = web_set.difference(&lib_set).collect();

    if !only_lib.is_empty() || !only_web.is_empty() {
        panic!(
            "parity violation for {filter:?}:\n  only library: {only_lib:?}\n  only web: {only_web:?}\n  library: {lib_set:?}\n  web: {web_set:?}"
        );
    }
}

// ── Bio atom parity ──────────────────────────────────────────

#[tokio::test]
async fn p_w21_001_country_eq_parity() {
    assert_parity("country=CAN").await;
}

#[tokio::test]
async fn p_w21_002_country_in_set_parity() {
    assert_parity("country IN (CAN, USA)").await;
}

#[tokio::test]
async fn p_w21_003_country_not_in_parity() {
    assert_parity("country NOT IN (RUS)").await;
}

#[tokio::test]
async fn p_w21_004_age_strict_lt_parity() {
    assert_parity("age<25").await;
}

#[tokio::test]
async fn p_w21_005_age_between_parity() {
    assert_parity("age BETWEEN 22 AND 32").await;
}

#[tokio::test]
async fn p_w21_006_country_like_parity() {
    assert_parity(r#"country LIKE "CA*""#).await;
}

// ── Position + draft parity ──────────────────────────────────

#[tokio::test]
async fn p_w21_007_pos_in_set_parity() {
    assert_parity("pos IN (C, LW, RW)").await;
}

#[tokio::test]
async fn p_w21_008_first_round_parity() {
    assert_parity("draft-round=1").await;
}

#[tokio::test]
async fn p_w21_009_draft_year_2015_parity() {
    assert_parity("draft-year=2015").await;
}

#[tokio::test]
async fn p_w21_010_top10_overall_parity() {
    assert_parity("draft-overall<=10").await;
}

// ── Compound parity ──────────────────────────────────────────

#[tokio::test]
async fn p_w21_011_canadian_centers_parity() {
    assert_parity("country=CAN AND pos=C").await;
}

#[tokio::test]
async fn p_w21_012_under_25_canadian_parity() {
    assert_parity("country=CAN AND age<25").await;
}

#[tokio::test]
async fn p_w21_013_canadians_in_set_parity() {
    assert_parity("country IN (CAN, USA) AND pos=C").await;
}

#[tokio::test]
async fn p_w21_014_age_range_canadian_parity() {
    assert_parity("country=CAN AND age BETWEEN 25 AND 35").await;
}

#[tokio::test]
async fn p_w21_015_demorgan_parity() {
    assert_parity("NOT (country=CAN AND pos=C)").await;
}

#[tokio::test]
async fn p_w21_016_or_parity() {
    assert_parity("country=CAN OR country=USA").await;
}

#[tokio::test]
async fn p_w21_017_paren_grouping_parity() {
    assert_parity("(country=CAN OR country=USA) AND pos=C").await;
}

// ── Strict comparator boundary parity ────────────────────────

#[tokio::test]
async fn p_w21_018_strict_lt_age_30_parity() {
    assert_parity("age<30").await;
}

#[tokio::test]
async fn p_w21_019_strict_le_age_30_parity() {
    assert_parity("age<=30").await;
}

#[tokio::test]
async fn p_w21_020_age_ne_parity() {
    assert_parity("age!=20").await;
}

// ── Negation parity ──────────────────────────────────────────

#[tokio::test]
async fn p_w21_021_negation_country_parity() {
    assert_parity("NOT country=CAN").await;
}

#[tokio::test]
async fn p_w21_022_double_negation_parity() {
    assert_parity("NOT NOT country=CAN").await;
}

// ── Triple-compound real-world parity ────────────────────────

#[tokio::test]
async fn p_w21_023_north_american_under_30_parity() {
    assert_parity("country IN (CAN, USA) AND pos=C AND age<30").await;
}

#[tokio::test]
async fn p_w21_024_top_picks_under_25_parity() {
    assert_parity("draft-overall<=10 AND age<25").await;
}

#[tokio::test]
async fn p_w21_025_kitchen_sink_parity() {
    assert_parity("country IN (CAN, USA) AND pos IN (C, LW, RW) AND age BETWEEN 25 AND 32").await;
}
