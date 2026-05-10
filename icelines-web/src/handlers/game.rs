use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Debug, Clone, serde::Serialize)]
struct GameDetailView {
    game_id: u64,
    away_abbrev: String,
    home_abbrev: String,
    away_score: u8,
    home_score: u8,
    state_label: String,
    is_live: bool,
    auto_refresh: bool,
    goalies: Vec<GameGoalieView>,
    goals: Vec<GameGoalView>,
    away_top_skaters: Vec<GameSkaterView>,
    home_top_skaters: Vec<GameSkaterView>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameGoalieView {
    player_id: u32,
    player_name: String,
    saves: u32,
    shots: u32,
    decision: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameGoalView {
    period: u8,
    time_in_period: String,
    scorer_team: String,
    scorer_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GameSkaterView {
    player_id: u32,
    player_name: String,
    position: String,
    goals: u32,
    assists: u32,
    points: u32,
    plus_minus: i32,
}

#[derive(Debug, serde::Serialize)]
struct GameMeta {
    game_id: u64,
}

pub async fn get_game(Path(id): Path<u64>) -> Response {
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let body_html = match client.fetch_boxscore(id).await {
        Ok(boxscore) => render_game_html(&project_game_detail(boxscore)),
        Err(e) => render_error_html(id, &e.to_string()),
    };
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body_html),
    )
        .into_response()
}

pub async fn get_game_json(Path(id): Path<u64>) -> Response {
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let (data, error) = match client.fetch_boxscore(id).await {
        Ok(boxscore) => (Some(project_game_detail(boxscore)), None),
        Err(e) => (None, Some(e.to_string())),
    };
    crate::api::json_envelope("game", data, GameMeta { game_id: id }, error)
}

fn project_game_detail(b: icelines_fetch::nhl_api::Boxscore) -> GameDetailView {
    let state = b.game_state.as_deref().unwrap_or("");
    let last = b.last_period.as_deref().unwrap_or("");
    let state_label = match (state, last) {
        ("FINAL" | "OFF", "OT") => "Final/OT",
        ("FINAL" | "OFF", "SO") => "Final/SO",
        ("FINAL" | "OFF", _) => "Final",
        ("LIVE" | "CRIT", _) => "LIVE",
        ("PRE", _) => "Pre-game",
        _ => "",
    }
    .to_owned();
    let is_live = matches!(state, "LIVE" | "CRIT");
    let auto_refresh = matches!(state, "LIVE" | "CRIT" | "PRE");

    let mut away_skaters = b.away_skaters;
    away_skaters.sort_by_key(|s| std::cmp::Reverse(s.goals + s.assists));
    let mut home_skaters = b.home_skaters;
    home_skaters.sort_by_key(|s| std::cmp::Reverse(s.goals + s.assists));

    GameDetailView {
        game_id: b.game_id,
        away_abbrev: b.away_abbrev,
        home_abbrev: b.home_abbrev,
        away_score: b.away_score,
        home_score: b.home_score,
        state_label,
        is_live,
        auto_refresh,
        goalies: b
            .goalies
            .into_iter()
            .map(|g| GameGoalieView {
                player_id: g.player_id,
                player_name: g.player_name,
                saves: g.saves,
                shots: g.shots,
                decision: g.decision,
            })
            .collect(),
        goals: b
            .goals
            .into_iter()
            .map(|g| GameGoalView {
                period: g.period,
                time_in_period: g.time_in_period,
                scorer_team: g.scorer_team,
                scorer_name: g.scorer_name,
            })
            .collect(),
        away_top_skaters: project_top_skaters(away_skaters),
        home_top_skaters: project_top_skaters(home_skaters),
    }
}

fn project_top_skaters(skaters: Vec<icelines_fetch::nhl_api::SkaterLine>) -> Vec<GameSkaterView> {
    skaters
        .into_iter()
        .take(5)
        .map(|s| {
            let points = s.goals + s.assists;
            GameSkaterView {
                player_id: s.player_id,
                player_name: s.player_name,
                position: s.position,
                goals: s.goals,
                assists: s.assists,
                points,
                plus_minus: s.plus_minus,
            }
        })
        .collect()
}

fn render_game_html(b: &GameDetailView) -> String {
    let suffix = if b.state_label.is_empty() {
        String::new()
    } else {
        format!(" - {}", b.state_label)
    };
    let meta_refresh = if b.auto_refresh {
        "<meta http-equiv=\"refresh\" content=\"30\">"
    } else {
        ""
    };
    let mut body = String::new();
    body.push_str("<!DOCTYPE html><html><head>");
    body.push_str("<meta charset=\"utf-8\">");
    body.push_str(meta_refresh);
    body.push_str(&format!(
        "<title>{} @ {} - game {}</title>",
        html_escape(&b.away_abbrev),
        html_escape(&b.home_abbrev),
        b.game_id
    ));
    body.push_str("<link rel=\"stylesheet\" href=\"/static/style.css\">");
    body.push_str("<style>");
    body.push_str(
        ".scoreboard { font-size: 2.4em; font-weight: bold; margin: 1rem 0; } \
                 .scoreboard .away, .scoreboard .home { display: inline-block; min-width: 6rem; \
                  text-align: center; } \
                 .state { color: #b8860b; font-size: 0.95em; margin-left: 0.6rem; } \
                 .goalies { background: #f5f5f5; padding: 0.6rem 1rem; \
                  border-radius: 4px; margin: 0.6rem 0; } \
                 .goal-list li { margin: 0.2rem 0; } \
                 .live-badge { background: #c00; color: white; padding: 0.2rem 0.6rem; \
                  border-radius: 3px; font-size: 0.85em; margin-left: 0.4rem; }",
    );
    body.push_str("</style></head><body>");
    body.push_str(
        "<nav><a href=\"/\">League</a> - <a href=\"/scores\">Scores</a> - \
                 <a href=\"/schedule\">Schedule</a> - <a href=\"/playoffs\">Playoffs</a> - \
                 <a href=\"/transactions\">Transactions</a> - \
                 <a href=\"/favorites\">Favorites</a> - \
                 <strong>Game</strong></nav>",
    );
    body.push_str("<main>");
    body.push_str(&format!(
        "<h1>{} @ {}</h1>",
        html_escape(&b.away_abbrev),
        html_escape(&b.home_abbrev)
    ));
    body.push_str(&format!(
        "<div class=\"scoreboard\">\
                 <span class=\"away\">{} {}</span>\
                 <span style=\"color:#888;\">vs</span>\
                 <span class=\"home\">{} {}</span>\
                 <span class=\"state\">{}{}</span>\
                 </div>",
        html_escape(&b.away_abbrev),
        b.away_score,
        b.home_score,
        html_escape(&b.home_abbrev),
        if b.is_live {
            "<span class=\"live-badge\">LIVE</span>"
        } else {
            ""
        },
        html_escape(&suffix),
    ));

    if !b.goalies.is_empty() {
        body.push_str("<section class=\"goalies\"><h3>Goalies</h3><ul>");
        for g in &b.goalies {
            let dec = g
                .decision
                .as_deref()
                .map(|d| format!(" ({d})"))
                .unwrap_or_default();
            body.push_str(&format!(
                "<li><strong>{}</strong>: {}/{} SV{}{} </li>",
                html_escape(&g.player_name),
                g.saves,
                g.shots,
                html_escape(&dec),
                if g.player_id != 0 {
                    format!(" - <a href=\"/player/{}\">card</a>", g.player_id)
                } else {
                    String::new()
                }
            ));
        }
        body.push_str("</ul></section>");
    }

    if !b.goals.is_empty() {
        body.push_str("<section><h3>Goals</h3><ul class=\"goal-list\">");
        for g in &b.goals {
            body.push_str(&format!(
                "<li>P{} - {} - <strong>{}</strong> {}</li>",
                g.period,
                html_escape(&g.time_in_period),
                html_escape(&g.scorer_team),
                html_escape(&g.scorer_name),
            ));
        }
        body.push_str("</ul></section>");
    }

    render_skaters(&mut body, "Away skaters", &b.away_top_skaters);
    render_skaters(&mut body, "Home skaters", &b.home_top_skaters);

    if b.auto_refresh {
        body.push_str(
            "<p style=\"color:#888;font-size:0.85em;\">\
                     Auto-refreshes every 30 seconds while live.</p>",
        );
    }
    body.push_str("</main></body></html>");
    body
}

fn render_skaters(body: &mut String, label: &str, skaters: &[GameSkaterView]) {
    if skaters.is_empty() {
        return;
    }
    body.push_str(&format!("<section><h3>{label} - top 5 by points</h3><ul>"));
    for s in skaters {
        body.push_str(&format!(
            "<li><a href=\"/player/{}\">{}</a> ({}) - {}G {}A {}P - {:+}</li>",
            s.player_id,
            html_escape(&s.player_name),
            html_escape(&s.position),
            s.goals,
            s.assists,
            s.points,
            s.plus_minus,
        ));
    }
    body.push_str("</ul></section>");
}

fn render_error_html(game_id: u64, err: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
                 <title>Game {game_id} - error</title>\
                 <link rel=\"stylesheet\" href=\"/static/style.css\"></head><body>\
                 <main><h1>Game {game_id}</h1>\
                 <p>Could not fetch boxscore: {err}</p>\
                 <p><a href=\"/scores\">back to scores</a></p>\
                 </main></body></html>",
        err = html_escape(err),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
