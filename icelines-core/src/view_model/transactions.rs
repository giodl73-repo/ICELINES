use serde::{Deserialize, Serialize};

use crate::model::TeamAbbr;
use crate::transactions::{Transaction, TransactionKind, TRANSACTIONS_EARLIEST_SEASON};
use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionsView {
    pub context: ViewContext,
    pub season: String,
    pub season_pretty: String,
    pub active_kind: String,
    pub active_team: String,
    pub rows: Vec<TransactionViewRow>,
    pub total: usize,
    pub empty_unfiltered: bool,
    pub out_of_coverage: bool,
    pub earliest_season_pretty: String,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl TransactionsView {
    pub fn from_rows(
        context: ViewContext,
        season: String,
        rows: Vec<Transaction>,
        kind_filter: Option<&[TransactionKind]>,
        active_kind: String,
        team_filter: Option<TeamAbbr>,
        out_of_coverage: bool,
    ) -> Self {
        Self::from_rows_limited(
            context,
            season,
            rows,
            kind_filter,
            active_kind,
            team_filter,
            out_of_coverage,
            Some(1000),
        )
    }

    pub fn from_rows_unlimited(
        context: ViewContext,
        season: String,
        rows: Vec<Transaction>,
        kind_filter: Option<&[TransactionKind]>,
        active_kind: String,
        team_filter: Option<TeamAbbr>,
        out_of_coverage: bool,
    ) -> Self {
        Self::from_rows_limited(
            context,
            season,
            rows,
            kind_filter,
            active_kind,
            team_filter,
            out_of_coverage,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_rows_limited(
        mut context: ViewContext,
        season: String,
        rows: Vec<Transaction>,
        kind_filter: Option<&[TransactionKind]>,
        active_kind: String,
        team_filter: Option<TeamAbbr>,
        out_of_coverage: bool,
        row_limit: Option<usize>,
    ) -> Self {
        if out_of_coverage {
            context
                .source_state
                .push(SourceState::missing(SourceKind::Transactions));
        } else {
            context
                .source_state
                .push(SourceState::complete(SourceKind::Transactions));
        }

        let active_team = team_filter
            .as_ref()
            .map(|team| team.0.clone())
            .unwrap_or_default();
        let mut rows: Vec<TransactionViewRow> = rows
            .into_iter()
            .filter(|transaction| match kind_filter {
                None => true,
                Some(kinds) => kinds.contains(&transaction.kind),
            })
            .filter(|transaction| match &team_filter {
                None => true,
                Some(team) if team.0.eq_ignore_ascii_case("LEAGUE") => transaction.team.is_none(),
                Some(team) => transaction
                    .team
                    .as_ref()
                    .map(|transaction_team| transaction_team.0.eq_ignore_ascii_case(&team.0))
                    .unwrap_or(false),
            })
            .map(transaction_row)
            .collect();

        rows.sort_by(|a, b| b.date.cmp(&a.date));
        if let Some(row_limit) = row_limit {
            rows.truncate(row_limit);
        }
        let total = rows.len();
        let empty_unfiltered = total == 0 && kind_filter.is_none() && team_filter.is_none();
        let empty_state = if total == 0 {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No transactions".to_string(),
                detail: Some(if out_of_coverage {
                    "Transactions coverage is not available for this season.".to_string()
                } else {
                    "No transactions matched the selected filters.".to_string()
                }),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            season_pretty: pretty_season(&season),
            season,
            active_kind,
            active_team,
            rows,
            total,
            empty_unfiltered,
            out_of_coverage,
            earliest_season_pretty: pretty_season(TRANSACTIONS_EARLIEST_SEASON),
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionViewRow {
    pub id: String,
    pub date: String,
    pub team: String,
    #[serde(skip, default = "default_transaction_kind")]
    pub kind: TransactionKind,
    pub kind_label: String,
    pub kind_pretty: String,
    pub description: String,
}

fn transaction_row(transaction: Transaction) -> TransactionViewRow {
    let kind = transaction.kind;
    TransactionViewRow {
        id: transaction.id,
        date: transaction.date,
        team: transaction.team.map(|team| team.0).unwrap_or_default(),
        kind,
        kind_label: kind.label().to_string(),
        kind_pretty: pretty_kind(kind).to_string(),
        description: transaction.description,
    }
}

fn pretty_season(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[0..4], &season[6..8])
    } else {
        season.to_string()
    }
}

fn pretty_kind(kind: TransactionKind) -> &'static str {
    match kind {
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

fn default_transaction_kind() -> TransactionKind {
    TransactionKind::Other
}
