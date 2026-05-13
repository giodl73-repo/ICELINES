use askama::Template;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    CareerRow, CareerSortKey, CareerView, ViewContext, ViewWindow,
    CAREER_HISTORY_MISSING_STORE_MESSAGE,
};
use serde::Deserialize;

use crate::templates::{CareerLeaderRow, CareerTemplate};

#[derive(Debug, Deserialize)]
pub struct CareerQuery {
    pub league: Option<String>,
    pub season: Option<String>,
    pub sort: Option<String>,
    pub top: Option<usize>,
}

#[derive(Debug, serde::Serialize)]
struct Meta {
    league: String,
    season: u32,
    sort: String,
    count: usize,
    total: usize,
}

fn meta_from_view(view: &CareerView) -> Meta {
    Meta {
        league: view.league.clone(),
        season: view.season,
        sort: view.sort.as_str().to_owned(),
        count: view.count,
        total: view.total,
    }
}

fn error_meta_from_query(q: &CareerQuery) -> Meta {
    Meta {
        league: q.league.clone().unwrap_or_default(),
        season: q
            .season
            .as_deref()
            .and_then(|season| season.parse::<u32>().ok())
            .unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_else(|| "points".to_owned()),
        count: 0,
        total: 0,
    }
}

/// Resolve league + season + sort + top from query params,
/// load the local store, project into a shared CareerView.
pub(super) fn build_view(q: &CareerQuery) -> Result<CareerView, String> {
    let league = q
        .league
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing required ?league=… param".to_owned())?;
    let season = match q.season.as_deref() {
        None => None,
        Some(s) => Some(
            s.parse::<u32>()
                .map_err(|_| format!("season '{s}' is not a YYYYZZZZ id"))?,
        ),
    };
    let sort_token = q.sort.as_deref().unwrap_or("points");
    let sort =
        CareerSortKey::parse(sort_token).ok_or_else(|| format!("unknown sort '{sort_token}'"))?;
    let top = q.top.unwrap_or(20).min(500);

    let store = icelines_fetch::career_landing::load_local_store();
    if store.is_empty() {
        return Err(CAREER_HISTORY_MISSING_STORE_MESSAGE.to_owned());
    }

    let histories: Vec<(u32, icelines_core::CareerHistory)> = store
        .histories
        .iter()
        .filter_map(|(pid_str, history)| {
            pid_str
                .parse::<u32>()
                .ok()
                .map(|pid| (pid, history.clone()))
        })
        .collect();
    let pids: Vec<u32> = histories.iter().map(|(pid, _)| *pid).collect();
    let names = resolve_names(&pids);
    Ok(CareerView::from_histories(
        ViewContext::new(ViewWindow::new(Season(0), SeasonType::Regular)),
        league.to_owned(),
        season,
        sort,
        top,
        histories,
        names,
    ))
}

fn resolve_names(wanted: &[u32]) -> std::collections::HashMap<u32, String> {
    use icelines_fetch::bundled;
    let want: std::collections::HashSet<u32> = wanted.iter().copied().collect();
    let mut out: std::collections::HashMap<u32, String> = Default::default();
    for season_id in bundled::BUNDLED_SEASONS {
        if let Some(bios) = bundled::get_bios(season_id) {
            for b in bios {
                if want.contains(&b.player_id) {
                    out.entry(b.player_id)
                        .or_insert_with(|| b.skater_full_name.clone());
                }
            }
        }
        if let Some(goalies) = bundled::get_goalie_stats(season_id) {
            for g in goalies {
                if want.contains(&g.player_id) {
                    out.entry(g.player_id)
                        .or_insert_with(|| g.goalie_full_name.clone());
                }
            }
        }
        if out.len() == want.len() {
            break;
        }
    }
    out
}

/// `GET /api/v1/career` — JSON twin. King.2.4 envelope shape.
pub async fn get_career_json(Query(q): Query<CareerQuery>) -> Response {
    match build_view(&q) {
        Ok(view) => {
            let meta = meta_from_view(&view);
            crate::api::json_data_meta("career", view.rows, meta)
        }
        Err(msg) => crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            "career",
            Vec::<CareerRow>::new(),
            error_meta_from_query(&q),
            msg,
        ),
    }
}

/// `GET /career` — HTML sibling for the JSON cohort leaderboard.
pub async fn get_career(
    State(state): State<crate::WebState>,
    Query(q): Query<CareerQuery>,
) -> Response {
    match build_view(&q) {
        Ok(view) => {
            let active_label = state.config.read().await.active_label.clone();
            let tmpl = CareerTemplate {
                active_label,
                league: view.league.clone(),
                season_label: season_label(view.season),
                season: view.season,
                sort: view.sort.as_str().to_owned(),
                count: view.rows.len(),
                total: view.total,
                rows: view.rows.iter().map(career_leader_row).collect(),
            };
            render_template(tmpl)
        }
        Err(msg) => (
            axum::http::StatusCode::BAD_REQUEST,
            Html(format!(
                "<!doctype html><body><h1>400</h1><p>{msg}</p>\
                        <p>Try <code>/career?league=OHL&amp;season=20142015</code></p></body>"
            )),
        )
            .into_response(),
    }
}

fn season_label(season: u32) -> String {
    let season = season.to_string();
    if season.len() == 8 {
        format!("{}-{}", &season[..4], &season[6..])
    } else {
        season
    }
}

fn career_leader_row(row: &CareerRow) -> CareerLeaderRow {
    CareerLeaderRow {
        rank: row.rank,
        player_id: row.player_id,
        name: row.name.clone(),
        team: row.team.clone(),
        gp: row.gp,
        goals: optional_u32(row.goals),
        assists: optional_u32(row.assists),
        points: optional_u32(row.points),
        points_per_game: row
            .points_per_game
            .map(|p| format!("{p:.2}"))
            .unwrap_or_else(|| "-".to_owned()),
    }
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|n| n.to_string())
        .unwrap_or_else(|| "-".to_owned())
}

fn render_template<T: Template>(tmpl: T) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<!doctype html><body><h1>500</h1><p>{}</p></body>",
                html_escape(&e.to_string())
            )),
        )
            .into_response(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
