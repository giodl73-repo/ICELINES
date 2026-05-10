use crate::state::WebState;
use crate::templates::{GoalieRow, HomeTemplate, LeaderRow};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{HomeGoalieRow, HomeSkaterRow, HomeView};

/// Goalie qualified-GP floors used in the home preview.
/// Mirrors the constants in the goalies handler — kept local
/// rather than re-exported because the values are identical
/// today and a divergence would be a deliberate decision.
const HOME_QUALIFIED_GP_REGULAR: u32 = 5;
const HOME_QUALIFIED_GP_PLAYOFF: u32 = 1;
const HOME_PREVIEW_N: usize = 3;

/// `GET /` — askama-rendered home with top-3 skater + goalie
/// previews. Reads the active (season, season_type) from
/// `WebState.config`, then takes one read lock on the repo to
/// project both slices. Empty-vec fallbacks (rather than
/// erroring) so the home page stays useful even when the
/// active season has no data loaded yet.
pub async fn get_home(State(state): State<WebState>) -> Response {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };

    let (top_skaters, top_goalies) = match season_str.parse::<u32>() {
        Ok(season_u32) => {
            let season = Season(season_u32);
            let goalie_floor = match season_type {
                SeasonType::Regular => HOME_QUALIFIED_GP_REGULAR,
                SeasonType::Playoff => HOME_QUALIFIED_GP_PLAYOFF,
            };
            let repo = state.repo.read().await;
            let view =
                HomeView::from_repository(&repo, season, season_type, goalie_floor, HOME_PREVIEW_N);
            (
                view.top_skaters
                    .iter()
                    .map(|row| skater_preview_from_view(row, season.0))
                    .collect(),
                view.top_goalies
                    .iter()
                    .map(|row| goalie_preview_from_view(row, season.0))
                    .collect(),
            )
        }
        Err(_) => (Vec::new(), Vec::new()),
    };

    let tmpl = HomeTemplate {
        active_label,
        top_skaters,
        top_goalies,
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

fn skater_preview_from_view(row: &HomeSkaterRow, season: u32) -> LeaderRow {
    let team = row.team.0.clone();
    LeaderRow {
        nhl_id: row.player_id.0,
        name: row.display_name.clone(),
        position: row.position.abbreviation().to_string(),
        team: team.clone(),
        gp: row.gp,
        goals: row.goals,
        assists: row.assists,
        points: row.points,
        ppg_str: if row.gp == 0 {
            String::new()
        } else {
            format!("{:.2}", row.points as f64 / row.gp as f64)
        },
        headshot_url: super::shared::build_headshot_url_for_display(season, &team, row.player_id.0),
        headshot_fallback_url: format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            row.player_id.0
        ),
        plus_minus_str: String::new(),
        pim: 0,
        shots: 0,
        shooting_pct_str: String::new(),
        hits_str: String::new(),
        blocks_str: String::new(),
        faceoff_pct_str: String::new(),
        pp_points: 0,
        plus_minus: 0,
        shooting_pct: None,
        hits: None,
        blocks: None,
        faceoff_pct: None,
        points_per_60_str: String::new(),
        goals_per_60_str: String::new(),
        assists_per_60_str: String::new(),
        hits_per_60_str: String::new(),
        blocks_per_60_str: String::new(),
        points_per_60: None,
        goals_per_60: None,
        assists_per_60: None,
        hits_per_60: None,
        blocks_per_60: None,
        points_delta: None,
        points_delta_str: String::new(),
        points_delta_class: String::new(),
    }
}

fn goalie_preview_from_view(row: &HomeGoalieRow, season: u32) -> GoalieRow {
    let team = row.team.0.clone();
    GoalieRow {
        nhl_id: row.player_id.0,
        name: row.display_name.clone(),
        team: team.clone(),
        gp: row.gp,
        wins: row.wins,
        losses: row.losses,
        saves: 0,
        shutouts: row.shutouts,
        save_pct_str: row
            .save_pct
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "—".to_owned()),
        gaa_str: row
            .goals_against_average
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "—".to_owned()),
        headshot_url: super::shared::build_headshot_url_for_display(season, &team, row.player_id.0),
        headshot_fallback_url: format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            row.player_id.0
        ),
    }
}
