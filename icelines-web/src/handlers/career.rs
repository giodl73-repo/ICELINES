use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{CareerRow, CareerSortKey, CareerView, ViewContext, ViewWindow};
use serde::Deserialize;

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
fn build_view(q: &CareerQuery) -> Result<CareerView, String> {
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
        return Err("career history store is empty — populate \
                     ~/.icelines/career_history.json via \
                     `icelines fetch career --bundled-seasons 5`"
            .to_owned());
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
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            axum::Json(crate::api::ApiEnvelope::new(
                "career",
                Vec::<CareerRow>::new(),
                error_meta_from_query(&q),
                Some(msg),
            )),
        )
            .into_response(),
    }
}

/// `GET /career` — minimal HTML rendering. Not a templated
/// page yet (Calder.5 polish); plain HTML with the rows so
/// the route exists and the JSON twin has a sibling.
pub async fn get_career(Query(q): Query<CareerQuery>) -> Response {
    match build_view(&q) {
        Ok(view) => {
            let season_label = if view.season.to_string().len() == 8 {
                format!(
                    "{}-{}",
                    &view.season.to_string()[..4],
                    &view.season.to_string()[6..]
                )
            } else {
                view.season.to_string()
            };
            let sort = view.sort.as_str();
            let mut html = format!(
                        "<!doctype html><html><head><title>{league} {season_label} Leaders — IceLines</title>\
                        <style>body{{font-family:system-ui;margin:2rem;max-width:64rem}}\
                        table{{border-collapse:collapse;width:100%}}\
                        th,td{{border-bottom:1px solid #e0e0e0;padding:0.5rem;text-align:left}}\
                        th{{background:#f5f5f5}}.right{{text-align:right}}</style>\
                        </head><body><h1>{league} Leaders — {season_label}</h1>\
                        <p>Sort: <strong>{sort}</strong>  ·  Showing {} of {} rows.  \
                        JSON twin: <a href=\"/api/v1/career?league={league}&season={season}&sort={sort}\">/api/v1/career</a></p>\
                        <table><thead><tr><th>Rank</th><th>Player</th><th>Team</th>\
                        <th class=right>GP</th><th class=right>G</th><th class=right>A</th>\
                        <th class=right>P</th><th class=right>PPG</th></tr></thead><tbody>",
                        view.rows.len(),
                        view.total,
                        league = view.league,
                        season = view.season,
                    );
            for row in &view.rows {
                push_career_row(&mut html, row);
            }
            html.push_str("</tbody></table></body></html>");
            Html(html).into_response()
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

fn push_career_row(html: &mut String, row: &CareerRow) {
    let goals = row
        .goals
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let assists = row
        .assists
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let points = row
        .points
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let ppg = row
        .points_per_game
        .map(|p| format!("{p:.2}"))
        .unwrap_or_else(|| "—".into());
    html.push_str(&format!(
        "<tr><td>{}</td><td><a href=\"/player/{}\">{}</a></td><td>{}</td>\
                            <td class=right>{}</td><td class=right>{}</td><td class=right>{}</td>\
                            <td class=right><strong>{}</strong></td><td class=right>{}</td></tr>",
        row.rank, row.player_id, row.name, row.team, row.gp, goals, assists, points, ppg
    ));
}
