use crate::state::WebState;
use crate::templates::{RecordsTemplate, RecordsTemplateRow};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    PlayerRecordsView, RecordsOpponentRow, TeamRecordsView, ViewContext, ViewWindow,
};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct RecordsQuery {
    metric: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum PlayerWebRecordsMetric {
    TeamsScoredAgainst,
    GoaliesScoredAgainst,
    FightOpponents,
}

impl PlayerWebRecordsMetric {
    const ALLOWED: &'static [&'static str] = &[
        "teams-scored-against",
        "goalies-scored-against",
        "fight-opponents",
    ];

    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("teams-scored-against") {
            "teams-scored-against" => Ok(Self::TeamsScoredAgainst),
            "goalies-scored-against" => Ok(Self::GoaliesScoredAgainst),
            "fight-opponents" => Ok(Self::FightOpponents),
            other => Err(other.to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "teams-scored-against",
            Self::GoaliesScoredAgainst => "goalies-scored-against",
            Self::FightOpponents => "fight-opponents",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "NHL teams scored against",
            Self::GoaliesScoredAgainst => "NHL goalies scored against",
            Self::FightOpponents => "Fight opponents",
        }
    }

    fn subject_label(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "opponent team",
            Self::GoaliesScoredAgainst => "goalie",
            Self::FightOpponents => "opponent",
        }
    }

    fn empty_hint(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => {
                "No matching game records are loaded yet. Load active-season game lines from the NHL API, then this page will read them from the local cache."
            }
            Self::GoaliesScoredAgainst => {
                "No goalie matchup records are loaded yet. Load active-season play-by-play from the NHL API, then this page will read it from the local cache."
            }
            Self::FightOpponents => {
                "No fight records are loaded yet. Load active-season play-by-play from the NHL API, then this page will read it from the local cache."
            }
        }
    }

    fn cache_artifacts(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "boxscore",
            Self::GoaliesScoredAgainst | Self::FightOpponents => "play-by-play",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TeamWebRecordsMetric {
    PlayersScored,
    GoaliesBeaten,
    FightOpponents,
}

impl TeamWebRecordsMetric {
    const ALLOWED: &'static [&'static str] = &[
        "players-scored-against-team",
        "goalies-beaten-by-team",
        "fight-opponents-by-team",
    ];

    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("players-scored-against-team") {
            "players-scored-against-team" => Ok(Self::PlayersScored),
            "goalies-beaten-by-team" => Ok(Self::GoaliesBeaten),
            "fight-opponents-by-team" => Ok(Self::FightOpponents),
            other => Err(other.to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::PlayersScored => "players-scored-against-team",
            Self::GoaliesBeaten => "goalies-beaten-by-team",
            Self::FightOpponents => "fight-opponents-by-team",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::PlayersScored => "Players who scored against this team",
            Self::GoaliesBeaten => "Goalies this team scored against",
            Self::FightOpponents => "Opposing players fought by this team",
        }
    }

    fn subject_label(self) -> &'static str {
        match self {
            Self::PlayersScored => "player",
            Self::GoaliesBeaten => "goalie",
            Self::FightOpponents => "opponent",
        }
    }

    fn empty_hint(self) -> &'static str {
        match self {
            Self::PlayersScored => {
                "No matching team game records are loaded yet. Load active-season game lines from the NHL API, then this page will read them from the local cache."
            }
            Self::GoaliesBeaten => {
                "No team goalie matchup records are loaded yet. Load active-season play-by-play from the NHL API, then this page will read it from the local cache."
            }
            Self::FightOpponents => {
                "No team fight records are loaded yet. Load active-season play-by-play from the NHL API, then this page will read it from the local cache."
            }
        }
    }

    fn cache_artifacts(self) -> &'static str {
        match self {
            Self::PlayersScored => "boxscore",
            Self::GoaliesBeaten | Self::FightOpponents => "play-by-play",
        }
    }
}

pub async fn get_player_records(
    State(state): State<WebState>,
    Path(id): Path<u32>,
    Query(query): Query<RecordsQuery>,
) -> Response {
    let metric = match PlayerWebRecordsMetric::parse(query.metric.as_deref()) {
        Ok(metric) => metric,
        Err(metric) => {
            return metric_error("records-player", &metric, PlayerWebRecordsMetric::ALLOWED)
        }
    };
    let (active_label, view, cache_teams) =
        match build_player_records_view(&state, id, metric).await {
            Ok(result) => result,
            Err(response) => return response,
        };
    let cache_return_to = format!("/records/player/{id}?metric={}", metric.as_str());
    let template = records_template(RecordsTemplateInput {
        active_label,
        active_season: view.context.window.season.as_str(),
        active_season_type: view.context.window.season_type.label().to_string(),
        title: format!("{} Records", view.player_name),
        subtitle: metric.subtitle().to_string(),
        back_href: format!("/player/{id}"),
        back_label: "player card".to_string(),
        json_href: format!("/api/v1/records/player/{id}?metric={}", metric.as_str()),
        subject_label: metric.subject_label().to_string(),
        empty_hint: metric.empty_hint().to_string(),
        cache_teams: cache_teams.join(","),
        cache_artifacts: metric.cache_artifacts().to_string(),
        cache_return_to,
        cache_button_label: "Load game cache for this player".to_string(),
        rows: &view.rows,
    });
    render_template(template)
}

pub async fn get_player_records_json(
    State(state): State<WebState>,
    Path(id): Path<u32>,
    Query(query): Query<RecordsQuery>,
) -> Response {
    let metric = match PlayerWebRecordsMetric::parse(query.metric.as_deref()) {
        Ok(metric) => metric,
        Err(metric) => {
            return metric_error("records-player", &metric, PlayerWebRecordsMetric::ALLOWED)
        }
    };
    match build_player_records_view(&state, id, metric).await {
        Ok((_active_label, view, _cache_teams)) => {
            let meta = serde_json::json!({
                "player_id": view.player_id,
                "player_name": view.player_name.clone(),
                "metric": view.metric.clone(),
                "rows": view.rows.len(),
                "incomplete_goal_rows": view.incomplete_goal_rows,
                "source_state": view.context.source_state.clone(),
            });
            crate::api::json_data_meta("records-player", view, meta)
        }
        Err(response) => response,
    }
}

pub async fn get_team_records(
    State(state): State<WebState>,
    Path(abbrev): Path<String>,
    Query(query): Query<RecordsQuery>,
) -> Response {
    let metric = match TeamWebRecordsMetric::parse(query.metric.as_deref()) {
        Ok(metric) => metric,
        Err(metric) => return metric_error("records-team", &metric, TeamWebRecordsMetric::ALLOWED),
    };
    let (active_label, view) = match build_team_records_view(&state, &abbrev, metric).await {
        Ok(result) => result,
        Err(response) => return response,
    };
    let cache_return_to = format!("/records/team/{}?metric={}", view.team, metric.as_str());
    let template = records_template(RecordsTemplateInput {
        active_label,
        active_season: view.context.window.season.as_str(),
        active_season_type: view.context.window.season_type.label().to_string(),
        title: format!("{} Records", view.team),
        subtitle: metric.subtitle().to_string(),
        back_href: format!("/team/{}/season", view.team),
        back_label: "team season".to_string(),
        json_href: format!(
            "/api/v1/records/team/{}?metric={}",
            view.team,
            metric.as_str()
        ),
        subject_label: metric.subject_label().to_string(),
        empty_hint: metric.empty_hint().to_string(),
        cache_teams: view.team.clone(),
        cache_artifacts: metric.cache_artifacts().to_string(),
        cache_return_to,
        cache_button_label: "Load game cache for this team".to_string(),
        rows: &view.rows,
    });
    render_template(template)
}

pub async fn get_team_records_json(
    State(state): State<WebState>,
    Path(abbrev): Path<String>,
    Query(query): Query<RecordsQuery>,
) -> Response {
    let metric = match TeamWebRecordsMetric::parse(query.metric.as_deref()) {
        Ok(metric) => metric,
        Err(metric) => return metric_error("records-team", &metric, TeamWebRecordsMetric::ALLOWED),
    };
    match build_team_records_view(&state, &abbrev, metric).await {
        Ok((_active_label, view)) => {
            let meta = serde_json::json!({
                "team": view.team.clone(),
                "metric": view.metric.clone(),
                "rows": view.rows.len(),
                "incomplete_goal_rows": view.incomplete_goal_rows,
                "source_state": view.context.source_state.clone(),
            });
            crate::api::json_data_meta("records-team", view, meta)
        }
        Err(response) => response,
    }
}

async fn build_player_records_view(
    state: &WebState,
    id: u32,
    metric: PlayerWebRecordsMetric,
) -> Result<(String, PlayerRecordsView, Vec<String>), Response> {
    let (active_label, context) = active_context(state, "records-player").await?;
    let pid = PlayerId(id);
    {
        let mut repo = state.repo.write().await;
        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid) {
            eprintln!("warn: records career fan-out for pid={id} failed: {e}");
        }
    }
    let (player_name, cache_teams) = {
        let repo = state.repo.read().await;
        match repo.identity(pid) {
            Some(identity) => (
                identity.full_name.clone(),
                player_cache_teams(
                    &repo,
                    pid,
                    context.window.season,
                    context.window.season_type,
                ),
            ),
            None => {
                return Err(crate::api::json_error_meta(
                    StatusCode::NOT_FOUND,
                    "records-player",
                    serde_json::json!({ "player_id": id }),
                    serde_json::json!({}),
                    format!("No player with NHL id {id} in the active repository."),
                ));
            }
        }
    };
    let view = match metric {
        PlayerWebRecordsMetric::TeamsScoredAgainst => {
            let goals =
                icelines_fetch::records_provider::load_goal_record_inputs_from_default_store()
                    .map_err(|err| server_error("records-player", err))?;
            PlayerRecordsView::teams_scored_against(context, id, player_name, &goals)
        }
        PlayerWebRecordsMetric::GoaliesScoredAgainst => {
            let goals =
                icelines_fetch::records_provider::load_play_by_play_goal_record_inputs_from_default_store()
                    .map_err(|err| server_error("records-player", err))?;
            PlayerRecordsView::goalies_scored_against(context, id, player_name, &goals)
        }
        PlayerWebRecordsMetric::FightOpponents => {
            let fights =
                icelines_fetch::records_provider::load_fight_record_inputs_from_default_store()
                    .map_err(|err| server_error("records-player", err))?;
            PlayerRecordsView::fight_opponents(context, id, player_name, &fights)
        }
    };
    Ok((active_label, view, cache_teams))
}

async fn build_team_records_view(
    state: &WebState,
    abbrev: &str,
    metric: TeamWebRecordsMetric,
) -> Result<(String, TeamRecordsView), Response> {
    let (active_label, context) = active_context(state, "records-team").await?;
    let team = TeamAbbr::parse(abbrev).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::NOT_FOUND,
            "records-team",
            serde_json::json!({ "team": abbrev.to_ascii_uppercase() }),
            serde_json::json!({}),
            format!("'{}' is not a valid NHL team abbreviation", abbrev),
        )
    })?;
    let view = match metric {
        TeamWebRecordsMetric::PlayersScored => {
            let goals =
                icelines_fetch::records_provider::load_goal_record_inputs_from_default_store()
                    .map_err(|err| server_error("records-team", err))?;
            TeamRecordsView::players_scored_against_team(context, team.0, &goals)
        }
        TeamWebRecordsMetric::GoaliesBeaten => {
            let goals =
                icelines_fetch::records_provider::load_play_by_play_goal_record_inputs_from_default_store()
                    .map_err(|err| server_error("records-team", err))?;
            TeamRecordsView::goalies_beaten_by_team(context, team.0, &goals)
        }
        TeamWebRecordsMetric::FightOpponents => {
            let fights =
                icelines_fetch::records_provider::load_fight_record_inputs_from_default_store()
                    .map_err(|err| server_error("records-team", err))?;
            TeamRecordsView::fight_opponents_by_team(context, team.0, &fights)
        }
    };
    Ok((active_label, view))
}

async fn active_context(
    state: &WebState,
    route: &'static str,
) -> Result<(String, ViewContext), Response> {
    let cfg = state.config.read().await;
    let season = cfg.active_season.parse::<u32>().map(Season).map_err(|_| {
        crate::api::json_error_meta(
            StatusCode::BAD_REQUEST,
            route,
            serde_json::json!({}),
            serde_json::json!({ "season": cfg.active_season }),
            format!("Season '{}' is not a valid YYYYZZZZ id", cfg.active_season),
        )
    })?;
    let season_type = SeasonType::parse_lossy(&cfg.active_season_type);
    Ok((
        cfg.active_label.clone(),
        ViewContext::new(ViewWindow::new(season, season_type)),
    ))
}

struct RecordsTemplateInput<'a> {
    active_label: String,
    active_season: String,
    active_season_type: String,
    title: String,
    subtitle: String,
    back_href: String,
    back_label: String,
    json_href: String,
    subject_label: String,
    empty_hint: String,
    cache_teams: String,
    cache_artifacts: String,
    cache_return_to: String,
    cache_button_label: String,
    rows: &'a [RecordsOpponentRow],
}

fn records_template(input: RecordsTemplateInput<'_>) -> RecordsTemplate {
    let rows: Vec<_> = input.rows.iter().map(record_row).collect();
    let records_count_svg = render_records_count_svg(&input.subtitle, &rows);
    RecordsTemplate {
        active_label: input.active_label,
        active_season: input.active_season,
        active_season_type: input.active_season_type,
        title: input.title,
        subtitle: input.subtitle,
        back_href: input.back_href,
        back_label: input.back_label,
        json_href: input.json_href,
        subject_label: input.subject_label,
        empty_hint: input.empty_hint,
        cache_teams: input.cache_teams,
        cache_artifacts: input.cache_artifacts,
        cache_return_to: input.cache_return_to,
        cache_button_label: input.cache_button_label,
        total: input.rows.len(),
        records_count_svg,
        rows,
    }
}

fn player_cache_teams(
    repo: &icelines_core::stats_repository::StatsRepository,
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
) -> Vec<String> {
    let Some(stats) = repo.season(pid, season, season_type) else {
        let Some(stats) = repo.career_all(pid).and_then(|rows| {
            rows.filter(|row| row.season <= season && row.season_type == season_type)
                .max_by_key(|row| row.season)
        }) else {
            return Vec::new();
        };
        return sorted_teams_from_stats(stats);
    };
    sorted_teams_from_stats(stats)
}

fn sorted_teams_from_stats(stats: &icelines_core::season_stats::SeasonStats) -> Vec<String> {
    let mut teams: Vec<String> = stats
        .team_stints
        .iter()
        .map(|stint| stint.team.0.clone())
        .collect();
    teams.sort();
    teams.dedup();
    teams
}

fn record_row(row: &RecordsOpponentRow) -> RecordsTemplateRow {
    RecordsTemplateRow {
        key: row.key.clone(),
        label: row.label.clone(),
        count: row.count,
        first_game_id: row.first_game_id,
        first_date: row.first_date.clone().unwrap_or_default(),
        last_game_id: row.last_game_id,
        last_date: row.last_date.clone().unwrap_or_default(),
    }
}

fn render_records_count_svg(title: &str, rows: &[RecordsTemplateRow]) -> Option<String> {
    let mut values: Vec<_> = rows.iter().filter(|row| row.count > 0).collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
    values.truncate(10);
    let max = values.iter().map(|row| row.count).max().unwrap_or(0);
    if max == 0 {
        return None;
    }

    let mut bars = String::new();
    for (idx, row) in values.iter().enumerate() {
        let y = 46 + idx * 24;
        let width = ((row.count as f64 / max as f64) * 390.0).max(2.0);
        let label = escape_svg_text(&row.label);
        bars.push_str(&format!(
            r##"  <text x="24" y="{label_y}" fill="#334155" font-size="11">{label}</text>
  <rect x="176" y="{bar_y}" width="{width:.1}" height="14" rx="3" fill="#7c3aed"/>
  <text x="{value_x:.1}" y="{label_y}" fill="#0f172a" font-size="11">{count}</text>
"##,
            label_y = y + 11,
            bar_y = y,
            value_x = 184.0 + width,
            count = row.count,
        ));
    }
    let height = 72 + values.len() * 24;
    let title = escape_svg_text(title);

    Some(format!(
        r##"<svg class="records-count-svg" viewBox="0 0 640 {height}" role="img" aria-labelledby="records-count-title records-count-desc">
  <title id="records-count-title">Records count chart</title>
  <desc id="records-count-desc">{title} by record count.</desc>
  <rect x="0" y="0" width="640" height="{height}" rx="8" fill="#f8fafc"/>
  <text x="24" y="26" fill="#334155" font-size="13">Record counts</text>
  <line x1="176" y1="36" x2="566" y2="36" stroke="#cbd5e1"/>
{bars}</svg>"##
    ))
}

fn escape_svg_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_template(template: RecordsTemplate) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_records_count_svg_renders_positive_rows() {
        let rows = vec![
            RecordsTemplateRow {
                key: "EDM".to_string(),
                label: "Edmonton Oilers".to_string(),
                count: 3,
                first_game_id: 1,
                first_date: "2025-10-01".to_string(),
                last_game_id: 2,
                last_date: "2025-10-02".to_string(),
            },
            RecordsTemplateRow {
                key: "SEA".to_string(),
                label: "Seattle Kraken".to_string(),
                count: 1,
                first_game_id: 3,
                first_date: "2025-10-03".to_string(),
                last_game_id: 3,
                last_date: "2025-10-03".to_string(),
            },
        ];

        let svg = render_records_count_svg("NHL teams scored against", &rows)
            .expect("positive rows should render");

        assert!(svg.contains("records-count-svg"));
        assert!(svg.contains("Records count chart"));
        assert!(svg.contains("Edmonton Oilers"));
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn l0_records_count_svg_skips_empty_counts() {
        let rows = vec![RecordsTemplateRow {
            key: "EDM".to_string(),
            label: "Edmonton Oilers".to_string(),
            count: 0,
            first_game_id: 1,
            first_date: String::new(),
            last_game_id: 1,
            last_date: String::new(),
        }];

        assert!(render_records_count_svg("NHL teams scored against", &rows).is_none());
    }
}

fn server_error(route: &'static str, err: anyhow::Error) -> Response {
    crate::api::json_error_meta(
        StatusCode::INTERNAL_SERVER_ERROR,
        route,
        serde_json::json!({}),
        serde_json::json!({}),
        err.to_string(),
    )
}

fn metric_error(route: &'static str, metric: &str, allowed: &[&str]) -> Response {
    crate::api::json_error_meta(
        StatusCode::BAD_REQUEST,
        route,
        serde_json::json!({ "metric": metric }),
        serde_json::json!({ "allowed": allowed }),
        format!("Unsupported records metric '{metric}'"),
    )
}
