use crate::state::WebState;
use crate::templates::{TransactionRow, TransactionsTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::transactions::{TransactionKind, TRANSACTIONS_EARLIEST_SEASON};
use serde::Deserialize;

/// Query params accepted by `/transactions`.
#[derive(Debug, Deserialize, Default)]
pub struct TransactionsQuery {
    /// Filter by kind: `trade`, `signing`, `recall`,
    /// `reassignment`, `waiver` (expands to all 3 waiver kinds),
    /// `ir`, `other`. Unknown input returns 400.
    #[serde(default)]
    pub kind: Option<String>,
    /// Filter by team abbreviation (case-insensitive).
    #[serde(default)]
    pub team: Option<String>,
}

struct TransactionsResult {
    active_label: String,
    season_pretty: String,
    rows: Vec<TransactionRow>,
    total: usize,
    empty_unfiltered: bool,
    active_kind: String,
    active_team: String,
    out_of_coverage: bool,
    earliest_season_pretty: String,
}

#[derive(Debug, serde::Serialize)]
struct TransactionsEnvelope {
    schema_version: u32,
    route: &'static str,
    data: Vec<TransactionRow>,
    meta: TransactionsMeta,
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct TransactionsMeta {
    season: String,
    total: usize,
    active_kind: String,
    active_team: String,
    empty_unfiltered: bool,
    out_of_coverage: bool,
    earliest_season: String,
}

fn pretty_season(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}", &s[0..4], &s[6..8])
    } else {
        s.to_owned()
    }
}

/// Pretty-cased label per kind, for the chip column. Matches the
/// CLI's display style ("Waiver claim" not "waiver_claim").
fn pretty_kind(k: TransactionKind) -> &'static str {
    match k {
        TransactionKind::Trade => "Trade",
        TransactionKind::WaiverPlacement => "Waivers",
        TransactionKind::WaiverClear => "Waivers",
        TransactionKind::WaiverClaim => "Waiver claim",
        TransactionKind::Signing => "Signing",
        TransactionKind::Recall => "Recall",
        TransactionKind::Reassignment => "Reassignment",
        TransactionKind::InjuryReserve => "IR",
        TransactionKind::Other => "Other",
    }
}

pub async fn get_transactions(
    State(state): State<WebState>,
    Query(q): Query<TransactionsQuery>,
) -> Response {
    let result = match build_transactions_result(&state, &q).await {
        Ok(result) => result,
        Err(msg) => {
            return (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    "<!doctype html><html><body>\
                             <h1>Bad filter</h1><p>{msg}</p>\
                             <p><a href=\"/transactions\">back to transactions</a></p>\
                             </body></html>",
                )),
            )
                .into_response();
        }
    };

    let tmpl = TransactionsTemplate {
        active_label: result.active_label,
        season_pretty: result.season_pretty,
        rows: result.rows,
        total: result.total,
        empty_unfiltered: result.empty_unfiltered,
        active_kind: result.active_kind,
        active_team: result.active_team,
        out_of_coverage: result.out_of_coverage,
        earliest_season_pretty: result.earliest_season_pretty,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<!doctype html><html><body><h1>500</h1>\
                         <p>template render failed: {e}</p></body></html>"
            )),
        )
            .into_response(),
    }
}

pub async fn get_transactions_json(
    State(state): State<WebState>,
    Query(q): Query<TransactionsQuery>,
) -> Response {
    match build_transactions_result(&state, &q).await {
        Ok(result) => axum::Json(TransactionsEnvelope {
            schema_version: 1,
            route: "transactions",
            meta: TransactionsMeta {
                season: result.season_pretty,
                total: result.total,
                active_kind: result.active_kind,
                active_team: result.active_team,
                empty_unfiltered: result.empty_unfiltered,
                out_of_coverage: result.out_of_coverage,
                earliest_season: result.earliest_season_pretty,
            },
            data: result.rows,
            error: None,
        })
        .into_response(),
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            axum::Json(TransactionsEnvelope {
                schema_version: 1,
                route: "transactions",
                data: Vec::new(),
                meta: TransactionsMeta {
                    season: String::new(),
                    total: 0,
                    active_kind: q.kind.unwrap_or_default(),
                    active_team: q.team.unwrap_or_default(),
                    empty_unfiltered: true,
                    out_of_coverage: false,
                    earliest_season: pretty_season(TRANSACTIONS_EARLIEST_SEASON),
                },
                error: Some(msg),
            }),
        )
            .into_response(),
    }
}

async fn build_transactions_result(
    state: &WebState,
    q: &TransactionsQuery,
) -> Result<TransactionsResult, String> {
    let (season_str, active_label) = {
        let cfg = state.config.read().await;
        (cfg.active_season.clone(), cfg.active_label.clone())
    };

    let kind_filter: Option<Vec<TransactionKind>> = match q.kind.as_deref() {
        None | Some("") => None,
        Some(k) => Some(TransactionKind::parse_filter(k)?),
    };
    let team_filter = q
        .team
        .as_deref()
        .map(|t| t.trim().to_ascii_uppercase())
        .filter(|t| !t.is_empty());
    let active_kind = q.kind.clone().unwrap_or_default();
    let active_team = team_filter.clone().unwrap_or_default();
    let out_of_coverage = season_str.as_str() < TRANSACTIONS_EARLIEST_SEASON;

    let snapshots_root = match state.snapshots_root.as_ref() {
        Some(p) => p.clone(),
        None => icelines_fetch::snapshot::SnapshotStore::default_root(),
    };
    let store = icelines_fetch::snapshot::SnapshotStore::new(snapshots_root);

    let envelope_result = if out_of_coverage {
        Err(())
    } else {
        icelines_fetch::bundled::load_transactions_with_fallback(&season_str, &store)
            .map_err(|_| ())
    };

    let mut rows: Vec<TransactionRow> = match envelope_result {
        Ok(env) => env
            .rows
            .into_iter()
            .filter(|t| match &kind_filter {
                None => true,
                Some(kinds) => kinds.contains(&t.kind),
            })
            .filter(|t| match &team_filter {
                None => true,
                Some(team) => t
                    .team
                    .as_ref()
                    .map(|a| a.as_str().eq_ignore_ascii_case(team))
                    .unwrap_or(false),
            })
            .map(|t| TransactionRow {
                date: t.date,
                team: t.team.map(|a| a.as_str().to_owned()).unwrap_or_default(),
                kind_label: t.kind.label().to_owned(),
                kind_pretty: pretty_kind(t.kind).to_owned(),
                description: t.description,
            })
            .collect(),
        Err(()) => Vec::new(),
    };

    rows.sort_by(|a, b| b.date.cmp(&a.date));
    rows.truncate(1000);
    let total = rows.len();
    let empty_unfiltered = rows.is_empty() && kind_filter.is_none() && team_filter.is_none();

    Ok(TransactionsResult {
        active_label,
        season_pretty: pretty_season(&season_str),
        rows,
        total,
        empty_unfiltered,
        active_kind,
        active_team,
        out_of_coverage,
        earliest_season_pretty: pretty_season(TRANSACTIONS_EARLIEST_SEASON),
    })
}
