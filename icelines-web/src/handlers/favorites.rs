use axum::extract::{Form, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

pub async fn get_favorites() -> Response {
    // Read members from the local SQLite db. The web server
    // runs on the same machine as the CLI / TUI so the same
    // `~/.icelines/icelines.db` is reachable.
    let members: Vec<(String, String)> = match std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
    {
        Some(home) => {
            let dir = std::path::PathBuf::from(&home).join(".icelines");
            let db_path = dir.join("icelines.db");
            if !db_path.exists() {
                Vec::new()
            } else {
                match rusqlite::Connection::open(&db_path) {
                    Ok(conn) => {
                        // Match icelines-cli's GroupDb queries:
                        // post-006 the column is entity_ref.
                        let mut stmt = match conn.prepare(
                            "SELECT entity_ref FROM group_members \
                                     WHERE group_name = 'Favorites' \
                                     ORDER BY entity_ref",
                        ) {
                            Ok(s) => s,
                            Err(_) => {
                                return error_response("Could not read favorites from local db.")
                            }
                        };
                        let rows: Vec<String> = stmt
                            .query_map([], |r| r.get::<_, String>(0))
                            .ok()
                            .map(|i| i.filter_map(Result::ok).collect())
                            .unwrap_or_default();
                        rows.into_iter()
                            .map(|er| match er.split_once(':') {
                                Some(("team", k)) => ("team".into(), k.into()),
                                Some(("player", k)) => ("player".into(), k.into()),
                                _ => ("player".into(), er),
                            })
                            .collect()
                    }
                    Err(_) => Vec::new(),
                }
            }
        }
        None => Vec::new(),
    };

    // Phase Foster +21 — for each favorited player resolve to
    // a PlayerId and walk the persisted boxscore JSON to pull
    // tonight's stat line. Best-effort: missing bundle bios
    // → row drops to "no resolved pid"; missing boxscore →
    // dash row.
    let stat_lines = compute_player_stat_lines(&members).await;

    let body = render_html(&members, &stat_lines);
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body),
    )
        .into_response()
}

pub async fn get_watchlist(State(state): State<crate::WebState>) -> Response {
    let members = read_group_members("Watchlist");
    let notes = read_watch_notes();
    let active_label = state.config.read().await.active_label.clone();
    let body = render_watchlist_html(&members, &notes, &active_label);
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body),
    )
        .into_response()
}

pub async fn get_favorites_json() -> Response {
    let members = read_group_members("Favorites");
    let rows = group_api_rows(&members);
    let meta = GroupApiMeta {
        group: "Favorites",
        count: rows.len(),
        player_count: rows.iter().filter(|r| r.kind == "player").count(),
        team_count: rows.iter().filter(|r| r.kind == "team").count(),
    };
    axum::Json(GroupApiResponse {
        schema_version: "favorites.v1",
        route: "favorites",
        data: rows,
        meta,
    })
    .into_response()
}

pub async fn get_watchlist_json() -> Response {
    let members = read_group_members("Watchlist");
    let notes = read_watch_notes();
    let rows = watchlist_api_rows(&members, &notes);
    let meta = WatchlistApiMeta {
        group: "Watchlist",
        count: rows.len(),
        player_count: rows.iter().filter(|r| r.kind == "player").count(),
        team_count: rows.iter().filter(|r| r.kind == "team").count(),
    };
    axum::Json(WatchlistApiResponse {
        schema_version: "watchlist.v1",
        route: "watchlist",
        data: rows,
        meta,
    })
    .into_response()
}

#[derive(Debug, Clone)]
struct WatchNote {
    reason: String,
    source: String,
    updated_at: String,
}

#[derive(Debug, serde::Serialize)]
struct GroupApiResponse {
    schema_version: &'static str,
    route: &'static str,
    data: Vec<GroupApiRow>,
    meta: GroupApiMeta,
}

#[derive(Debug, serde::Serialize)]
struct GroupApiMeta {
    group: &'static str,
    count: usize,
    player_count: usize,
    team_count: usize,
}

#[derive(Debug, serde::Serialize)]
struct GroupApiRow {
    kind: String,
    key: String,
}

#[derive(Debug, serde::Serialize)]
struct WatchlistApiResponse {
    schema_version: &'static str,
    route: &'static str,
    data: Vec<WatchlistApiRow>,
    meta: WatchlistApiMeta,
}

#[derive(Debug, serde::Serialize)]
struct WatchlistApiMeta {
    group: &'static str,
    count: usize,
    player_count: usize,
    team_count: usize,
}

#[derive(Debug, serde::Serialize)]
struct WatchlistApiRow {
    kind: String,
    key: String,
    reason: Option<String>,
    source: Option<String>,
    updated_at: Option<String>,
}

fn group_api_rows(members: &[(String, String)]) -> Vec<GroupApiRow> {
    members
        .iter()
        .map(|(kind, key)| GroupApiRow {
            kind: kind.clone(),
            key: key.clone(),
        })
        .collect()
}

fn watchlist_api_rows(
    members: &[(String, String)],
    notes: &std::collections::HashMap<String, WatchNote>,
) -> Vec<WatchlistApiRow> {
    members
        .iter()
        .map(|(kind, key)| {
            let entity_ref = format!("{kind}:{key}");
            let note = notes.get(&entity_ref);
            WatchlistApiRow {
                kind: kind.clone(),
                key: key.clone(),
                reason: note.map(|n| n.reason.clone()),
                source: note.map(|n| n.source.clone()),
                updated_at: note.map(|n| n.updated_at.clone()),
            }
        })
        .collect()
}

fn read_group_members(group_name: &str) -> Vec<(String, String)> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return Vec::new();
    };
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return Vec::new();
    }
    let Ok(conn) = rusqlite::Connection::open(&db_path) else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT entity_ref FROM group_members \
                 WHERE group_name = ?1 \
                 ORDER BY entity_ref",
    ) else {
        return Vec::new();
    };
    stmt.query_map(rusqlite::params![group_name], |r| r.get::<_, String>(0))
        .ok()
        .map(|rows| {
            rows.filter_map(Result::ok)
                .map(|er| match er.split_once(':') {
                    Some(("team", k)) => ("team".into(), k.into()),
                    Some(("player", k)) => ("player".into(), k.into()),
                    _ => ("player".into(), er),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_watch_notes() -> std::collections::HashMap<String, WatchNote> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return std::collections::HashMap::new();
    };
    let db_path = std::path::PathBuf::from(&home)
        .join(".icelines")
        .join("icelines.db");
    if !db_path.exists() {
        return std::collections::HashMap::new();
    }
    let Ok(conn) = rusqlite::Connection::open(&db_path) else {
        return std::collections::HashMap::new();
    };
    let Ok(mut stmt) =
        conn.prepare("SELECT entity_ref, reason, source, updated_at FROM watch_notes")
    else {
        return std::collections::HashMap::new();
    };
    stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            WatchNote {
                reason: r.get::<_, String>(1)?,
                source: r.get::<_, String>(2)?,
                updated_at: r.get::<_, String>(3)?,
            },
        ))
    })
    .ok()
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

/// Per-favorited-player stat-line lookup. Returns a flat
/// vec of (display_name, formatted_line) pairs the renderer
/// drops in below the player's name. Empty when no boxscore
/// data is on disk yet — caller falls back to plain listing.
async fn compute_player_stat_lines(
    members: &[(String, String)],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut out = HashMap::new();

    // Today's slate fetch (best-effort).
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let slate = match client.fetch_schedule_for_date(&today).await {
        Ok(g) => g
            .into_iter()
            .filter(|g| g.date == today)
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    if slate.is_empty() {
        return out;
    }

    let home = match std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        Some(h) => std::path::PathBuf::from(h),
        None => return out,
    };
    let data_root = home.join(".icelines").join("data");
    let store = match icelines_fetch::datastore::DataStore::open(&data_root) {
        Ok(s) => s,
        Err(_) => return out,
    };

    for (kind, name) in members {
        if kind != "player" {
            continue;
        }
        let Some(pid) = icelines_fetch::stats_loader::resolve_player_id_by_name(name) else {
            continue;
        };
        // Find the player's team, then the day's game.
        let team = match player_team(pid) {
            Some(t) => t.to_uppercase(),
            None => continue,
        };
        let game = match slate.iter().find(|g| {
            g.away_abbrev.eq_ignore_ascii_case(&team) || g.home_abbrev.eq_ignore_ascii_case(&team)
        }) {
            Some(g) => g,
            None => continue,
        };
        let key =
            icelines_fetch::manifest::DataKey::Game(icelines_core::identity::GameId(game.game_id));
        // Foster +23 — lazy-fetch the boxscore body when it's
        // not on disk so users see real numbers without a
        // separate `icelines fetch boxscore` step. Persists
        // the body to the manifest as a side effect so the
        // TUI / CLI / next page-load all benefit. Failures
        // are non-fatal (drop to "no line").
        let raw_opt = match store.load_boxscore_raw(key.clone()) {
            Some(r) => Some(r),
            None => match client.fetch_boxscore_with_raw(game.game_id).await {
                Ok((_, raw_body)) => {
                    // Best-effort persist so subsequent renders
                    // don't re-hit the network. Same write
                    // pattern as `icelines fetch boxscore`.
                    let path = data_root
                        .join("boxscores")
                        .join(&today)
                        .join(format!("{}.json", game.game_id));
                    if let Ok(bytes) = serde_json::to_vec(&raw_body) {
                        let _ = icelines_fetch::atomic_write::write_bytes_atomic(&path, &bytes);
                        let _ = store.manifest().upsert(
                            icelines_fetch::manifest::DataKind::Boxscore,
                            icelines_fetch::manifest::ManifestEntry {
                                key: key.clone(),
                                path,
                                freshness: icelines_core::Freshness {
                                    fetched_at: chrono::Utc::now(),
                                    source: icelines_core::FetchSource::Live,
                                    ttl: icelines_core::Ttl::Static,
                                },
                            },
                        );
                    }
                    Some(raw_body)
                }
                Err(_) => None,
            },
        };
        let Some(raw) = raw_opt else { continue };
        let parsed = icelines_fetch::nhl_api::parse_boxscore(&raw, game.game_id);
        if let Some(line) =
            icelines_fetch::boxscore_to_night_line::extract_skater_line(&parsed, pid)
        {
            out.insert(name.clone(), format_skater_line_html(&line));
        }
    }
    out
}

fn player_team(pid: u32) -> Option<String> {
    for season in icelines_fetch::bundled::BUNDLED_SEASONS {
        if let Some(bios) = icelines_fetch::bundled::get_bios(season) {
            if let Some(b) = bios.iter().find(|b| b.player_id == pid) {
                if let Some(team) = &b.current_team_abbrev {
                    return Some(team.clone());
                }
            }
        }
    }
    None
}

fn format_skater_line_html(line: &icelines_core::favorites::SkaterNightLine) -> String {
    use icelines_core::favorites::{GameResult, HomeAway};
    let matchup = match line.home_or_away {
        HomeAway::Home => format!("{} vs {}", line.team.0, line.opponent.0),
        HomeAway::Away => format!("{} @ {}", line.team.0, line.opponent.0),
    };
    let result = match line.result {
        GameResult::Win => "W",
        GameResult::Loss => "L",
        GameResult::OtLoss => "OTL",
        GameResult::InProgress => "LIVE",
    };
    let toi = line
        .toi_seconds
        .map(|s| format!("{}:{:02}", s / 60, s % 60))
        .unwrap_or_else(|| "—".to_string());
    format!(
        "{} {}-{} {} · {}G {}A {}P · {:+} · TOI {} · {} SOG",
        matchup,
        line.team_score,
        line.opponent_score,
        result,
        line.goals,
        line.assists,
        line.points,
        line.plus_minus,
        toi,
        line.shots
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".into()),
    )
}

fn error_response(msg: &str) -> Response {
    let body = format!(
        "<!DOCTYPE html><html><body><h1>Favorites</h1><p>Error: {}</p></body></html>",
        html_escape(msg)
    );
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(body),
    )
        .into_response()
}

fn render_html(
    members: &[(String, String)],
    stat_lines: &std::collections::HashMap<String, String>,
) -> String {
    let player_count = members.iter().filter(|(k, _)| k == "player").count();
    let team_count = members.iter().filter(|(k, _)| k == "team").count();
    let mut body = String::new();
    body.push_str("<!DOCTYPE html><html><head>");
    body.push_str("<meta charset=\"utf-8\">");
    body.push_str("<title>Favorites — IceLines</title>");
    body.push_str("<link rel=\"stylesheet\" href=\"/static/style.css\">");
    body.push_str("<style>");
    body.push_str(
        ".fav-form { margin: 1rem 0; padding: 1rem; \
                 background: #f5f5f5; border-radius: 4px; } \
                 .fav-form input[type=text] { padding: 0.4rem; min-width: 18rem; } \
                 .fav-form button { padding: 0.4rem 0.9rem; cursor: pointer; } \
                 .fav-form .row { display: flex; gap: 0.5rem; \
                 align-items: center; margin: 0.4rem 0; flex-wrap: wrap; } \
                 .fav-list li { display: flex; gap: 0.6rem; \
                 align-items: center; margin: 0.2rem 0; } \
                 .fav-list .remove-btn { background: none; border: 1px solid #c00; \
                 color: #c00; padding: 0.1rem 0.5rem; border-radius: 3px; \
                 cursor: pointer; font-size: 0.85em; }",
    );
    body.push_str("</style>");
    body.push_str("</head><body>");
    body.push_str(
        "<nav><a href=\"/\">League</a> · <a href=\"/scores\">Scores</a> · \
                 <a href=\"/schedule\">Schedule</a> · <a href=\"/playoffs\">Playoffs</a> · \
                 <a href=\"/transactions\">Transactions</a> · \
                 <strong>Favorites</strong></nav>",
    );
    body.push_str("<main>");
    body.push_str("<h1>Favorites</h1>");
    body.push_str(&format!(
        "<p>{player_count} player(s), {team_count} team(s).</p>"
    ));

    // Add form — Foster +18. Auto-detects team-vs-player from
    // the input string (3-char ASCII abbrev → team).
    body.push_str(
        r##"<section class="fav-form">
  <h3 style="margin: 0 0 0.5rem 0;">Add to Favorites</h3>
  <form method="POST" action="/favorites/add">
    <div class="row">
      <label for="key">Player name or team abbrev:</label>
      <input type="text" id="key" name="key"
        placeholder="e.g. Connor McDavid · EDM · TOR" autofocus>
      <button type="submit">★ Add</button>
    </div>
    <p style="font-size: 0.85em; color: #666; margin: 0.4rem 0 0;">
      Auto-detects: 3-letter uppercase abbrevs route to teams; everything else is a player.
      Override with <code>kind=team</code> or <code>kind=player</code> below.
    </p>
    <div class="row">
      <label><input type="radio" name="kind" value=""> auto-detect</label>
      <label><input type="radio" name="kind" value="player"> player</label>
      <label><input type="radio" name="kind" value="team"> team</label>
    </div>
    <input type="hidden" name="return_to" value="/favorites">
  </form>
</section>"##,
    );

    if members.is_empty() {
        body.push_str("<section class=\"empty-state\">");
        body.push_str("<p><strong>No favorites yet.</strong> ");
        body.push_str("Use the form above, or run from the CLI:</p>");
        body.push_str(
            "<pre><code>icelines group add Favorites \"Connor McDavid\"\n\
                     icelines group add Favorites EDM</code></pre>",
        );
        body.push_str("</section>");
    } else {
        let players: Vec<&str> = members
            .iter()
            .filter(|(k, _)| k == "player")
            .map(|(_, v)| v.as_str())
            .collect();
        let teams: Vec<&str> = members
            .iter()
            .filter(|(k, _)| k == "team")
            .map(|(_, v)| v.as_str())
            .collect();
        if !players.is_empty() {
            body.push_str("<h2>Players</h2><ul class=\"fav-list\">");
            for p in players {
                let stat_line = stat_lines
                    .get(p)
                    .map(|l| {
                        format!(
                            "<br><span style=\"color:#444;font-size:0.92em;\">{}</span>",
                            html_escape(l)
                        )
                    })
                    .unwrap_or_default();
                body.push_str(&format!(
                    "<li><div><strong>{}</strong>{}</div>{}</li>",
                    html_escape(p),
                    stat_line,
                    remove_form(p, "player"),
                ));
            }
            body.push_str("</ul>");
        }
        if !teams.is_empty() {
            body.push_str("<h2>Teams</h2><ul class=\"fav-list\">");
            for t in teams {
                body.push_str(&format!(
                    "<li><a href=\"/team/{}\">{}</a>{}</li>",
                    html_escape(t),
                    html_escape(t),
                    remove_form(t, "team"),
                ));
            }
            body.push_str("</ul>");
        }
        body.push_str(
            "<p><em>Per-night stat lines + box scores wire in via \
                     <code>icelines fetch boxscore</code> (Foster.3+ orchestration).</em></p>",
        );
    }
    body.push_str("</main></body></html>");
    body
}

fn render_watchlist_html(
    members: &[(String, String)],
    notes: &std::collections::HashMap<String, WatchNote>,
    active_label: &str,
) -> String {
    let player_count = members.iter().filter(|(k, _)| k == "player").count();
    let team_count = members.iter().filter(|(k, _)| k == "team").count();
    let mut body = String::new();
    body.push_str("<!DOCTYPE html><html><head>");
    body.push_str("<meta charset=\"utf-8\">");
    body.push_str("<title>Watchlist - IceLines</title>");
    body.push_str("<link rel=\"stylesheet\" href=\"/static/style.css\">");
    body.push_str("</head><body>");
    body.push_str(
        "<nav><a href=\"/\">League</a> · <a href=\"/poach\">Poach</a> · \
                 <a href=\"/favorites\">Favorites</a> · <strong>Watchlist</strong></nav>",
    );
    body.push_str("<main>");
    body.push_str("<h1>Watchlist</h1>");
    body.push_str(&format!(
        "<p class=\"season-header\">{}</p>",
        html_escape(active_label)
    ));
    body.push_str(&format!(
        "<p>{player_count} player(s), {team_count} team(s).</p>"
    ));
    body.push_str(
        "<p>Toggle candidates from the TUI Poach board with <code>w</code>, \
                 or manage the group with <code>icelines group add Watchlist ...</code>.</p>",
    );

    if members.is_empty() {
        body.push_str("<section class=\"empty-state\">");
        body.push_str("<p><strong>No watched players yet.</strong></p>");
        body.push_str(
            "<pre><code>icelines tui poach\n\
                     icelines group add Watchlist \"Matthew Knies\"</code></pre>",
        );
        body.push_str("</section>");
    } else {
        let players: Vec<&str> = members
            .iter()
            .filter(|(k, _)| k == "player")
            .map(|(_, v)| v.as_str())
            .collect();
        let teams: Vec<&str> = members
            .iter()
            .filter(|(k, _)| k == "team")
            .map(|(_, v)| v.as_str())
            .collect();
        if !players.is_empty() {
            body.push_str("<h2>Players</h2><ul>");
            for player in players {
                let entity_ref = format!("player:{player}");
                let note = notes
                    .get(&entity_ref)
                    .map(|note| {
                        format!(
                            "<br><span style=\"color:#555;font-size:0.92em;\">why: {}</span>",
                            html_escape(&note.reason)
                        )
                    })
                    .unwrap_or_default();
                body.push_str(&format!("<li>{}{}</li>", html_escape(player), note));
            }
            body.push_str("</ul>");
        }
        if !teams.is_empty() {
            body.push_str("<h2>Teams</h2><ul>");
            for team in teams {
                body.push_str(&format!(
                    "<li><a href=\"/team/{}\">{}</a></li>",
                    html_escape(team),
                    html_escape(team)
                ));
            }
            body.push_str("</ul>");
        }
    }

    body.push_str("</main></body></html>");
    body
}

fn remove_form(key: &str, kind: &str) -> String {
    format!(
        r##"<form method="POST" action="/favorites/remove" style="display:inline;">
                    <input type="hidden" name="key" value="{}">
                    <input type="hidden" name="kind" value="{}">
                    <input type="hidden" name="return_to" value="/favorites">
                    <button type="submit" class="remove-btn" title="Remove from Favorites">×</button>
                </form>"##,
        html_escape(key),
        kind,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Foster +18 — POST handlers for add/remove ────────────────────

#[derive(Debug, Deserialize)]
pub struct FavoritesMutation {
    /// Free-text key — auto-detected as a team if it parses as
    /// a TeamAbbr, otherwise treated as a player name and
    /// normalized via `icelines_core::name::normalize_name`.
    /// Same auto-detect as the CLI `group add` path.
    pub key: String,
    /// Optional explicit kind override (`player` / `team`). When
    /// omitted, auto-detect runs.
    #[serde(default)]
    pub kind: Option<String>,
    /// Where to send the user after the mutation. Defaults to
    /// `/favorites`. Caller-supplied so each surface (team page,
    /// player card, favorites page itself) can route back to
    /// itself.
    #[serde(default)]
    pub return_to: Option<String>,
}

pub async fn post_add(headers: HeaderMap, Form(req): Form<FavoritesMutation>) -> Response {
    // Snapshot the resolved key + display name BEFORE mutate
    // so we can fire the career-history augment off in the
    // background after the redirect is queued. Augment is
    // best-effort + non-blocking from the user's POV — they
    // get the redirect immediately; the network call
    // completes off the request path.
    let display = req.key.trim().to_string();
    let kind_hint = req.kind.clone();
    let response = mutate(&headers, req, MutateOp::Add);
    // Foster +18 — opportunistic career-history augment for
    // newly-favorited players. Mirrors the CLI `group add`
    // behavior so favoriting from either surface populates
    // the local store identically. Skip on team adds.
    let is_player = match kind_hint.as_deref() {
        Some("team") => false,
        Some("player") => true,
        _ => icelines_core::TeamAbbr::parse(&display).is_err(),
    };
    if is_player && !display.is_empty() {
        let normalized = icelines_core::name::normalize_name(&display);
        tokio::spawn(async move {
            icelines_fetch::career_landing::augment_career_history_for_player(
                &display,
                &normalized,
                true,
            )
            .await;
        });
    }
    response
}

pub async fn post_remove(headers: HeaderMap, Form(req): Form<FavoritesMutation>) -> Response {
    mutate(&headers, req, MutateOp::Remove)
}

enum MutateOp {
    Add,
    Remove,
}

fn mutate(headers: &HeaderMap, req: FavoritesMutation, op: MutateOp) -> Response {
    let trimmed = req.key.trim();
    if trimmed.is_empty() {
        return error_response("Empty key — pass a player name or team abbrev.");
    }

    // Same auto-detect as the CLI: try TeamAbbr first; fall
    // back to normalized player name. Explicit `kind` wins.
    let (kind, key) = match req.kind.as_deref() {
        Some("team") => ("team", trimmed.to_uppercase()),
        Some("player") => ("player", icelines_core::name::normalize_name(trimmed)),
        _ => match icelines_core::TeamAbbr::parse(trimmed) {
            Ok(abbr) => ("team", abbr.0),
            Err(_) => ("player", icelines_core::name::normalize_name(trimmed)),
        },
    };
    let entity_ref = format!("{kind}:{key}");

    // Open the local db. Same path the CLI uses.
    let home = match std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        Some(h) => std::path::PathBuf::from(h),
        None => return error_response("HOME / USERPROFILE not set."),
    };
    let dir = home.join(".icelines");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return error_response(&format!("create {}: {e}", dir.display()));
    }
    let db_path = dir.join("icelines.db");
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("open db: {e}")),
    };
    // Make sure the schema is present — the GroupDb opens
    // first usually but the web server can be the first thing
    // the user runs. Best-effort; ignore failures.
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS groups (
                    name        TEXT PRIMARY KEY,
                    description TEXT NOT NULL DEFAULT '',
                    created_at  TEXT NOT NULL
                 );
                 INSERT OR IGNORE INTO groups (name, description, created_at) \
                    VALUES ('Favorites', '', datetime('now'));
                 CREATE TABLE IF NOT EXISTS group_members (
                    group_name TEXT NOT NULL,
                    entity_ref TEXT NOT NULL,
                    added_at   TEXT NOT NULL,
                    PRIMARY KEY (group_name, entity_ref)
                 );",
    );

    let result = match op {
        MutateOp::Add => conn.execute(
            "INSERT OR IGNORE INTO group_members \
                     (group_name, entity_ref, added_at) \
                     VALUES ('Favorites', ?1, datetime('now'))",
            rusqlite::params![entity_ref],
        ),
        MutateOp::Remove => conn.execute(
            "DELETE FROM group_members \
                     WHERE group_name = 'Favorites' AND entity_ref = ?1",
            rusqlite::params![entity_ref],
        ),
    };
    if let Err(e) = result {
        return error_response(&format!("db mutation: {e}"));
    }

    // 303 redirect to the caller-supplied return_to, defaulting
    // to /favorites. Validate the target is a relative path so
    // we don't act as an open-redirect.
    let dest = req
        .return_to
        .as_deref()
        .or_else(|| referer_path(headers))
        .filter(|p| p.starts_with('/') && !p.starts_with("//"))
        .unwrap_or("/favorites")
        .to_string();
    Redirect::to(&dest).into_response()
}

fn referer_path(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::REFERER)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            // Strip the scheme+host so we only return a relative
            // path. Anything we can't parse falls through to
            // /favorites via the unwrap_or above.
            if let Some(rest) = s.strip_prefix("http://") {
                rest.find('/').map(|i| &rest[i..])
            } else if let Some(rest) = s.strip_prefix("https://") {
                rest.find('/').map(|i| &rest[i..])
            } else if s.starts_with('/') {
                Some(s)
            } else {
                None
            }
        })
}
