use crate::state::WebState;
use crate::templates::{TransactionRow, TransactionsTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::transactions::{TransactionKind, TRANSACTIONS_EARLIEST_SEASON};
use icelines_core::{TransactionViewRow, TransactionsView, ViewContext, ViewWindow};
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
        Ok(result) => crate::api::json_data_meta(
            "transactions",
            result.rows,
            TransactionsMeta {
                season: result.season_pretty,
                total: result.total,
                active_kind: result.active_kind,
                active_team: result.active_team,
                empty_unfiltered: result.empty_unfiltered,
                out_of_coverage: result.out_of_coverage,
                earliest_season: result.earliest_season_pretty,
            },
        ),
        Err(msg) => crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            "transactions",
            Vec::<TransactionRow>::new(),
            TransactionsMeta {
                season: String::new(),
                total: 0,
                active_kind: q.kind.unwrap_or_default(),
                active_team: q.team.unwrap_or_default(),
                empty_unfiltered: true,
                out_of_coverage: false,
                earliest_season: pretty_season(TRANSACTIONS_EARLIEST_SEASON),
            },
            msg,
        ),
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

    let rows = envelope_result.map(|env| env.rows).unwrap_or_default();
    let view = TransactionsView::from_rows(
        ViewContext::new(ViewWindow::new(
            season_str.parse::<u32>().map(Season).unwrap_or(Season(0)),
            SeasonType::parse_lossy(&state.config.read().await.active_season_type),
        )),
        season_str,
        rows,
        kind_filter.as_deref(),
        active_kind,
        team_filter.map(TeamAbbr),
        out_of_coverage,
    );

    Ok(TransactionsResult {
        active_label,
        season_pretty: view.season_pretty,
        rows: view.rows.iter().map(transaction_row_from_view).collect(),
        total: view.total,
        empty_unfiltered: view.empty_unfiltered,
        active_kind: view.active_kind,
        active_team: view.active_team,
        out_of_coverage: view.out_of_coverage,
        earliest_season_pretty: view.earliest_season_pretty,
    })
}

fn transaction_row_from_view(row: &TransactionViewRow) -> TransactionRow {
    TransactionRow {
        date: row.date.clone(),
        team: row.team.clone(),
        kind_label: row.kind_label.clone(),
        kind_pretty: row.kind_pretty.clone(),
        description: row.description.clone(),
    }
}
