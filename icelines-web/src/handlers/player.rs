use crate::state::WebState;
use crate::templates::{CareerRow, PlayerTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    MetricCell, MetricValue, PlayerCardView, PlayerCareerSummary, PlayerSeasonSummary,
};

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
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
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

    let (view, compare_suggestions) = {
        let repo = state.repo.read().await;
        let view = match PlayerCardView::from_repository(&repo, pid, season, season_type) {
            Some(view) => view,
            None => {
                return not_found_page(format!(
                    "No player with NHL id {id} in the active repository. \
                             They may not have a row in the {season_str} season — \
                             try editing `~/.icelines/config.toml` to switch seasons."
                ));
            }
        };

        let mut compare_suggestions: Vec<(String, u32)> = repo
            .iter_identities()
            .filter(|i| i.id.0 != pid.0)
            .map(|i| (i.full_name.clone(), i.id.0))
            .collect();
        compare_suggestions.sort_by(|a, b| a.0.cmp(&b.0));
        (view, compare_suggestions)
    };
    let projection = player_template_from_view(
        view,
        active_label,
        id,
        season,
        season_type,
        compare_suggestions,
    );

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

// Shared player page projectors.

fn player_template_from_view(
    view: PlayerCardView,
    active_label: String,
    id: u32,
    season: Season,
    season_type: SeasonType,
    compare_suggestions: Vec<(String, u32)>,
) -> PlayerTemplate {
    let active = view.active.as_ref();
    let (gp, goals, assists, points, position, team, team_link) = active_summary(active);
    let active_metrics = active
        .map(|active| active.metrics.as_slice())
        .unwrap_or(&[]);
    let ppg_str = metric_f64(active_metrics, "points_per_game")
        .map(|ppg| format!("{ppg:.2}"))
        .unwrap_or_default();

    let career_rows: Vec<CareerRow> = view
        .career
        .iter()
        .filter(|row| row.season_type == season_type)
        .map(career_row_from_view_for_html)
        .collect();

    let prior_row = career_rows.get(1);
    let prior_season_label = prior_row
        .map(|row| format!("vs {}", row.season))
        .unwrap_or_default();
    let prior_exists = prior_row.is_some();
    let prior_gp = prior_row.map(|row| row.gp as i64).unwrap_or(0);
    let prior_goals = prior_row.map(|row| row.goals as i64).unwrap_or(0);
    let prior_assists = prior_row.map(|row| row.assists as i64).unwrap_or(0);
    let prior_points = prior_row.map(|row| row.points as i64).unwrap_or(0);
    let (gp_delta, gp_delta_class) = delta_int(gp as i64, prior_gp, prior_exists);
    let (goals_delta, goals_delta_class) = delta_int(goals as i64, prior_goals, prior_exists);
    let (assists_delta, assists_delta_class) =
        delta_int(assists as i64, prior_assists, prior_exists);
    let (points_delta, points_delta_class) = delta_int(points as i64, prior_points, prior_exists);

    let pre_nhl_career = {
        let store = icelines_fetch::career_landing::load_local_store();
        let stints = store
            .get(id)
            .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
            .unwrap_or_default();
        crate::templates::project_pre_nhl_html_rows(&stints)
    };

    PlayerTemplate {
        active_label,
        nhl_id: id,
        full_name: view.display_name,
        position,
        team,
        team_link: team_link.clone(),
        headshot_url: if !team_link.is_empty() {
            Some(super::shared::build_headshot_url(season.0, &team_link, id))
        } else {
            view.headshot_url
        },
        gp,
        goals,
        assists,
        points,
        ppg_str,
        plus_minus_str: metric_i32(active_metrics, "plus_minus")
            .map(|value| format!("{value:+}"))
            .unwrap_or_else(dash),
        pim_str: metric_string_u32_or_dash(active_metrics, "pim"),
        shots_str: metric_string_u32_or_dash(active_metrics, "shots"),
        shooting_pct_str: metric_percent_string(active_metrics, "shooting_pct"),
        hits_str: metric_string_u32_or_dash(active_metrics, "hits"),
        blocks_str: metric_string_u32_or_dash(active_metrics, "blocks"),
        takeaways_str: metric_string_u32_or_dash(active_metrics, "takeaways"),
        giveaways_str: metric_string_u32_or_dash(active_metrics, "giveaways"),
        faceoff_pct_str: metric_percent_string(active_metrics, "faceoff_win_pct"),
        pp_goals_str: metric_string_u32_or_dash(active_metrics, "pp_goals"),
        pp_points_str: metric_string_u32_or_dash(active_metrics, "pp_points"),
        sh_goals_str: metric_string_u32_or_dash(active_metrics, "sh_goals"),
        gwg_str: metric_string_u32_or_dash(active_metrics, "gwg"),
        toi_per_game_str: metric_toi_mmss(active_metrics, "toi_per_game_sec"),
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
        pre_nhl_career,
        compare_suggestions,
    }
}

fn active_summary(
    active: Option<&PlayerSeasonSummary>,
) -> (u32, u32, u32, u32, String, String, String) {
    match active {
        Some(active) => {
            let gp = metric_u32(&active.metrics, "gp").unwrap_or(0);
            let goals = metric_u32(&active.metrics, "goals").unwrap_or(0);
            let assists = metric_u32(&active.metrics, "assists").unwrap_or(0);
            let points = metric_u32(&active.metrics, "points").unwrap_or(0);
            let team_link = team_link_for_display(&active.team_display);
            (
                gp,
                goals,
                assists,
                points,
                active.position.abbreviation().to_owned(),
                active.team_display.clone(),
                team_link,
            )
        }
        None => (0, 0, 0, 0, dash(), dash(), String::new()),
    }
}

fn career_row_from_view_for_html(row: &PlayerCareerSummary) -> CareerRow {
    let gp = metric_u32(&row.metrics, "gp").unwrap_or(0);
    let points = metric_u32(&row.metrics, "points").unwrap_or(0);
    CareerRow {
        season: pretty_season(row.season),
        season_type: season_type_title(row.season_type).to_owned(),
        team: row.team.0.clone(),
        team_link: team_link_for_display(&row.team.0),
        gp,
        goals: metric_u32(&row.metrics, "goals").unwrap_or(0),
        assists: metric_u32(&row.metrics, "assists").unwrap_or(0),
        points,
        ppg_str: if gp > 0 {
            format!("{:.2}", points as f64 / gp as f64)
        } else {
            String::new()
        },
    }
}

fn team_link_for_display(team: &str) -> String {
    if team.chars().all(|c| c.is_ascii_alphabetic()) && (2..=3).contains(&team.len()) {
        team.to_owned()
    } else {
        String::new()
    }
}

fn delta_int(now: i64, prior: i64, prior_exists: bool) -> (String, String) {
    if !prior_exists {
        return (String::new(), String::new());
    }
    let delta = now - prior;
    let class = if delta > 0 {
        "delta-up"
    } else if delta < 0 {
        "delta-down"
    } else {
        "delta-flat"
    };
    (format!("{delta:+}"), class.to_owned())
}

fn metric_string_u32_or_dash(metrics: &[MetricCell], key: &str) -> String {
    metric_u32(metrics, key)
        .map(|value| value.to_string())
        .unwrap_or_else(dash)
}

fn metric_percent_string(metrics: &[MetricCell], key: &str) -> String {
    metric_f64(metrics, key)
        .map(|value| {
            if value.abs() <= 1.5 {
                format!("{:.1}%", value * 100.0)
            } else {
                format!("{value:.1}%")
            }
        })
        .unwrap_or_else(dash)
}

fn metric_toi_mmss(metrics: &[MetricCell], key: &str) -> String {
    metric_u32(metrics, key)
        .map(|secs| format!("{}:{:02}", secs / 60, secs % 60))
        .unwrap_or_else(dash)
}

fn dash() -> String {
    "\u{2014}".to_owned()
}

// JSON twin types.
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
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
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

    let view = {
        let repo = state.repo.read().await;
        match PlayerCardView::from_repository(&repo, pid, season, season_type) {
            Some(view) => view,
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
        }
    };

    let active = view.active.as_ref();
    let (gp, goals, assists, points, position, team) = match active {
        Some(active) => (
            metric_u32(&active.metrics, "gp").unwrap_or(0),
            metric_u32(&active.metrics, "goals").unwrap_or(0),
            metric_u32(&active.metrics, "assists").unwrap_or(0),
            metric_u32(&active.metrics, "points").unwrap_or(0),
            active.position.abbreviation().to_owned(),
            active.team_display.clone(),
        ),
        None => (0, 0, 0, 0, String::new(), String::new()),
    };
    let ppg = metric_f64(
        active
            .map(|active| active.metrics.as_slice())
            .unwrap_or(&[]),
        "points_per_game",
    );
    let career: Vec<PlayerCareerRow> = view
        .career
        .iter()
        .map(player_career_row_from_view)
        .collect();
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
        full_name: view.display_name.clone(),
        position,
        team,
        headshot_url: view.headshot_url.clone(),
        active_season_stats: PlayerActiveStats {
            season: season_str.clone(),
            season_type: season_type.label().to_owned(),
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
        season_type: season_type.label().to_owned(),
        career_rows: career_rows_n,
        pre_nhl_career_rows,
    };
    crate::api::json_data_meta("player", data, meta)
}

fn player_career_row_from_view(row: &PlayerCareerSummary) -> PlayerCareerRow {
    PlayerCareerRow {
        season: pretty_season(row.season),
        season_type: row.season_type.label().to_owned(),
        team: row.team.0.clone(),
        games: metric_u32(&row.metrics, "gp").unwrap_or(0),
        goals: metric_u32(&row.metrics, "goals").unwrap_or(0),
        assists: metric_u32(&row.metrics, "assists").unwrap_or(0),
        points: metric_u32(&row.metrics, "points").unwrap_or(0),
        points_per_game: metric_f64(&row.metrics, "points_per_game"),
    }
}

fn season_type_title(season_type: SeasonType) -> &'static str {
    match season_type {
        SeasonType::Regular => "Regular",
        SeasonType::Playoff => "Playoff",
    }
}

fn metric_u32(metrics: &[MetricCell], key: &str) -> Option<u32> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => u32::try_from(value).ok(),
            _ => None,
        })
}

fn metric_i32(metrics: &[MetricCell], key: &str) -> Option<i32> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => i32::try_from(value).ok(),
            _ => None,
        })
}

fn metric_f64(metrics: &[MetricCell], key: &str) -> Option<f64> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            _ => None,
        })
}
