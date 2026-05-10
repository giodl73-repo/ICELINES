use serde::{Deserialize, Serialize};

use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayoffsView {
    pub context: ViewContext,
    pub season: String,
    pub season_pretty: String,
    pub source_label: String,
    pub rounds: Vec<PlayoffsRoundRow>,
    pub empty: bool,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl PlayoffsView {
    pub fn from_bracket(
        mut context: ViewContext,
        season: String,
        source_label: String,
        bracket: PlayoffsBracketInput,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Playoffs));

        let rounds: Vec<PlayoffsRoundRow> = bracket
            .rounds
            .into_iter()
            .map(|round| PlayoffsRoundRow {
                round_number: round.round_number,
                label: round.label,
                series: round.series.into_iter().map(playoffs_series_row).collect(),
            })
            .collect();
        let empty = rounds.iter().all(|round| round.series.is_empty());
        let empty_state = if empty {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No playoff bracket".to_string(),
                detail: Some("No playoff series matched the selected season.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            season_pretty: pretty_season(&season),
            season,
            source_label,
            rounds,
            empty,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayoffsBracketInput {
    pub rounds: Vec<PlayoffsRoundInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayoffsRoundInput {
    pub round_number: u8,
    pub label: String,
    pub series: Vec<PlayoffsSeriesInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayoffsSeriesInput {
    pub top_abbrev: String,
    pub top_name: String,
    pub top_wins: u8,
    pub bottom_abbrev: String,
    pub bottom_name: String,
    pub bottom_wins: u8,
    pub winner_abbrev: Option<String>,
    pub conference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayoffsRoundRow {
    pub round_number: u8,
    pub label: String,
    pub series: Vec<PlayoffsSeriesRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayoffsSeriesRow {
    pub top_abbrev: String,
    pub top_name: String,
    pub top_wins: u8,
    pub bottom_abbrev: String,
    pub bottom_name: String,
    pub bottom_wins: u8,
    pub summary: String,
    pub is_complete: bool,
    pub conference: String,
}

fn playoffs_series_row(series: PlayoffsSeriesInput) -> PlayoffsSeriesRow {
    let summary = series_summary(
        &series.top_abbrev,
        series.top_wins,
        &series.bottom_abbrev,
        series.bottom_wins,
        series.winner_abbrev.as_deref(),
    );
    let is_complete = series.top_wins == 4 || series.bottom_wins == 4;

    PlayoffsSeriesRow {
        top_abbrev: series.top_abbrev,
        top_name: series.top_name,
        top_wins: series.top_wins,
        bottom_abbrev: series.bottom_abbrev,
        bottom_name: series.bottom_name,
        bottom_wins: series.bottom_wins,
        summary,
        is_complete,
        conference: series.conference.unwrap_or_default(),
    }
}

fn series_summary(
    top_abbrev: &str,
    top_wins: u8,
    bottom_abbrev: &str,
    bottom_wins: u8,
    winner_abbrev: Option<&str>,
) -> String {
    if let Some(winner) = winner_abbrev {
        format!("{top_abbrev} {top_wins}-{bottom_wins} {bottom_abbrev} · {winner} wins")
    } else if top_wins > bottom_wins {
        format!("{top_abbrev} leads {top_wins}-{bottom_wins}")
    } else if bottom_wins > top_wins {
        format!("{bottom_abbrev} leads {bottom_wins}-{top_wins}")
    } else if top_wins == 0 {
        format!("{top_abbrev} vs {bottom_abbrev} · series begins")
    } else {
        format!("Tied {top_wins}-{bottom_wins}")
    }
}

fn pretty_season(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[0..4], &season[6..8])
    } else {
        season.to_string()
    }
}
