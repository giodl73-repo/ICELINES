use crate::state::WebState;
use crate::templates::{CareerRow, PlayerTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;

/// Format a YYYYZZZZ season as "YYYY-YY" (e.g. 20242025 → "2024-25").
fn pretty_season(s: Season) -> String {
    let raw = s.0;
    if raw < 10_000_000 {
        return raw.to_string();
    }
    let yyyy_start = raw / 10_000;
    let yy_end = raw % 100;
    format!("{:04}-{:02}", yyyy_start, yy_end)
}

pub async fn get_player(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = match cfg.active_season_type.as_str() {
            "playoff" | "playoffs" => SeasonType::Playoff,
            _ => SeasonType::Regular,
        };
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return not_found_page(format!("Season '{season_str}' is not a valid YYYYZZZZ id"));
        }
    };
    let season = Season(season_u32);
    let pid = PlayerId(id);

    // King.3.2 — lazy career fan-out (UX.1 pattern). Brief
    // write lock loads all 38 bundled seasons for this pid
    // into the repo. Idempotent — re-opening the same player
    // is a ~5ms no-op aside from the bundle scans.
    // Per spec: subsequent reads are concurrent (RwLock).
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!(
                "warn: career fan-out for pid={id} failed: {e} — \
                         player card will show only seasons already loaded"
            );
        }
    }

    let projection = {
        let repo = state.repo.read().await;
        let identity = match repo.identity(pid) {
            Some(i) => i,
            None => {
                return not_found_page(format!(
                    "No player with NHL id {id} in the active repository. \
                             They may not have a row in the {season_str} season — \
                             try editing `~/.icelines/config.toml` to switch seasons."
                ));
            }
        };
        // Try the active season's view; fall back to None if
        // the player has no row that season (e.g. injured all
        // year, traded mid-season, retired).
        let view = repo.view(pid, season, season_type);

        // UX.B — pull the expanded stat slice. Each
        // pre-formatted to a String so the template renders
        // without inline casts and Option<> shows "—".
        let opt_u = |o: Option<u32>| -> String {
            match o {
                Some(n) => n.to_string(),
                None => "—".to_owned(),
            }
        };
        let opt_pct = |o: Option<f32>| -> String {
            match o {
                Some(p) => {
                    // NHL APIs report shooting/faceoff% as
                    // 0.105 (10.5%) — surface as percentage
                    // with one decimal so users see "10.5".
                    if p.abs() <= 1.5 {
                        format!("{:.1}%", p * 100.0)
                    } else {
                        format!("{:.1}%", p)
                    }
                }
                None => "—".to_owned(),
            }
        };
        let toi_mmss = |o: Option<u32>| -> String {
            match o {
                Some(secs) => {
                    let m = secs / 60;
                    let s = secs % 60;
                    format!("{m}:{s:02}")
                }
                None => "—".to_owned(),
            }
        };

        let (
            gp,
            goals,
            assists,
            points,
            position,
            team,
            team_link,
            plus_minus_str,
            pim_str,
            shots_str,
            shooting_pct_str,
            hits_str,
            blocks_str,
            takeaways_str,
            giveaways_str,
            faceoff_pct_str,
            pp_goals_str,
            pp_points_str,
            sh_goals_str,
            gwg_str,
            toi_per_game_str,
        ) = match view {
            Some(v) => {
                let totals = &v.stats.totals;
                let team_display = v.team_display().to_owned();
                // Only build a /team/ link when the display is
                // a single uppercase abbrev (skip the "TBL/CGY"
                // mid-season-trade format).
                let team_link = if team_display.chars().all(|c| c.is_ascii_alphabetic())
                    && team_display.len() <= 3
                {
                    team_display.clone()
                } else {
                    String::new()
                };
                (
                    v.gp(),
                    v.goals(),
                    v.assists(),
                    v.points(),
                    v.position().abbreviation().to_owned(),
                    team_display,
                    team_link,
                    format!("{:+}", v.plus_minus()),
                    totals.pim.to_string(),
                    totals.shots.to_string(),
                    opt_pct(totals.shooting_pct),
                    opt_u(v.hits()),
                    opt_u(v.blocked_shots()),
                    opt_u(v.takeaways()),
                    opt_u(v.giveaways()),
                    opt_pct(totals.faceoff_win_pct),
                    totals.pp_goals.to_string(),
                    totals.pp_points.to_string(),
                    totals.sh_goals.to_string(),
                    totals.gwg.to_string(),
                    toi_mmss(totals.toi_per_game_sec),
                )
            }
            None => (
                0,
                0,
                0,
                0,
                "—".to_owned(),
                "—".to_owned(),
                String::new(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
            ),
        };
        let ppg_str = if gp > 0 {
            format!("{:.2}", points as f64 / gp as f64)
        } else {
            String::new()
        };

        // King.3.2 — collect every (season, type) row this
        // player has stats for. Newest first. Skips empty
        // (gp=0) rows so a player who was rostered but never
        // played a regular-season game in a given (year,type)
        // doesn't add noise.
        //
        // UX.G — filter to the active season_type so the
        // career table matches what the global toggle says.
        // Mixing Regular + Playoff rows under a "Regular"
        // toggle was confusing.
        let mut career_rows: Vec<CareerRow> = match repo.career_all(pid) {
            Some(iter) => iter
                .filter(|s| s.season_type == season_type)
                .filter_map(|s| {
                    let totals = &s.totals;
                    if totals.gp == 0 {
                        return None;
                    }
                    let last_team = s
                        .team_stints
                        .last()
                        .map(|st| st.team.0.as_str().to_owned())
                        .unwrap_or_else(|| "—".to_owned());
                    // Link only when the team is a single
                    // 2-3 char alpha abbrev — multi-team
                    // values like "SEA/NYR" or sentinels
                    // like "—"/"RET" don't get a /team/ URL.
                    let team_link = if last_team.chars().all(|c| c.is_ascii_alphabetic())
                        && (2..=3).contains(&last_team.len())
                    {
                        last_team.clone()
                    } else {
                        String::new()
                    };
                    let ppg_str = if totals.gp > 0 {
                        format!("{:.2}", totals.points as f64 / totals.gp as f64)
                    } else {
                        String::new()
                    };
                    Some(CareerRow {
                        season: pretty_season(s.season),
                        season_type: match s.season_type {
                            SeasonType::Regular => "Regular".to_owned(),
                            SeasonType::Playoff => "Playoff".to_owned(),
                        },
                        team: last_team,
                        team_link,
                        gp: totals.gp,
                        goals: totals.goals,
                        assists: totals.assists,
                        points: totals.points,
                        ppg_str,
                    })
                })
                .collect(),
            None => Vec::new(),
        };
        // Newest season first; within a season, regular before playoff.
        career_rows.sort_by(|a, b| {
            b.season
                .cmp(&a.season)
                .then(a.season_type.cmp(&b.season_type))
        });

        // Sasq.3 — compute YoY delta against the prior season
        // of the SAME season-type (Regular vs Playoff).
        // career_rows is already filtered to active type and
        // sorted newest-first, so the prior season's row is
        // index 1 (index 0 is the active season we're showing).
        let prior_row = career_rows.get(1);
        let prior_season_label = prior_row
            .map(|r| format!("vs {}", r.season))
            .unwrap_or_default();

        fn delta_int(now: i64, prior: i64, prior_exists: bool) -> (String, String) {
            if !prior_exists {
                return (String::new(), String::new());
            }
            let d = now - prior;
            let class = if d > 0 {
                "delta-up"
            } else if d < 0 {
                "delta-down"
            } else {
                "delta-flat"
            };
            (format!("{:+}", d), class.to_owned())
        }

        let prior_exists = prior_row.is_some();
        let prior_gp = prior_row.map(|r| r.gp as i64).unwrap_or(0);
        let prior_goals = prior_row.map(|r| r.goals as i64).unwrap_or(0);
        let prior_assists = prior_row.map(|r| r.assists as i64).unwrap_or(0);
        let prior_points = prior_row.map(|r| r.points as i64).unwrap_or(0);
        let (gp_delta, gp_delta_class) = delta_int(gp as i64, prior_gp, prior_exists);
        let (goals_delta, goals_delta_class) = delta_int(goals as i64, prior_goals, prior_exists);
        let (assists_delta, assists_delta_class) =
            delta_int(assists as i64, prior_assists, prior_exists);
        let (points_delta, points_delta_class) =
            delta_int(points as i64, prior_points, prior_exists);

        PlayerTemplate {
            active_label: active_label.clone(),
            nhl_id: id,
            full_name: identity.full_name.clone(),
            position,
            team,
            team_link: team_link.clone(),
            // Prefer the seasonal team-keyed CDN path (real
            // mug shot for current rosters); fall back to the
            // legacy `default/{id}.png` (silhouette for many
            // players) only when we don't have a team to key
            // by.
            headshot_url: if !team_link.is_empty() {
                Some(super::shared::build_headshot_url(season.0, &team_link, id))
            } else {
                identity.headshot_canonical_url.clone()
            },
            gp,
            goals,
            assists,
            points,
            ppg_str,
            plus_minus_str,
            pim_str,
            shots_str,
            shooting_pct_str,
            hits_str,
            blocks_str,
            takeaways_str,
            giveaways_str,
            faceoff_pct_str,
            pp_goals_str,
            pp_points_str,
            sh_goals_str,
            gwg_str,
            toi_per_game_str,
            goals_delta,
            goals_delta_class,
            assists_delta,
            assists_delta_class,
            points_delta,
            points_delta_class,
            gp_delta,
            gp_delta_class,
            prior_season_label,
            career_rows,
            // Phase Calder.3 — pre-NHL career rows for the
            // template. Loaded from the local store and
            // pre-formatted into PreNhlRow strings so askama
            // doesn't have to do float-to-string casts.
            pre_nhl_career: {
                let store = icelines_fetch::career_landing::load_local_store();
                let stints = store
                    .get(id)
                    .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
                    .unwrap_or_default();
                crate::templates::project_pre_nhl_html_rows(&stints)
            },
            // UX.H — every active player + goalie name in
            // the repo, sorted alphabetically. Renders as a
            // <datalist> on the page so the Compare-with
            // input gets native browser autocomplete with
            // zero JS. Skips the player you're already
            // viewing — comparing someone with themselves is
            // never useful.
            compare_suggestions: {
                let mut pairs: Vec<(String, u32)> = repo
                    .iter_identities()
                    .filter(|i| i.id.0 != pid.0)
                    .map(|i| (i.full_name.clone(), i.id.0))
                    .collect();
                pairs.sort_by(|a, b| a.0.cmp(&b.0));
                pairs
            },
        }
    };

    match projection.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

fn not_found_page(msg: String) -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(format!(
            "<!doctype html><html><body>\
                     <h1>Player not found</h1>\
                     <p>{msg}</p>\
                     <p><a href=\"/leaders\">← back to leaders</a></p>\
                     </body></html>"
        )),
    )
        .into_response()
}

// ── King.3.3 — JSON twin ──────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct PlayerData {
    pub nhl_id: u32,
    pub full_name: String,
    pub position: String,
    pub team: String,
    pub headshot_url: Option<String>,
    pub active_season_stats: PlayerActiveStats,
    pub career: Vec<PlayerCareerRow>,
    /// Phase Calder.3 — pre-NHL career stints (junior / NCAA /
    /// AHL / European pro). Empty when the user hasn't run
    /// `icelines fetch career` to populate the local store.
    pub pre_nhl_career: Vec<PreNhlStint>,
}

/// Phase Calder.3 — one pre-NHL stint for the JSON twin.
/// Mirrors `icelines_core::career_history::CareerStint` but
/// flattened to the fields the player card actually shows.
#[derive(Debug, serde::Serialize)]
pub struct PreNhlStint {
    pub season: String,
    pub league: String,
    pub league_tier: &'static str,
    pub team: String,
    pub games: u32,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct PlayerActiveStats {
    pub season: String,
    pub season_type: String,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct PlayerCareerRow {
    pub season: String,
    pub season_type: String,
    pub team: String,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct PlayerMeta {
    pub season: String,
    pub season_type: String,
    pub career_rows: usize,
    /// Phase Calder.3 — count of pre-NHL stints surfaced.
    pub pre_nhl_career_rows: usize,
}

/// Phase Calder.3 — load pre-NHL career stints for one player
/// from the local store at `~/.icelines/career_history.json`.
/// Returns an empty Vec if the store doesn't exist yet (the
/// user can run `icelines fetch career` to populate). Same
/// filtering as the CLI: drops NHL stints, drops international
/// tournaments, drops youth/minor — keeps Pro/Junior/College
/// development arc, regular season only.
pub(crate) fn project_pre_nhl_rows(
    stints: &[icelines_core::career_history::CareerStint],
) -> Vec<PreNhlStint> {
    use icelines_core::career_history::LeagueTier;
    stints
        .iter()
        .map(|s| PreNhlStint {
            season: s.season.to_string(),
            league: s.league.0.clone(),
            league_tier: match s.league.tier() {
                LeagueTier::Pro => "pro",
                LeagueTier::Junior => "junior",
                LeagueTier::College => "college",
                LeagueTier::International => "international",
                LeagueTier::Other => "other",
            },
            team: s.team.clone(),
            games: s.gp,
            goals: s.goals,
            assists: s.assists,
            points: s.points,
            points_per_game: s.points_per_game().map(|p| p as f64),
        })
        .collect()
}

/// `GET /api/v1/player/:id` — JSON twin of `/player/:id`.
///
/// Same load + projection path as the HTML handler. Errors for
/// unknown id collapse into a 404 JSON body (axum default body
/// is fine — clients should branch on status code).
pub async fn get_player_json(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        let st = match cfg.active_season_type.as_str() {
            "playoff" | "playoffs" => SeasonType::Playoff,
            _ => SeasonType::Regular,
        };
        (cfg.active_season.clone(), st)
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "bad_active_season",
                    "message": format!("Season '{season_str}' is not a valid YYYYZZZZ id"),
                })),
            )
                .into_response();
        }
    };
    let season = Season(season_u32);
    let pid = PlayerId(id);

    // Mirror the HTML handler's lazy career fan-out.
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!(
                "warn: career fan-out for pid={id} failed: {e} — \
                         /api/v1/player/:id will return only seasons already loaded"
            );
        }
    }

    let repo = state.repo.read().await;
    let identity = match repo.identity(pid) {
        Some(i) => i,
        None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": "player_not_found",
                    "message": format!(
                        "No player with NHL id {id} in the active repository."
                    ),
                    "nhl_id": id,
                })),
            )
                .into_response();
        }
    };

    let (gp, goals, assists, points, position, team) = match repo.view(pid, season, season_type) {
        Some(v) => (
            v.gp(),
            v.goals(),
            v.assists(),
            v.points(),
            v.position().abbreviation().to_owned(),
            v.team_display().to_owned(),
        ),
        None => (0, 0, 0, 0, String::new(), String::new()),
    };
    let ppg = if gp > 0 {
        Some((points as f64) / (gp as f64))
    } else {
        None
    };

    let mut career: Vec<PlayerCareerRow> = match repo.career_all(pid) {
        Some(iter) => iter
            .filter_map(|s| {
                let totals = &s.totals;
                if totals.gp == 0 {
                    return None;
                }
                let last_team = s
                    .team_stints
                    .last()
                    .map(|st| st.team.0.as_str().to_owned())
                    .unwrap_or_default();
                let ppg = if totals.gp > 0 {
                    Some((totals.points as f64) / (totals.gp as f64))
                } else {
                    None
                };
                Some(PlayerCareerRow {
                    season: pretty_season(s.season),
                    season_type: match s.season_type {
                        SeasonType::Regular => "regular".to_owned(),
                        SeasonType::Playoff => "playoff".to_owned(),
                    },
                    team: last_team,
                    games: totals.gp,
                    goals: totals.goals,
                    assists: totals.assists,
                    points: totals.points,
                    points_per_game: ppg,
                })
            })
            .collect(),
        None => Vec::new(),
    };
    career.sort_by(|a, b| {
        b.season
            .cmp(&a.season)
            .then(a.season_type.cmp(&b.season_type))
    });
    let career_rows_n = career.len();

    let pre_nhl_stints = {
        let store = icelines_fetch::career_landing::load_local_store();
        store
            .get(id)
            .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
            .unwrap_or_default()
    };
    let pre_nhl_career = project_pre_nhl_rows(&pre_nhl_stints);
    let pre_nhl_career_rows = pre_nhl_career.len();

    let data = PlayerData {
        nhl_id: id,
        full_name: identity.full_name.clone(),
        position,
        team,
        headshot_url: identity.headshot_canonical_url.clone(),
        active_season_stats: PlayerActiveStats {
            season: season_str.clone(),
            season_type: match season_type {
                SeasonType::Regular => "regular".to_owned(),
                SeasonType::Playoff => "playoff".to_owned(),
            },
            games: gp,
            goals,
            assists,
            points,
            points_per_game: ppg,
        },
        career,
        pre_nhl_career,
    };
    let meta = PlayerMeta {
        season: season_str,
        season_type: match season_type {
            SeasonType::Regular => "regular".to_owned(),
            SeasonType::Playoff => "playoff".to_owned(),
        },
        career_rows: career_rows_n,
        pre_nhl_career_rows,
    };
    crate::api::json_data_meta("player", data, meta)
}
