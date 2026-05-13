use crate::state::WebState;
use crate::templates::{PlayoffsRoundView, PlayoffsSeriesView, PlayoffsTemplate};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    PlayoffsBracketInput, PlayoffsGameInput, PlayoffsRoundInput, PlayoffsRoundRow,
    PlayoffsSeriesInput, PlayoffsSeriesRow, PlayoffsView, ViewContext, ViewWindow,
};

pub(super) struct PlayoffsResult {
    pub(super) active_label: String,
    pub(super) season_pretty: String,
    pub(super) source_label: String,
    pub(super) rounds: Vec<PlayoffsRoundView>,
    pub(super) empty: bool,
    pub(super) fetch_error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct PlayoffsMeta {
    season: String,
    source: String,
    empty: bool,
    round_count: usize,
    series_count: usize,
    source_error: Option<String>,
}

pub async fn get_playoffs(State(state): State<WebState>) -> Response {
    let result = build_playoffs_result(&state).await;
    let tmpl = PlayoffsTemplate {
        active_label: result.active_label,
        season_pretty: result.season_pretty,
        source_label: result.source_label,
        rounds: result.rounds,
        empty: result.empty,
        fetch_error: result.fetch_error,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

pub async fn get_playoffs_json(State(state): State<WebState>) -> Response {
    let result = build_playoffs_result(&state).await;
    let round_count = result.rounds.len();
    let series_count = result.rounds.iter().map(|r| r.series.len()).sum();
    crate::api::json_data_meta(
        "playoffs",
        result.rounds,
        PlayoffsMeta {
            season: result.season_pretty,
            source: result.source_label,
            empty: result.empty,
            round_count,
            series_count,
            source_error: result.fetch_error,
        },
    )
}

pub(super) async fn build_playoffs_result(state: &WebState) -> PlayoffsResult {
    let (active_label, season_str, season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_label.clone(),
            cfg.active_season.clone(),
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(0)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };

    // 1. Try bundled (instant, historical seasons).
    let bundled = icelines_fetch::bundled::load_playoffs(&season_str).map(|b| b.to_bracket());

    let (bracket, source_label, fetch_error) = if let Some(bracket) = bundled {
        (
            playoff_bracket_input(bracket),
            "historical bundle".to_owned(),
            None,
        )
    } else {
        // 2. Fall back to the live API. The playoff endpoint takes
        //    the second year of the season (2026 for 25-26).
        let year: u16 = season_str
            .get(4..8)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if year == 0 {
            (
                PlayoffsBracketInput { rounds: Vec::new() },
                "—".to_owned(),
                Some(format!(
                    "Cannot derive playoff year from season '{season_str}'"
                )),
            )
        } else {
            let client = super::nhl_client();
            match client.fetch_playoff_bracket(year).await {
                Ok(bracket) => (
                    playoff_bracket_input(bracket),
                    format!("live · /v1/playoff-bracket/{year}"),
                    None,
                ),
                Err(e) => (
                    PlayoffsBracketInput { rounds: Vec::new() },
                    "—".to_owned(),
                    Some(e.to_string()),
                ),
            }
        }
    };

    let view = PlayoffsView::from_bracket(
        ViewContext::new(ViewWindow::new(season, season_type)),
        season_str,
        source_label,
        bracket,
    );

    PlayoffsResult {
        active_label,
        season_pretty: view.season_pretty,
        source_label: view.source_label,
        rounds: view.rounds.iter().map(playoffs_round_from_view).collect(),
        empty: view.empty,
        fetch_error,
    }
}

fn playoff_bracket_input(bracket: icelines_fetch::nhl_api::PlayoffBracket) -> PlayoffsBracketInput {
    PlayoffsBracketInput {
        rounds: bracket
            .rounds
            .into_iter()
            .map(|round| PlayoffsRoundInput {
                round_number: round.round_number,
                label: round.label,
                series: round
                    .series
                    .into_iter()
                    .map(|series| PlayoffsSeriesInput {
                        letter: series.letter,
                        top_abbrev: series.top_seed_abbrev,
                        top_name: series.top_seed_name,
                        top_wins: series.top_seed_wins,
                        top_seed_rank: series.top_seed_rank,
                        bottom_abbrev: series.bottom_seed_abbrev,
                        bottom_name: series.bottom_seed_name,
                        bottom_wins: series.bottom_seed_wins,
                        bottom_seed_rank: series.bottom_seed_rank,
                        winner_abbrev: series.winner_abbrev,
                        conference: series.conference,
                        games: series
                            .games
                            .into_iter()
                            .map(|game| PlayoffsGameInput {
                                date: game.date,
                                home_abbrev: game.home_abbrev,
                                away_abbrev: game.away_abbrev,
                                home_score: game.home_score,
                                away_score: game.away_score,
                                series_after: game.series_after,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn playoffs_round_from_view(round: &PlayoffsRoundRow) -> PlayoffsRoundView {
    PlayoffsRoundView {
        round_number: round.round_number,
        label: round.label.clone(),
        series: round.series.iter().map(playoffs_series_from_view).collect(),
    }
}

fn playoffs_series_from_view(series: &PlayoffsSeriesRow) -> PlayoffsSeriesView {
    PlayoffsSeriesView {
        top_abbrev: series.top_abbrev.clone(),
        top_name: series.top_name.clone(),
        top_wins: series.top_wins,
        bottom_abbrev: series.bottom_abbrev.clone(),
        bottom_name: series.bottom_name.clone(),
        bottom_wins: series.bottom_wins,
        summary: series.summary.clone(),
        is_complete: series.is_complete,
        conference: series.conference.clone(),
    }
}
