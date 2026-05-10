use crate::state::WebState;
use crate::templates::{PlayoffsRoundView, PlayoffsSeriesView, PlayoffsTemplate};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

fn pretty_season(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}", &s[0..4], &s[6..8])
    } else {
        s.to_owned()
    }
}

/// Convert a `PlayoffBracket` (live or bundled-derived) into
/// the template's view shape.
fn project_bracket(b: icelines_fetch::nhl_api::PlayoffBracket) -> Vec<PlayoffsRoundView> {
    b.rounds
        .into_iter()
        .map(|r| {
            let series = r
                .series
                .iter()
                .map(|s| PlayoffsSeriesView {
                    top_abbrev: s.top_seed_abbrev.clone(),
                    top_name: s.top_seed_name.clone(),
                    top_wins: s.top_seed_wins,
                    bottom_abbrev: s.bottom_seed_abbrev.clone(),
                    bottom_name: s.bottom_seed_name.clone(),
                    bottom_wins: s.bottom_seed_wins,
                    summary: s.summary(),
                    is_complete: s.is_complete(),
                    conference: s.conference.clone().unwrap_or_default(),
                })
                .collect();
            PlayoffsRoundView {
                round_number: r.round_number,
                label: r.label,
                series,
            }
        })
        .collect()
}

pub async fn get_playoffs(State(state): State<WebState>) -> Response {
    let (active_label, season_str) = {
        let cfg = state.config.read().await;
        (cfg.active_label.clone(), cfg.active_season.clone())
    };

    // 1. Try bundled (instant, historical seasons).
    let bundled = icelines_fetch::bundled::load_playoffs(&season_str).map(|b| b.to_bracket());

    let (rounds, source_label, fetch_error) = if let Some(bracket) = bundled {
        (
            project_bracket(bracket),
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
                Vec::new(),
                "—".to_owned(),
                Some(format!(
                    "Cannot derive playoff year from season '{season_str}'"
                )),
            )
        } else {
            let client = super::nhl_client();
            match client.fetch_playoff_bracket(year).await {
                Ok(b) => (
                    project_bracket(b),
                    format!("live · /v1/playoff-bracket/{year}"),
                    None,
                ),
                Err(e) => (Vec::new(), "—".to_owned(), Some(e.to_string())),
            }
        }
    };

    let empty = rounds.iter().all(|r| r.series.is_empty());

    let tmpl = PlayoffsTemplate {
        active_label,
        season_pretty: pretty_season(&season_str),
        source_label,
        rounds,
        empty,
        fetch_error,
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
