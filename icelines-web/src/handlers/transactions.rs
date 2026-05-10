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
    /// `ir`, `other`. Unknown → 400.
    #[serde(default)]
    pub kind: Option<String>,
    /// Filter by team abbreviation (case-insensitive).
    #[serde(default)]
    pub team: Option<String>,
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
    let (season_str, active_label) = {
        let cfg = state.config.read().await;
        (cfg.active_season.clone(), cfg.active_label.clone())
    };

    // Validate the kind filter early. Bad input → 400, not 500.
    let kind_filter: Option<Vec<TransactionKind>> = match q.kind.as_deref() {
        None | Some("") => None,
        Some(k) => match TransactionKind::parse_filter(k) {
            Ok(v) => Some(v),
            Err(msg) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Html(format!(
                        "<!doctype html><html><body>\
                                 <h1>Bad filter</h1><p>{msg}</p>\
                                 <p><a href=\"/transactions\">← back to transactions</a></p>\
                                 </body></html>",
                    )),
                )
                    .into_response();
            }
        },
    };
    let team_filter = q
        .team
        .as_deref()
        .map(|t| t.trim().to_ascii_uppercase())
        .filter(|t| !t.is_empty());
    let active_kind = q.kind.clone().unwrap_or_default();
    let active_team = team_filter.clone().unwrap_or_default();

    // Out-of-coverage check matches the CLI behavior.
    let out_of_coverage = season_str.as_str() < TRANSACTIONS_EARLIEST_SEASON;

    // Build the SnapshotStore for this request. Cheap — just a
    // PathBuf wrap. If `snapshots_root` is None (test setup),
    // fall back to the default (~/.icelines/snapshots).
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

    // Newest first. Date is YYYY-MM-DD so string sort works.
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    // Cap to 1000 to keep the page render bounded.
    rows.truncate(1000);
    let total = rows.len();
    let empty_unfiltered = rows.is_empty() && kind_filter.is_none() && team_filter.is_none();

    let tmpl = TransactionsTemplate {
        active_label,
        season_pretty: pretty_season(&season_str),
        rows,
        total,
        empty_unfiltered,
        active_kind,
        active_team,
        out_of_coverage,
        earliest_season_pretty: pretty_season(TRANSACTIONS_EARLIEST_SEASON),
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
