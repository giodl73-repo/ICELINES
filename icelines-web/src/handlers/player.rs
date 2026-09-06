use crate::state::WebState;
use crate::templates::{CareerRow, PlayerTemplate};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::StatsRepository;
use icelines_core::{
    MetricCell, MetricValue, PlayerCardView, PlayerCareerSummary, PlayerPreNhlCareerRow,
    PlayerSeasonSummary,
};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct PlayerByNameQuery {
    team: Option<String>,
}

/// Resolve a human-readable player name to the canonical NHL-id player card.
///
/// This bookmarkable adapter is intended for reports and other IceLines
/// consumers that retain canonical player names but do not duplicate NHL IDs.
/// Exact normalized names redirect immediately. Duplicate names require a team
/// hint or render a choice page instead of silently selecting the wrong player.
pub async fn get_player_by_name(
    Path(name): Path<String>,
    Query(query): Query<PlayerByNameQuery>,
) -> Response {
    let normalized_name = icelines_core::name::normalize_name(&name);
    let mut candidates: Vec<_> = icelines_fetch::stats_loader::find_player_candidates(&name)
        .into_iter()
        .filter(|candidate| {
            icelines_core::name::normalize_name(&candidate.full_name) == normalized_name
        })
        .collect();

    if candidates.len() > 1 {
        if let Some(team) = query.team.as_deref().map(normalize_team_hint) {
            let team_matches: Vec<_> = candidates
                .iter()
                .filter(|candidate| candidate.last_team.as_deref() == Some(team.as_str()))
                .collect();
            if let [candidate] = team_matches.as_slice() {
                return Redirect::temporary(&format!("/player/{}", candidate.pid)).into_response();
            }
        }
    }

    match candidates.as_slice() {
        [candidate] => Redirect::temporary(&format!("/player/{}", candidate.pid)).into_response(),
        [] => (
            StatusCode::NOT_FOUND,
            Html(format!(
                "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Player not found</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>Player not found</h1><p>IceLines could not resolve <strong>{}</strong> to one canonical NHL player.</p><p><a href=\"/leaders\">Browse player leaders</a></p></main></body></html>",
                html_escape(&name)
            )),
        )
            .into_response(),
        _ => {
            candidates.sort_by(|left, right| {
                left.full_name
                    .cmp(&right.full_name)
                    .then_with(|| left.pid.cmp(&right.pid))
            });
            let choices = candidates
                .iter()
                .map(|candidate| {
                    let team = candidate.last_team.as_deref().unwrap_or("team unavailable");
                    format!(
                        "<li><a href=\"/player/{}\">{} — {}</a></li>",
                        candidate.pid,
                        html_escape(&candidate.full_name),
                        html_escape(team)
                    )
                })
                .collect::<String>();
            (
                StatusCode::MULTIPLE_CHOICES,
                Html(format!(
                    "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Choose player</title><link rel=\"stylesheet\" href=\"/static/style.css\"></head><body><a href=\"#main\" class=\"skip-link\">Skip to content</a><main id=\"main\" tabindex=\"-1\"><h1>Choose player</h1><p>More than one NHL player matches <strong>{}</strong>.</p><ul>{choices}</ul></main></body></html>",
                    html_escape(&name)
                )),
            )
                .into_response()
        }
    }
}

fn normalize_team_hint(team: &str) -> String {
    match team.trim().to_ascii_uppercase().as_str() {
        "LA" => "LAK".to_owned(),
        "NJ" => "NJD".to_owned(),
        "SJ" => "SJS".to_owned(),
        "TB" => "TBL".to_owned(),
        other => other.to_owned(),
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

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
    match build_player_template(&state, id).await {
        Ok(projection) => match projection.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("template render failed: {e}")),
            )
                .into_response(),
        },
        Err(response) => response,
    }
}

pub async fn build_player_template(state: &WebState, id: u32) -> Result<PlayerTemplate, Response> {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return Err(not_found_page(format!(
                "Season '{season_str}' is not a valid YYYYZZZZ id"
            )));
        }
    };
    let season = Season(season_u32);
    let pid = PlayerId(id);

    let (view, compare_suggestions) = {
        let repo = state.repo.read().await;
        let mut local_repo = player_local_repo(&repo, pid);
        if let Err(e) =
            icelines_fetch::stats_loader::load_player_career_into_repo(&mut local_repo, pid)
        {
            eprintln!(
                "warn: career fan-out for pid={id} failed: {e} — \
                         player card will show only seasons already loaded"
            );
        }
        let view = match PlayerCardView::from_repository(&local_repo, pid, season, season_type) {
            Some(view) => view,
            None => {
                return Err(not_found_page(format!(
                    "No player with NHL id {id} in the active repository. \
                             They may not have a row in the {season_str} season — \
                             try editing `~/.icelines/config.toml` to switch seasons."
                )));
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
    Ok(player_template_from_view(
        view,
        active_label,
        id,
        season,
        season_type,
        compare_suggestions,
    ))
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

fn player_local_repo(shared: &StatsRepository, pid: PlayerId) -> StatsRepository {
    let mut local = StatsRepository::with_lru_cap(shared.lru_cap());
    if let Some(identity) = shared.identity(pid) {
        if let Err(err) = local.upsert_identity(identity.clone()) {
            eprintln!(
                "warn: local player repo identity merge for pid={} failed: {err}",
                pid.0
            );
        }
    }
    if let Some(contract) = shared.contract(pid) {
        local.upsert_contract(pid, contract.clone());
    }
    for stats in shared.iter_stats().filter(|stats| stats.player_id == pid) {
        if let Err(err) = local.upsert_stats(stats.clone()) {
            eprintln!(
                "warn: local player repo stat copy for pid={} failed: {err}",
                pid.0
            );
        }
    }
    local
}

// Shared player page projectors.

fn player_template_from_view(
    mut view: PlayerCardView,
    active_label: String,
    id: u32,
    season: Season,
    season_type: SeasonType,
    compare_suggestions: Vec<(String, u32)>,
) -> PlayerTemplate {
    let pre_nhl_stints = {
        let store = icelines_fetch::career_landing::load_local_store();
        store
            .get(id)
            .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
            .unwrap_or_default()
    };
    view = view.with_pre_nhl_stints(&pre_nhl_stints);

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

    let pre_nhl_career = crate::templates::project_pre_nhl_html_rows(&pre_nhl_stints);
    let career_trend_svg = render_player_career_trend_svg(&view);

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
        headshot_fallback_url: format!("https://assets.nhle.com/mugs/nhl/default/{id}.png"),
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
        career_trend_svg,
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

fn render_player_career_trend_svg(view: &PlayerCardView) -> Option<String> {
    let points = career_points_per_82_chronological(&view.career);
    if points.len() < 2 {
        return None;
    }

    let mut values: Vec<f64> = points.iter().map(|(_, value)| *value).collect();
    values.sort_by(f64::total_cmp);
    let min = *values.first()?;
    let max = *values.last()?;
    let range = if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        max - min
    };

    let path = svg_polyline_points(&points, min, range);
    let first = points.first()?.0.as_str();
    let last = points.last()?.0.as_str();
    let latest = points.last()?.1;
    let name = escape_svg_text(&view.display_name);

    Some(format!(
        r##"<svg class="player-career-trend-svg" viewBox="0 0 640 240" role="img" aria-labelledby="player-career-trend-title player-career-trend-desc">
  <title id="player-career-trend-title">Pts/82 career trend</title>
  <desc id="player-career-trend-desc">{name} regular-season career trend from {first} to {last}. Values are bundled points per 82 games.</desc>
  <rect x="0" y="0" width="640" height="240" rx="8" fill="#f8fafc"/>
  <line x1="58" y1="188" x2="600" y2="188" stroke="#cbd5e1"/>
  <line x1="58" y1="40" x2="58" y2="188" stroke="#cbd5e1"/>
  <text x="58" y="26" fill="#334155" font-size="13">Pts/82 career trend</text>
  <text x="58" y="212" fill="#64748b" font-size="11">older</text>
  <text x="560" y="212" fill="#64748b" font-size="11">latest</text>
  <polyline points="{path}" fill="none" stroke="#0f766e" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
  <circle cx="600" cy="58" r="5" fill="#0f766e"/>
  <text x="612" y="63" fill="#0f172a" font-size="12">{name} {latest:.1}</text>
</svg>"##
    ))
}

fn career_points_per_82_chronological(career: &[PlayerCareerSummary]) -> Vec<(String, f64)> {
    let mut rows: Vec<(u32, String, f64)> = career
        .iter()
        .filter(|row| row.season_type == SeasonType::Regular)
        .filter_map(|row| {
            let gp = metric_u32(&row.metrics, "gp")?;
            if gp == 0 {
                return None;
            }
            let points = metric_u32(&row.metrics, "points")?;
            Some((
                row.season.0,
                pretty_season(row.season),
                f64::from(points) * 82.0 / f64::from(gp),
            ))
        })
        .collect();
    rows.sort_by_key(|(season, _, _)| *season);
    rows.into_iter()
        .map(|(_, label, points)| (label, points))
        .collect()
}

fn svg_polyline_points(points: &[(String, f64)], min: f64, range: f64) -> String {
    let width = 542.0;
    let height = 148.0;
    let x0 = 58.0;
    let y0 = 188.0;
    let denom = (points.len() - 1) as f64;
    points
        .iter()
        .enumerate()
        .map(|(idx, (_, value))| {
            let x = x0 + (idx as f64 / denom) * width;
            let y = y0 - ((*value - min) / range) * height;
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_svg_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

impl From<&PlayerPreNhlCareerRow> for PreNhlStint {
    fn from(row: &PlayerPreNhlCareerRow) -> Self {
        Self {
            season: row.season.to_string(),
            league: row.league.clone(),
            league_tier: match row.league_tier.as_str() {
                "pro" => "pro",
                "junior" => "junior",
                "college" => "college",
                "international" => "international",
                _ => "other",
            },
            team: row.team.clone(),
            games: row.games,
            goals: row.goals,
            assists: row.assists,
            points: row.points,
            points_per_game: row.points_per_game.map(f64::from),
        }
    }
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

#[derive(Debug, serde::Serialize)]
struct PlayerErrorData {
    nhl_id: u32,
}

fn player_error_meta(season: &str, season_type: SeasonType) -> PlayerMeta {
    PlayerMeta {
        season: season.to_owned(),
        season_type: season_type.label().to_owned(),
        career_rows: 0,
        pre_nhl_career_rows: 0,
    }
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
            return crate::api::json_error_meta(
                StatusCode::BAD_REQUEST,
                "player",
                PlayerErrorData { nhl_id: id },
                player_error_meta(&season_str, season_type),
                format!("Season '{season_str}' is not a valid YYYYZZZZ id"),
            );
        }
    };
    let season = Season(season_u32);
    let pid = PlayerId(id);

    let mut view = {
        let repo = state.repo.read().await;
        let mut local_repo = player_local_repo(&repo, pid);
        if let Err(e) =
            icelines_fetch::stats_loader::load_player_career_into_repo(&mut local_repo, pid)
        {
            eprintln!(
                "warn: career fan-out for pid={id} failed: {e} — \
                         /api/v1/player/:id will return only seasons already loaded"
            );
        }
        match PlayerCardView::from_repository(&local_repo, pid, season, season_type) {
            Some(view) => view,
            None => {
                return crate::api::json_error_meta(
                    StatusCode::NOT_FOUND,
                    "player",
                    PlayerErrorData { nhl_id: id },
                    player_error_meta(&season_str, season_type),
                    format!("No player with NHL id {id} in the active repository."),
                );
            }
        }
    };

    let pre_nhl_stints = {
        let store = icelines_fetch::career_landing::load_local_store();
        store
            .get(id)
            .map(icelines_fetch::career_landing::extract_pre_nhl_stints)
            .unwrap_or_default()
    };
    view = view.with_pre_nhl_stints(&pre_nhl_stints);

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

    let pre_nhl_career: Vec<PreNhlStint> = view.pre_nhl_career.iter().map(Into::into).collect();
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
