use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub async fn get_game(Path(id): Path<u64>) -> Response {
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let body_html = match client.fetch_boxscore(id).await {
        Ok(boxscore) => render_game_html(&boxscore),
        Err(e) => render_error_html(id, &e.to_string()),
    };
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body_html),
    )
        .into_response()
}

fn render_game_html(b: &icelines_fetch::nhl_api::Boxscore) -> String {
    let state = b.game_state.as_deref().unwrap_or("");
    let last = b.last_period.as_deref().unwrap_or("");
    let suffix = match (state, last) {
        ("FINAL" | "OFF", "OT") => " · Final/OT",
        ("FINAL" | "OFF", "SO") => " · Final/SO",
        ("FINAL" | "OFF", _) => " · Final",
        ("LIVE" | "CRIT", _) => " · LIVE",
        ("PRE", _) => " · Pre-game",
        _ => "",
    };
    // Auto-refresh every 30s when live.
    let auto_refresh = matches!(state, "LIVE" | "CRIT" | "PRE");
    let meta_refresh = if auto_refresh {
        "<meta http-equiv=\"refresh\" content=\"30\">"
    } else {
        ""
    };
    let mut body = String::new();
    body.push_str("<!DOCTYPE html><html><head>");
    body.push_str("<meta charset=\"utf-8\">");
    body.push_str(meta_refresh);
    body.push_str(&format!(
        "<title>{} @ {} — game {}</title>",
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
        "<nav><a href=\"/\">League</a> · <a href=\"/scores\">Scores</a> · \
                 <a href=\"/schedule\">Schedule</a> · <a href=\"/playoffs\">Playoffs</a> · \
                 <a href=\"/transactions\">Transactions</a> · \
                 <a href=\"/favorites\">Favorites</a> · \
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
        if matches!(state, "LIVE" | "CRIT") {
            "<span class=\"live-badge\">LIVE</span>"
        } else {
            ""
        },
        suffix,
    ));

    // Goalies
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
                dec,
                if g.player_id != 0 {
                    format!(" — <a href=\"/player/{}\">card</a>", g.player_id)
                } else {
                    String::new()
                }
            ));
        }
        body.push_str("</ul></section>");
    }

    // Goal summary
    if !b.goals.is_empty() {
        body.push_str("<section><h3>Goals</h3><ul class=\"goal-list\">");
        for g in &b.goals {
            body.push_str(&format!(
                "<li>P{} · {} · <strong>{}</strong> {}</li>",
                g.period,
                html_escape(&g.time_in_period),
                html_escape(&g.scorer_team),
                html_escape(&g.scorer_name),
            ));
        }
        body.push_str("</ul></section>");
    }

    // Per-team skater rows (top scorers)
    for (label, skaters) in [
        ("Away skaters", &b.away_skaters),
        ("Home skaters", &b.home_skaters),
    ] {
        if skaters.is_empty() {
            continue;
        }
        let mut sorted = skaters.clone();
        sorted.sort_by_key(|s| std::cmp::Reverse(s.goals + s.assists));
        let top: Vec<_> = sorted.iter().take(5).collect();
        if top.is_empty() {
            continue;
        }
        body.push_str(&format!("<section><h3>{label} — top 5 by points</h3><ul>"));
        for s in top {
            body.push_str(&format!(
                "<li><a href=\"/player/{}\">{}</a> ({}) — {}G {}A {}P · {:+}</li>",
                s.player_id,
                html_escape(&s.player_name),
                html_escape(&s.position),
                s.goals,
                s.assists,
                s.goals + s.assists,
                s.plus_minus,
            ));
        }
        body.push_str("</ul></section>");
    }

    if auto_refresh {
        body.push_str(
            "<p style=\"color:#888;font-size:0.85em;\">\
                     Auto-refreshes every 30 seconds while live.</p>",
        );
    }
    body.push_str("</main></body></html>");
    body
}

fn render_error_html(game_id: u64, err: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
                 <title>Game {game_id} — error</title>\
                 <link rel=\"stylesheet\" href=\"/static/style.css\"></head><body>\
                 <main><h1>Game {game_id}</h1>\
                 <p>Could not fetch boxscore: {err}</p>\
                 <p><a href=\"/scores\">← back to scores</a></p>\
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
