use axum::extract::Query;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::career_history::CareerGameType;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CareerQuery {
    pub league: Option<String>,
    pub season: Option<String>,
    pub sort: Option<String>,
    pub top: Option<usize>,
}

#[derive(Debug, serde::Serialize)]
pub struct CareerRow {
    pub rank: usize,
    pub player_id: u32,
    pub name: String,
    pub team: String,
    pub gp: u32,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
struct Meta<'a> {
    league: &'a str,
    season: u32,
    sort: &'static str,
    count: usize,
    total: usize,
}

#[derive(Debug, serde::Serialize)]
struct Envelope<'a> {
    schema_version: u32,
    route: &'static str,
    data: &'a [CareerRow],
    meta: Meta<'a>,
}

/// Resolve league + season + sort + top from query params,
/// load the local store, project into rows. Shared by HTML
/// and JSON handlers so they can't drift.
fn build_rows(
    q: &CareerQuery,
) -> Result<(Vec<CareerRow>, String, u32, &'static str, usize), String> {
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
    let sort_label: &'static str = match sort_token.to_ascii_lowercase().as_str() {
        "points" | "p" | "pts" => "points",
        "goals" | "g" => "goals",
        "assists" | "a" => "assists",
        "gp" | "games" => "gp",
        "ppg" | "points-per-game" => "ppg",
        _ => return Err(format!("unknown sort '{sort_token}'")),
    };
    let top = q.top.unwrap_or(20).min(500);

    let store = icelines_fetch::career_landing::load_local_store();
    if store.is_empty() {
        return Err("career history store is empty — populate \
                     ~/.icelines/career_history.json via \
                     `icelines fetch career --bundled-seasons 5`"
            .to_owned());
    }

    // Filter + sort. Mirrors icelines-cli/src/commands/query_career.rs.
    let needle = league.to_ascii_uppercase();
    let mut matched: Vec<(u32, &icelines_core::career_history::CareerStint)> = Vec::new();
    for (pid_str, h) in store.histories.iter() {
        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };
        for s in &h.stints {
            if s.league.0.to_ascii_uppercase() != needle {
                continue;
            }
            if !matches!(s.game_type, CareerGameType::Regular) {
                continue;
            }
            if let Some(want) = season {
                if s.season.0 != want {
                    continue;
                }
            }
            matched.push((pid, s));
        }
    }
    if season.is_none() {
        if let Some(latest) = matched.iter().map(|(_, s)| s.season.0).max() {
            matched.retain(|(_, s)| s.season.0 == latest);
        }
    }
    // Sort, descending.
    matched.sort_by(|(pa, a), (pb, b)| {
        let ka = metric(a, sort_label);
        let kb = metric(b, sort_label);
        kb.partial_cmp(&ka)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| pa.cmp(pb))
    });

    let total = matched.len();
    let resolved_season = matched.first().map(|(_, s)| s.season.0).unwrap_or(0);

    // Resolve names from bundled bios, single eager scan.
    let pids: Vec<u32> = matched.iter().take(top).map(|(p, _)| *p).collect();
    let names = resolve_names(&pids);

    let rows: Vec<CareerRow> = matched
        .iter()
        .take(top)
        .enumerate()
        .map(|(i, (pid, s))| CareerRow {
            rank: i + 1,
            player_id: *pid,
            name: names
                .get(pid)
                .cloned()
                .unwrap_or_else(|| format!("player:{pid}")),
            team: s.team.clone(),
            gp: s.gp,
            goals: s.goals,
            assists: s.assists,
            points: s.points,
            points_per_game: s.points_per_game().map(|p| p as f64),
        })
        .collect();
    Ok((rows, league.to_owned(), resolved_season, sort_label, total))
}

fn metric(s: &icelines_core::career_history::CareerStint, sort: &str) -> Option<f64> {
    match sort {
        "points" => s.points.map(|n| n as f64),
        "goals" => s.goals.map(|n| n as f64),
        "assists" => s.assists.map(|n| n as f64),
        "gp" => Some(s.gp as f64),
        "ppg" => s.points_per_game().map(|p| p as f64),
        _ => None,
    }
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
    match build_rows(&q) {
        Ok((rows, league, season, sort, total)) => {
            let env = Envelope {
                schema_version: 1,
                route: "career",
                data: &rows,
                meta: Meta {
                    league: &league,
                    season,
                    sort,
                    count: rows.len(),
                    total,
                },
            };
            axum::Json(env).into_response()
        }
        Err(msg) => (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": msg})),
        )
            .into_response(),
    }
}

/// `GET /career` — minimal HTML rendering. Not a templated
/// page yet (Calder.5 polish); plain HTML with the rows so
/// the route exists and the JSON twin has a sibling.
pub async fn get_career(Query(q): Query<CareerQuery>) -> Response {
    match build_rows(&q) {
        Ok((rows, league, season, sort, total)) => {
            let season_label = if season.to_string().len() == 8 {
                format!("{}-{}", &season.to_string()[..4], &season.to_string()[6..])
            } else {
                season.to_string()
            };
            let mut html = format!(
                        "<!doctype html><html><head><title>{league} {season_label} Leaders — IceLines</title>\
                        <style>body{{font-family:system-ui;margin:2rem;max-width:64rem}}\
                        table{{border-collapse:collapse;width:100%}}\
                        th,td{{border-bottom:1px solid #e0e0e0;padding:0.5rem;text-align:left}}\
                        th{{background:#f5f5f5}}.right{{text-align:right}}</style>\
                        </head><body><h1>{league} Leaders — {season_label}</h1>\
                        <p>Sort: <strong>{sort}</strong>  ·  Showing {} of {total} rows.  \
                        JSON twin: <a href=\"/api/v1/career?league={league}&season={season}&sort={sort}\">/api/v1/career</a></p>\
                        <table><thead><tr><th>Rank</th><th>Player</th><th>Team</th>\
                        <th class=right>GP</th><th class=right>G</th><th class=right>A</th>\
                        <th class=right>P</th><th class=right>PPG</th></tr></thead><tbody>",
                        rows.len()
                    );
            for r in &rows {
                let goals = r.goals.map(|n| n.to_string()).unwrap_or_else(|| "—".into());
                let assists = r
                    .assists
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "—".into());
                let points = r
                    .points
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "—".into());
                let ppg = r
                    .points_per_game
                    .map(|p| format!("{p:.2}"))
                    .unwrap_or_else(|| "—".into());
                html.push_str(&format!(
                    "<tr><td>{}</td><td><a href=\"/player/{}\">{}</a></td><td>{}</td>\
                            <td class=right>{}</td><td class=right>{}</td><td class=right>{}</td>\
                            <td class=right><strong>{}</strong></td><td class=right>{}</td></tr>",
                    r.rank, r.player_id, r.name, r.team, r.gp, goals, assists, points, ppg
                ));
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
