use crate::state::WebState;
use crate::templates::{GoalieRow, HomeTemplate, LeaderRow};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;

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
        let st = match cfg.active_season_type.as_str() {
            "playoff" | "playoffs" => SeasonType::Playoff,
            _ => SeasonType::Regular,
        };
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

            let mut skaters: Vec<LeaderRow> = repo
                .skaters(season, season_type)
                .map(|v| super::shared::project_leader_row(&v))
                .collect();
            skaters.sort_by(|a, b| {
                b.points
                    .cmp(&a.points)
                    .then(b.goals.cmp(&a.goals))
                    .then(a.name.cmp(&b.name))
            });
            skaters.truncate(HOME_PREVIEW_N);

            let mut goalies: Vec<GoalieRow> = repo
                .goalies(season, season_type)
                .filter(|v| v.gp() >= goalie_floor)
                .filter_map(|v| {
                    let g = v.stats.goalie.as_ref()?;
                    let save_pct_str = match g.save_pct {
                        Some(p) => format!("{:.3}", p),
                        None => "—".to_owned(),
                    };
                    let gaa_str = match g.goals_against_average {
                        Some(a) => format!("{:.2}", a),
                        None => "—".to_owned(),
                    };
                    let team_display = v.team_display().to_owned();
                    let headshot_url = super::shared::build_headshot_url_for_display(
                        v.season().0,
                        &team_display,
                        v.id().0,
                    );
                    Some(GoalieRow {
                        nhl_id: v.id().0,
                        name: v.full_name().to_owned(),
                        team: team_display,
                        gp: v.gp(),
                        wins: g.wins,
                        losses: g.losses,
                        shutouts: g.shutouts,
                        save_pct_str,
                        gaa_str,
                        headshot_url,
                        headshot_fallback_url: format!(
                            "https://assets.nhle.com/mugs/nhl/default/{}.png",
                            v.id().0
                        ),
                    })
                })
                .collect();
            goalies.sort_by(|a, b| {
                let ap = a.save_pct_str.parse::<f64>().unwrap_or(0.0);
                let bp = b.save_pct_str.parse::<f64>().unwrap_or(0.0);
                bp.partial_cmp(&ap)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.wins.cmp(&a.wins))
                    .then(a.name.cmp(&b.name))
            });
            goalies.truncate(HOME_PREVIEW_N);

            (skaters, goalies)
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
