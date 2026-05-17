use crate::state::WebState;
use crate::templates::{
    DashboardCatalogEntry, DashboardCatalogGroup, DashboardEntityRow, DashboardExperienceTab,
    DashboardFieldRow, DashboardLinkRow, DashboardPaneBindingRow, DashboardPaneModelRow,
    DashboardSummaryRow, DashboardTemplate, DashboardWorkspaceTemplate, PlayoffsSeriesView,
    ScheduleRow, ScoreRow, TransactionRow,
};
use askama::Template;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::{
    poach_report_from_board, weekly_poach_report_from_board_with_watched, AvailabilityState,
    FantasyRosterGapView, FantasySimulationView, PoachBoardView, PoachReportView, WatchNoteInput,
};
use icelines_core::{
    workbench_entry, workbench_field, workbench_pane_binding, workbench_pane_model, CareerView,
    DepthLeagueView, DepthTeamStrengthRow, FavoriteMemberInput, FavoritesView, HomeView,
    MetricCell, MetricValue, PlayerCardView, PlayerSeasonSummary, ScheduleRecord, TeamAbbr,
    TeamDepthView, TeamSeasonView, ViewContext, ViewWindow, WatchlistView, WorkbenchEntry,
    WorkbenchExperience, WorkbenchFieldId, WorkbenchFieldSource, WorkbenchFieldSummary,
    WorkbenchGroup, WorkbenchId, WorkbenchPaneBinding, WorkbenchPaneBindingId,
    WorkbenchPaneInteraction, WorkbenchPaneKind, WorkbenchPaneModelId, WorkbenchValueKind,
    WorkbenchZone, WORKBENCH_EXPERIENCES,
};
use serde::Deserialize;

const DASHBOARD_PREVIEW_N: usize = 10;
const DASHBOARD_GOALIE_GP_REGULAR: u32 = 5;
const DASHBOARD_GOALIE_GP_PLAYOFF: u32 = 1;
const DEFAULT_LEFT_PANE: WorkbenchPaneBindingId = WorkbenchPaneBindingId::FavoritesLeft;
const DEFAULT_RIGHT_PANE: WorkbenchPaneBindingId = WorkbenchPaneBindingId::ScheduleRight;

struct DashboardComposition {
    experience: Option<&'static WorkbenchExperience>,
    left: &'static WorkbenchPaneBinding,
    right: &'static WorkbenchPaneBinding,
}

#[derive(Debug, Deserialize, Default)]
pub struct DashboardQuery {
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub partial: Option<String>,
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default)]
    pub left_workspace: Option<String>,
    #[serde(default)]
    pub right_workspace: Option<String>,
    #[serde(default)]
    pub experience: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardCommandForm {
    pub command: String,
    #[serde(default)]
    pub workspace: Option<String>,
}

pub async fn get_dashboard(
    State(state): State<WebState>,
    Query(q): Query<DashboardQuery>,
) -> Response {
    let active_label = state.config.read().await.active_label.clone();
    let requested_experience = q.experience.as_deref().and_then(web_experience_by_slug);
    let workspace_url = requested_experience
        .filter(|_| q.workspace.is_none())
        .and_then(|experience| crate::workbench::route_for_workbench(experience.center))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| normalize_workspace(q.workspace.as_deref()));
    let left_pane_workspace_url = normalize_optional_workspace(q.left_workspace.as_deref());
    let right_pane_workspace_url = normalize_optional_workspace(q.right_workspace.as_deref());
    let left_pane_workspace_label = if left_pane_workspace_url.is_empty() {
        String::new()
    } else {
        workspace_label(&left_pane_workspace_url)
    };
    let right_pane_workspace_label = if right_pane_workspace_url.is_empty() {
        String::new()
    } else {
        workspace_label(&right_pane_workspace_url)
    };
    let workspace_label = workspace_label(&workspace_url);
    let workspace_links = workspace_links(&workspace_url);
    let center_workspace_summary = workspace_summary(&state, &workspace_url).await;
    let left_pane_workspace_summary = if left_pane_workspace_url.is_empty() {
        Vec::new()
    } else {
        workspace_summary(&state, &left_pane_workspace_url).await
    };
    let right_pane_workspace_summary = if right_pane_workspace_url.is_empty() {
        Vec::new()
    } else {
        workspace_summary(&state, &right_pane_workspace_url).await
    };
    let scores_preview = scores_workspace_summary(&state, "/scores").await;
    let active_workbench = workbench_id_for_workspace(&workspace_url);
    let composition = dashboard_composition(&q, active_workbench);
    let active_fields = active_dashboard_fields(active_workbench, composition.experience);
    let active_pane_models = active_dashboard_pane_models(active_workbench);
    let show_full_leaders = workspace_route_key(&workspace_url) == "/leaders";
    let show_full_goalies = workspace_route_key(&workspace_url) == "/goalies";
    let show_full_depth = workspace_route_key(&workspace_url) == "/depth";
    let show_full_player = workspace_route_key(&workspace_url).starts_with("/player/");
    let show_full_team = team_workspace_slug(workspace_route_key(&workspace_url)).is_some()
        && !workspace_route_key(&workspace_url).ends_with("/season");
    let show_full_team_season = team_workspace_slug(workspace_route_key(&workspace_url)).is_some()
        && workspace_route_key(&workspace_url).ends_with("/season");
    let show_full_scores = workspace_route_key(&workspace_url) == "/scores";
    let show_full_schedule = workspace_route_key(&workspace_url) == "/schedule";
    let (leaders_query, leaders_raw_query) = leaders_query_from_workspace(&workspace_url);
    let leaders_surface =
        match super::leaders::build_leaders_template(&state, leaders_query, &leaders_raw_query)
            .await
        {
            Ok(template) => template,
            Err(response) => return response,
        };
    let goalies_surface_html = if show_full_goalies {
        match full_goalies_workspace_html(&state, &workspace_url).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };
    let depth_surface_html = if show_full_depth {
        match full_depth_workspace_html(&state).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };
    let player_surface_html = if show_full_player {
        match full_player_workspace_html(&state, &workspace_url).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };
    let team_surface_html = if show_full_team {
        match full_team_workspace_html(&state, &workspace_url).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };
    let team_season_surface_html = if show_full_team_season {
        match full_team_season_workspace_html(&state, &workspace_url).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };
    let scores_surface_html = if show_full_scores {
        match full_scores_workspace_html(&state, &workspace_url).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };
    let schedule_surface_html = if show_full_schedule {
        match full_schedule_workspace_html(&state, &workspace_url).await {
            Ok(html) => html,
            Err(response) => return response,
        }
    } else {
        String::new()
    };

    if matches!(q.partial.as_deref(), Some("workspace")) {
        return render_template(DashboardWorkspaceTemplate {
            workspace_url,
            workspace_label,
            workspace_summary: center_workspace_summary,
            workspace_links,
            active_fields,
            active_pane_models,
            show_full_leaders,
            leaders_surface,
            show_full_goalies,
            goalies_surface_html,
            show_full_depth,
            depth_surface_html,
            show_full_player,
            player_surface_html,
            show_full_team,
            team_surface_html,
            show_full_team_season,
            team_season_surface_html,
            show_full_scores,
            scores_surface_html,
            show_full_schedule,
            schedule_surface_html,
        });
    }

    if q.partial.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Html("unknown dashboard partial".to_owned()),
        )
            .into_response();
    }

    let favorites = dashboard_favorites_entities(&state).await;
    let watchlist = dashboard_watchlist_entities(&state).await;
    let leaders_preview = leaders_workspace_summary(&state).await;
    let schedule_preview = schedule_workspace_summary(&state, "/schedule").await;

    let tmpl = DashboardTemplate {
        active_label,
        workspace_url: workspace_url.clone(),
        workspace_label,
        workspace_summary: center_workspace_summary,
        scores_summary: "Current slate".to_owned(),
        scores_preview,
        catalog_groups: dashboard_catalog_groups(active_workbench, &composition),
        experience_tabs: dashboard_experience_tabs(active_workbench, composition.experience),
        active_fields,
        active_pane_models,
        show_full_leaders,
        leaders_surface,
        show_full_goalies,
        goalies_surface_html,
        show_full_depth,
        depth_surface_html,
        show_full_player,
        player_surface_html,
        show_full_team,
        team_surface_html,
        show_full_team_season,
        team_season_surface_html,
        show_full_scores,
        scores_surface_html,
        show_full_schedule,
        schedule_surface_html,
        left_pane_workspace_url,
        left_pane_workspace_label,
        left_pane_workspace_summary,
        right_pane_workspace_url,
        right_pane_workspace_label,
        right_pane_workspace_summary,
        left_pane_binding: dashboard_pane_binding_row(
            composition.left,
            dashboard_href(
                &workspace_url,
                composition.left.id,
                composition.right.id,
                composition.experience,
            ),
            true,
        ),
        right_pane_binding: dashboard_pane_binding_row(
            composition.right,
            dashboard_href(
                &workspace_url,
                composition.left.id,
                composition.right.id,
                composition.experience,
            ),
            true,
        ),
        left_pane_options: dashboard_pane_options(
            WorkbenchZone::LeftPane,
            composition.left.id,
            composition.right.id,
            &workspace_url,
        ),
        right_pane_options: dashboard_pane_options(
            WorkbenchZone::RightPane,
            composition.left.id,
            composition.right.id,
            &workspace_url,
        ),
        leaders_preview,
        favorites,
        watchlist,
        schedule_preview,
        schedule_links: schedule_links(),
        workspace_links,
    };

    render_template(tmpl)
}

pub async fn post_dashboard_command(
    headers: HeaderMap,
    State(state): State<WebState>,
    Form(form): Form<DashboardCommandForm>,
) -> Response {
    let current_workspace = normalize_workspace(form.workspace.as_deref());
    match crate::dashboard_command::parse_dashboard_command(&form.command) {
        Ok(crate::dashboard_command::DashboardCommand::OpenWorkspace { url }) => {
            Redirect::to(&dashboard_workspace_href(&normalize_workspace(Some(&url))))
                .into_response()
        }
        Ok(crate::dashboard_command::DashboardCommand::HidePane(_))
        | Ok(crate::dashboard_command::DashboardCommand::ShowPane(_)) => {
            Redirect::to(&dashboard_workspace_href(&current_workspace)).into_response()
        }
        Ok(crate::dashboard_command::DashboardCommand::Mutation(
            crate::dashboard_command::DashboardMutationIntent::FavoriteAdd { player, .. },
        )) => {
            super::favorites::post_add(
                headers,
                Form(super::favorites::FavoritesMutation {
                    key: player,
                    kind: Some("player".to_owned()),
                    return_to: Some(dashboard_workspace_href(&current_workspace)),
                }),
            )
            .await
        }
        Ok(crate::dashboard_command::DashboardCommand::Mutation(
            crate::dashboard_command::DashboardMutationIntent::FavoriteRemove { player, .. },
        )) => {
            super::favorites::post_remove(
                headers,
                Form(super::favorites::FavoritesMutation {
                    key: player,
                    kind: Some("player".to_owned()),
                    return_to: Some(dashboard_workspace_href(&current_workspace)),
                }),
            )
            .await
        }
        Ok(crate::dashboard_command::DashboardCommand::Mutation(
            crate::dashboard_command::DashboardMutationIntent::WatchPlayer {
                player, trigger, ..
            },
        )) => {
            super::poach::post_watch_rule_create_form(Form(super::poach::WatchRuleCreateForm {
                player,
                trigger,
                return_to: Some(dashboard_workspace_href(&current_workspace)),
            }))
            .await
        }
        Ok(crate::dashboard_command::DashboardCommand::Mutation(
            crate::dashboard_command::DashboardMutationIntent::WatchSetEnabled {
                rule_id,
                enabled,
                ..
            },
        )) => {
            super::poach::post_watch_rule_enabled_form(
                State(state),
                Form(super::poach::WatchRuleMutationForm {
                    rule_id,
                    enabled,
                    return_to: Some(dashboard_workspace_href(&current_workspace)),
                }),
            )
            .await
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Html(format!("dashboard command error: {err}")),
        )
            .into_response(),
    }
}

fn render_template<T: Template>(tmpl: T) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

async fn full_player_workspace_html(
    state: &WebState,
    workspace_url: &str,
) -> Result<String, Response> {
    let Some(player_id) = player_workspace_id(workspace_route_key(workspace_url))
        .and_then(|raw| raw.parse::<u32>().ok())
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("invalid player workspace".to_owned()),
        )
            .into_response());
    };
    let template = super::player::build_player_template(state, player_id).await?;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("player template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

async fn full_team_workspace_html(
    state: &WebState,
    workspace_url: &str,
) -> Result<String, Response> {
    let Some(team) = team_workspace_slug(workspace_route_key(workspace_url)) else {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("invalid team workspace".to_owned()),
        )
            .into_response());
    };
    let template = super::team::build_team_template(state, team).await?;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("team template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

async fn full_team_season_workspace_html(
    state: &WebState,
    workspace_url: &str,
) -> Result<String, Response> {
    let Some(team) = team_workspace_slug(workspace_route_key(workspace_url)) else {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("invalid team season workspace".to_owned()),
        )
            .into_response());
    };
    let template = super::team::build_team_season_template(state, team).await?;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("team season template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

async fn full_goalies_workspace_html(
    state: &WebState,
    workspace_url: &str,
) -> Result<String, Response> {
    let q = goalies_query_from_workspace(workspace_url);
    let template = super::goalies::build_goalies_template(state, &q).await?;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("goalies template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

async fn full_depth_workspace_html(state: &WebState) -> Result<String, Response> {
    let template = super::depth::build_depth_template(state).await?;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("depth template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

async fn full_scores_workspace_html(
    state: &WebState,
    workspace_url: &str,
) -> Result<String, Response> {
    let q = scores_query_from_workspace(workspace_url);
    let template = super::scores::build_scores_template(state, &q).await;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("scores template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

async fn full_schedule_workspace_html(
    state: &WebState,
    workspace_url: &str,
) -> Result<String, Response> {
    let q = schedule_query_from_workspace(workspace_url);
    let template = super::schedule::build_schedule_template(state, &q).await;
    let rendered = template.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response()
    })?;
    extract_main_content(&rendered).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("schedule template did not render a main region".to_owned()),
        )
            .into_response()
    })
}

fn extract_main_content(rendered: &str) -> Option<String> {
    let start_marker = "<main id=\"main\">";
    let start = rendered.find(start_marker)? + start_marker.len();
    let end = rendered[start..].find("</main>")? + start;
    Some(rendered[start..end].trim().to_owned())
}

fn normalize_workspace(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|path| is_workspace_route(path))
        .unwrap_or("/leaders")
        .to_owned()
}

fn normalize_optional_workspace(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|path| is_workspace_route(path))
        .unwrap_or("")
        .to_owned()
}

fn is_workspace_route(path: &str) -> bool {
    if path.is_empty()
        || path.chars().any(char::is_control)
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("://")
    {
        return false;
    }

    let route = workspace_route_key(path);
    matches!(
        route,
        "/" | "/leaders"
            | "/goalies"
            | "/depth"
            | "/poach"
            | "/fantasy"
            | "/scores"
            | "/schedule"
            | "/transactions"
            | "/playoffs"
            | "/favorites"
            | "/watchlist"
            | "/career"
            | "/reports/poach"
            | "/reports/weekly"
            | "/admin"
            | "/docs"
    ) || player_workspace_id(route).is_some()
        || team_workspace_slug(route).is_some()
        || game_workspace_id(route).is_some()
}

fn workspace_route_key(path: &str) -> &str {
    let route = path.split('?').next().unwrap_or(path);
    if route.len() > 1 {
        route.trim_end_matches('/')
    } else {
        route
    }
}

fn player_workspace_id(route: &str) -> Option<&str> {
    route
        .strip_prefix("/player/")
        .filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
}

fn team_workspace_slug(route: &str) -> Option<&str> {
    let rest = route.strip_prefix("/team/")?;
    let slug = rest.strip_suffix("/season").unwrap_or(rest);
    if !slug.is_empty()
        && (2..=3).contains(&slug.len())
        && slug.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        Some(slug)
    } else {
        None
    }
}

fn game_workspace_id(route: &str) -> Option<&str> {
    route
        .strip_prefix("/game/")
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn leaders_query_from_workspace(path: &str) -> (super::leaders::LeadersQuery, String) {
    let raw_query = path
        .split_once('?')
        .map(|(_, query)| query.to_owned())
        .unwrap_or_default();
    let mut q = super::leaders::LeadersQuery::default();

    for pair in raw_query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = decode_query_component(value);
        match key {
            "sort" => q.sort = Some(value),
            "pos" => q.pos = Some(value),
            "top" => q.top = value.parse().ok(),
            "age-min" => q.age_min = value.parse().ok(),
            "age-max" => q.age_max = value.parse().ok(),
            "draft-min" => q.draft_year_min = value.parse().ok(),
            "draft-max" => q.draft_year_max = value.parse().ok(),
            "height-min" => q.height_min = value.parse().ok(),
            "height-max" => q.height_max = value.parse().ok(),
            "weight-min" => q.weight_min = value.parse().ok(),
            "weight-max" => q.weight_max = value.parse().ok(),
            "country" => q.country = Some(value),
            "shoots" => q.shoots = Some(value),
            _ => {}
        }
    }

    (q, raw_query)
}

fn decode_query_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(high), Some(low)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
                {
                    out.push(high * 16 + low);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn workspace_label(path: &str) -> String {
    match workspace_route_key(path) {
        "/" => "Home Preview",
        "/leaders" => "Leaders",
        "/goalies" => "Goalies",
        "/depth" => "Depth",
        "/poach" => "Poach",
        "/fantasy" => "Fantasy",
        "/scores" => "Scores",
        "/schedule" => "Schedule",
        "/transactions" => "Transactions",
        "/playoffs" => "Playoffs",
        "/favorites" => "Favorites",
        "/watchlist" => "Watchlist",
        "/career" => "Career Cohorts",
        "/reports/poach" => "Poach Report",
        "/reports/weekly" => "Weekly Report",
        "/docs" => "Docs",
        other if other.starts_with("/player/") => "Player Card",
        other if other.starts_with("/team/") && other.ends_with("/season") => "Team Season",
        other if other.starts_with("/team/") => "Team Depth",
        other if other.starts_with("/game/") => "Game Detail",
        _ => "Workspace",
    }
    .to_owned()
}

async fn workspace_summary(state: &WebState, path: &str) -> Vec<DashboardSummaryRow> {
    let route = workspace_route_key(path);
    if matches!(route, "/" | "/leaders") {
        return leaders_workspace_summary(state).await;
    }
    if route == "/goalies" {
        return goalies_workspace_summary(state).await;
    }
    if route == "/depth" {
        return depth_workspace_summary(state).await;
    }
    if route == "/poach" {
        return poach_workspace_summary(state, path).await;
    }
    if route == "/reports/poach" {
        return poach_report_workspace_summary(state, path, false).await;
    }
    if route == "/reports/weekly" {
        return poach_report_workspace_summary(state, path, true).await;
    }
    if route == "/fantasy" {
        return fantasy_workspace_summary(state, path).await;
    }
    if route == "/scores" {
        return scores_workspace_summary(state, path).await;
    }
    if route == "/schedule" {
        return schedule_workspace_summary(state, path).await;
    }
    if let Some(player_id) = route.strip_prefix("/player/") {
        return player_workspace_summary(state, player_id).await;
    }
    if let Some(game_id) = route.strip_prefix("/game/") {
        return game_workspace_summary(state, game_id).await;
    }
    if route == "/transactions" {
        return transactions_workspace_summary(state, path).await;
    }
    if route == "/playoffs" {
        return playoffs_workspace_summary(state).await;
    }
    if route == "/favorites" {
        return favorites_workspace_summary(state).await;
    }
    if route == "/watchlist" {
        return watchlist_workspace_summary(state).await;
    }
    if route == "/career" {
        return career_workspace_summary(path).await;
    }
    if let Some(team) = route
        .strip_prefix("/team/")
        .and_then(|rest| rest.strip_suffix("/season"))
    {
        return team_season_workspace_summary(state, team).await;
    }
    if let Some(team) = route.strip_prefix("/team/") {
        return team_depth_workspace_summary(state, team).await;
    }
    Vec::new()
}

async fn home_view_for_dashboard(state: &WebState) -> Option<HomeView> {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season = season_str.parse::<u32>().map(Season).ok()?;
    let goalie_floor = match season_type {
        SeasonType::Regular => DASHBOARD_GOALIE_GP_REGULAR,
        SeasonType::Playoff => DASHBOARD_GOALIE_GP_PLAYOFF,
    };
    let repo = state.repo.read().await;
    Some(HomeView::from_repository(
        &repo,
        season,
        season_type,
        goalie_floor,
        DASHBOARD_PREVIEW_N,
    ))
}

async fn leaders_workspace_summary(state: &WebState) -> Vec<DashboardSummaryRow> {
    let Some(view) = home_view_for_dashboard(state).await else {
        return Vec::new();
    };
    if view.top_skaters.is_empty() {
        return vec![summary_row(
            "Leaders",
            "No rows",
            "No skater rows loaded for the active season",
        )];
    }
    view.top_skaters
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            summary_row(
                format!("#{}", idx + 1),
                row.display_name.clone(),
                format!("{} {} pts · {} G", row.team.0, row.points, row.goals),
            )
            .with_href(format!("/player/{}", row.player_id.0))
        })
        .collect()
}

async fn goalies_workspace_summary(state: &WebState) -> Vec<DashboardSummaryRow> {
    let Some(view) = home_view_for_dashboard(state).await else {
        return Vec::new();
    };
    if view.top_goalies.is_empty() {
        return vec![summary_row(
            "Goalies",
            "No rows",
            "No qualified goalie rows loaded for the active season",
        )];
    }
    view.top_goalies
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            summary_row(
                format!("#{}", idx + 1),
                row.display_name.clone(),
                format!(
                    "{} {} W · SV% {}",
                    row.team.0,
                    row.wins,
                    row.save_pct
                        .map(|value| format!("{value:.3}"))
                        .unwrap_or_else(|| "-".to_string())
                ),
            )
        })
        .collect()
}

async fn depth_workspace_summary(state: &WebState) -> Vec<DashboardSummaryRow> {
    let (season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(icelines_core::CURRENT_SEASON)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let repo = state.repo.read().await;
    let view = DepthLeagueView::pace_from_repository(&repo, season, season_type);
    depth_summary_rows(&view)
}

fn depth_summary_rows(view: &DepthLeagueView) -> Vec<DashboardSummaryRow> {
    if view.rows.is_empty() {
        return vec![summary_row(
            "Depth",
            "No rows",
            "No league depth rows loaded for the active season",
        )];
    }

    view.rows
        .iter()
        .take(DASHBOARD_PREVIEW_N)
        .enumerate()
        .map(|(idx, row)| depth_summary_row(idx, row))
        .collect()
}

fn depth_summary_row(idx: usize, row: &DepthTeamStrengthRow) -> DashboardSummaryRow {
    summary_row(
        format!("#{}", idx + 1),
        row.team.0.clone(),
        format!("total {:.0} · C {} · D {}", row.total, row.c_top, row.d_top),
    )
}

async fn poach_workspace_summary(state: &WebState, path: &str) -> Vec<DashboardSummaryRow> {
    let q = poach_query_from_workspace(path);
    let Ok(result) = super::poach::build_poach_view(state, &q).await else {
        return Vec::new();
    };
    poach_summary_rows(&result.view)
}

fn poach_query_from_workspace(path: &str) -> super::poach::PoachWebQuery {
    let mut q = super::poach::PoachWebQuery {
        top: Some(DASHBOARD_PREVIEW_N as u16),
        ..Default::default()
    };
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "scheme" => q.scheme = Some(value),
            "category" | "categories" => q.categories = Some(value),
            "team" => q.team = Some(value),
            "pos" => q.pos = Some(value),
            "league" => q.league = Some(value),
            "availability" => q.availability = Some(value),
            "top" => q.top = value.parse::<u16>().ok(),
            _ => {}
        }
    }
    q.top = Some(q.top.unwrap_or(DASHBOARD_PREVIEW_N as u16).clamp(1, 10));
    q
}

fn poach_summary_rows(view: &PoachBoardView) -> Vec<DashboardSummaryRow> {
    let mut rows = vec![summary_row(
        "Candidates",
        view.rows.len().to_string(),
        format!(
            "{} · high {} · medium {}",
            view.scoring_scheme, view.confidence_summary.high, view.confidence_summary.medium
        ),
    )];

    if view.rows.is_empty() {
        rows[0].value = "No rows".to_string();
        rows[0].detail = view
            .empty_state
            .as_ref()
            .and_then(|state| state.detail.clone())
            .unwrap_or_else(|| "No poach candidates matched the active filters".to_string());
        return rows;
    }

    rows.extend(view.rows.iter().take(2).enumerate().map(|(idx, row)| {
        summary_row(
            format!("#{}", idx + 1),
            row.display_name.clone(),
            format!(
                "{} {} · {:.1} · {}",
                row.team.0,
                row.position.abbreviation(),
                row.score.final_score,
                availability_summary_label(row.availability)
            ),
        )
    }));
    rows
}

async fn poach_report_workspace_summary(
    state: &WebState,
    path: &str,
    weekly: bool,
) -> Vec<DashboardSummaryRow> {
    let q = poach_query_from_workspace(path);
    let Ok(result) = super::poach::build_poach_view(state, &q).await else {
        return Vec::new();
    };
    let report = if weekly {
        let league = q.league.as_deref().unwrap_or("default");
        let top = q.top.unwrap_or(DASHBOARD_PREVIEW_N as u16).clamp(1, 100);
        let watched = super::poach::read_watchlist_player_keys();
        weekly_poach_report_from_board_with_watched(result.view, league, top, &watched)
    } else {
        poach_report_from_board(result.view)
    };
    poach_report_summary_rows(&report)
}

fn poach_report_summary_rows(report: &PoachReportView) -> Vec<DashboardSummaryRow> {
    let candidate_count: usize = report
        .sections
        .iter()
        .map(|section| section.rows.len())
        .sum();
    let mut rows = vec![summary_row(
        "Report",
        report.context.title.clone(),
        format!(
            "{} candidates · {} sections",
            candidate_count,
            report.sections.len()
        ),
    )];
    if !report.scoring_categories.is_empty() {
        rows.push(summary_row(
            "Categories",
            report.scoring_categories.join(", "),
            report.scoring_scheme.clone(),
        ));
    }
    rows.extend(report.sections.iter().take(2).map(|section| {
        summary_row(
            section.title.clone(),
            section.rows.len().to_string(),
            section
                .rows
                .first()
                .map(|row| format!("top: {} · {:.1}", row.display_name, row.score.final_score))
                .unwrap_or_else(|| "No candidates in this section".to_string()),
        )
    }));
    rows
}

fn availability_summary_label(state: AvailabilityState) -> &'static str {
    match state {
        AvailabilityState::Available => "available",
        AvailabilityState::RosteredByUser => "my roster",
        AvailabilityState::Watched => "watched",
        AvailabilityState::ImportedRostered => "rostered",
        AvailabilityState::ImportedAvailable => "free",
        AvailabilityState::Unknown => "unknown",
    }
}

async fn fantasy_workspace_summary(state: &WebState, path: &str) -> Vec<DashboardSummaryRow> {
    let q = fantasy_query_from_workspace(path);
    let gaps = super::fantasy::build_fantasy_gaps(state, &q).await;
    let simulation = super::fantasy::build_fantasy_simulation(state, &q).await;
    let gaps_error = gaps.as_ref().err().cloned();
    fantasy_summary_rows(gaps.as_ref().ok(), simulation.as_ref().ok(), gaps_error)
}

fn fantasy_query_from_workspace(path: &str) -> super::fantasy::FantasyWebQuery {
    let mut q = super::fantasy::FantasyWebQuery {
        top: Some(DASHBOARD_PREVIEW_N),
        ..Default::default()
    };
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "league" => q.league = Some(value),
            "scheme" => q.scheme = Some(value),
            "category" | "categories" => q.categories = Some(value),
            "top" => q.top = value.parse::<usize>().ok(),
            "weeks" => q.weeks = value.parse::<u8>().ok(),
            "add_player" => q.add_player = Some(value),
            "drop_player" => q.drop_player = Some(value),
            _ => {}
        }
    }
    q.top = Some(q.top.unwrap_or(DASHBOARD_PREVIEW_N).clamp(1, 10));
    q
}

fn fantasy_summary_rows(
    gaps: Option<&FantasyRosterGapView>,
    simulation: Option<&FantasySimulationView>,
    gaps_error: Option<String>,
) -> Vec<DashboardSummaryRow> {
    let Some(gaps) = gaps else {
        return vec![summary_row(
            "Fantasy",
            "Unavailable",
            gaps_error.unwrap_or_else(|| "Fantasy league import is not loaded".to_string()),
        )];
    };

    let add_now = gaps
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.action,
                icelines_core::view_model::FantasyRosterGapAction::AddNow
            )
        })
        .count();
    let watch = gaps
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.action,
                icelines_core::view_model::FantasyRosterGapAction::Watch
            )
        })
        .count();
    let mut rows = vec![summary_row(
        "Roster Gaps",
        gaps.rows.len().to_string(),
        format!("add now {} · watch {}", add_now, watch),
    )];

    if let Some(best) = gaps.rows.iter().find_map(|row| {
        row.best_available
            .as_ref()
            .map(|candidate| (row, candidate))
    }) {
        rows.push(summary_row(
            "Best Add",
            best.1.display_name.clone(),
            format!(
                "{} {} · {} · {:.1}",
                best.1.team, best.1.position, best.0.category, best.1.weighted_value
            ),
        ));
    }

    if let Some(simulation) = simulation {
        if let Some(user_row) = simulation.rows.iter().find(|row| row.is_user_team) {
            rows.push(summary_row(
                "Simulation",
                format!("#{}", user_row.rank),
                format!(
                    "{} pts · gap {:.1}",
                    user_row.projected_score, user_row.score_gap_to_leader
                ),
            ));
        }
        if let Some(scenario) = simulation.scenarios.first() {
            rows.push(summary_row(
                "Scenario",
                format!("{:+.1}", scenario.projected_score_delta),
                scenario.explanation.clone(),
            ));
        }
    }

    rows
}

async fn scores_workspace_summary(state: &WebState, path: &str) -> Vec<DashboardSummaryRow> {
    let q = scores_query_from_workspace(path);
    let result = super::scores::build_scores_result(state, &q).await;
    scores_summary_rows(&result)
}

fn scores_query_from_workspace(path: &str) -> super::scores::ScoresQuery {
    let mut q = super::scores::ScoresQuery::default();
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "date" => q.date = Some(value),
            "range" => q.range = Some(value),
            _ => {}
        }
    }
    q
}

fn goalies_query_from_workspace(path: &str) -> super::goalies::GoaliesQuery {
    let mut q = super::goalies::GoaliesQuery::default();
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "sort" => q.sort = Some(value),
            "top" => q.top = value.parse().ok(),
            "gp_min" | "min_gp" | "gp-min" | "min-gp" => q.gp_min = value.parse().ok(),
            "include_below_threshold" => {
                q.include_below_threshold = value.parse().ok();
            }
            _ => {}
        }
    }
    q
}

fn scores_summary_rows(result: &super::scores::ScoresResult) -> Vec<DashboardSummaryRow> {
    if let Some(error) = &result.fetch_error {
        return vec![summary_row("Scores", "Unavailable", error.clone())];
    }

    let games = result
        .days
        .iter()
        .flat_map(|day| day.rows.iter())
        .collect::<Vec<_>>();
    let live = games.iter().filter(|row| row.state_class == "live").count();
    let finals = games
        .iter()
        .filter(|row| row.state_class == "final")
        .count();
    let upcoming = games.len().saturating_sub(live + finals);
    let mut rows = vec![summary_row(
        "Slate",
        result.total_games.to_string(),
        format!(
            "{} · live {} · final {} · upcoming {}",
            result.active_date, live, finals, upcoming
        ),
    )
    .with_href(format!("/scores?date={}", result.active_date))];

    rows.extend(games.into_iter().take(2).map(score_game_summary_row));
    rows
}

fn score_game_summary_row(game: &ScoreRow) -> DashboardSummaryRow {
    let score = if game.away_score_str.is_empty() || game.home_score_str.is_empty() {
        game.start_time_label.clone()
    } else {
        format!("{}-{}", game.away_score_str, game.home_score_str)
    };
    summary_row(
        game.state_label.clone(),
        format!("{} @ {}", game.away_abbrev, game.home_abbrev),
        score,
    )
    .with_href(format!("/game/{}", game.game_id))
}

async fn schedule_workspace_summary(state: &WebState, path: &str) -> Vec<DashboardSummaryRow> {
    let q = schedule_query_from_workspace(path);
    let result = super::schedule::build_schedule_result(state, &q).await;
    schedule_summary_rows(&result)
}

fn schedule_query_from_workspace(path: &str) -> super::schedule::ScheduleQuery {
    let mut q = super::schedule::ScheduleQuery::default();
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "team" => q.team = Some(value),
            "date" => q.date = Some(value),
            _ => {}
        }
    }
    q
}

fn schedule_summary_rows(result: &super::schedule::ScheduleResult) -> Vec<DashboardSummaryRow> {
    if let Some(error) = &result.fetch_error {
        return vec![summary_row("Schedule", "Unavailable", error.clone())];
    }

    let scope = if result.active_team.is_empty() {
        result
            .active_date
            .clone()
            .unwrap_or_else(|| result.season_pretty.clone())
    } else {
        result.active_team.clone()
    };
    let mut rows = vec![summary_row(
        "Schedule",
        result.total.to_string(),
        format!("{scope} · {}", result.season_pretty),
    )
    .with_href("/schedule")];
    rows.extend(result.rows.iter().take(2).map(schedule_game_summary_row));
    rows
}

fn schedule_game_summary_row(game: &ScheduleRow) -> DashboardSummaryRow {
    let score = if game.away_score_str.is_empty() || game.home_score_str.is_empty() {
        game.state_label.clone()
    } else {
        format!(
            "{} {}-{} {}",
            game.away_abbrev, game.away_score_str, game.home_score_str, game.home_abbrev
        )
    };
    let venue = if game.home_or_away.is_empty() {
        game.date.clone()
    } else {
        format!("{} · {}", game.date, game.home_or_away)
    };
    summary_row(
        venue,
        format!("{} @ {}", game.away_abbrev, game.home_abbrev),
        score,
    )
    .with_href(format!("/game/{}", game.game_id))
}

async fn player_workspace_summary(
    state: &WebState,
    player_id_raw: &str,
) -> Vec<DashboardSummaryRow> {
    let Ok(player_id) = player_id_raw.parse::<u32>().map(PlayerId) else {
        return Vec::new();
    };
    let (season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(icelines_core::CURRENT_SEASON)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let repo = state.repo.read().await;
    let Some(view) = PlayerCardView::from_repository(&repo, player_id, season, season_type) else {
        return vec![summary_row(
            "Player",
            "Not found",
            format!("No player with NHL id {}", player_id.0),
        )];
    };
    player_summary_rows(&view)
}

fn player_summary_rows(view: &PlayerCardView) -> Vec<DashboardSummaryRow> {
    let Some(active) = &view.active else {
        return vec![summary_row(
            "Player",
            view.display_name.clone(),
            view.empty_state
                .as_ref()
                .and_then(|state| state.detail.clone())
                .unwrap_or_else(|| "No active-season row for this player".to_string()),
        )];
    };

    vec![
        summary_row(
            "Player",
            view.display_name.clone(),
            format!(
                "{} · {}",
                active.team_display,
                active.position.abbreviation()
            ),
        ),
        summary_row(
            "Scoring",
            metric_u32(&active.metrics, "points")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            format!(
                "{} G · {} A · {} GP",
                metric_u32(&active.metrics, "goals").unwrap_or(0),
                metric_u32(&active.metrics, "assists").unwrap_or(0),
                metric_u32(&active.metrics, "gp").unwrap_or(0)
            ),
        ),
        summary_row(
            "Rate",
            metric_f64(&active.metrics, "points_per_game")
                .map(|value| format!("{value:.2} PPG"))
                .unwrap_or_else(|| "-".to_string()),
            player_secondary_detail(active),
        ),
    ]
}

fn player_secondary_detail(active: &PlayerSeasonSummary) -> String {
    let shots = metric_u32(&active.metrics, "shots")
        .map(|value| format!("{value} shots"))
        .unwrap_or_else(|| "shots -".to_string());
    let plus_minus = metric_i32(&active.metrics, "plus_minus")
        .map(|value| format!("{value:+}"))
        .unwrap_or_else(|| "-".to_string());
    format!("{shots} · +/- {plus_minus}")
}

async fn game_workspace_summary(state: &WebState, game_id_raw: &str) -> Vec<DashboardSummaryRow> {
    let Ok(game_id) = game_id_raw.parse::<u64>() else {
        return Vec::new();
    };
    match super::game::build_game_detail(state, game_id).await {
        Ok(view) => game_summary_rows(&view),
        Err(error) => vec![summary_row("Game", "Unavailable", error)],
    }
}

fn game_summary_rows(view: &super::game::GameDetailView) -> Vec<DashboardSummaryRow> {
    let mut rows = vec![summary_row(
        "Game",
        format!("{} @ {}", view.away_abbrev, view.home_abbrev),
        format!(
            "{} {}-{} · {}",
            view.away_abbrev, view.away_score, view.home_score, view.state_label
        ),
    )];
    if let Some(goal) = view.goals.last() {
        rows.push(summary_row(
            "Latest Goal",
            goal.scorer_name.clone(),
            format!(
                "{} {} · {}-{}",
                goal.scorer_team, goal.time_in_period, goal.away_score, goal.home_score
            ),
        ));
    }
    if let Some(skater) = view
        .away_top_skaters
        .iter()
        .chain(view.home_top_skaters.iter())
        .max_by_key(|skater| (skater.points, skater.goals, skater.assists))
    {
        rows.push(summary_row(
            "Top Skater",
            skater.player_name.clone(),
            format!(
                "{} P · {} G · {} A",
                skater.points, skater.goals, skater.assists
            ),
        ));
    }
    rows
}

async fn transactions_workspace_summary(state: &WebState, path: &str) -> Vec<DashboardSummaryRow> {
    let q = transactions_query_from_workspace(path);
    match super::transactions::build_transactions_result(state, &q).await {
        Ok(result) => transactions_summary_rows(&result),
        Err(error) => vec![summary_row(
            "Transactions",
            error.title(),
            error.message().to_string(),
        )],
    }
}

fn transactions_query_from_workspace(path: &str) -> super::transactions::TransactionsQuery {
    let mut q = super::transactions::TransactionsQuery::default();
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "kind" => q.kind = Some(value),
            "team" => q.team = Some(value),
            _ => {}
        }
    }
    q
}

fn transactions_summary_rows(
    result: &super::transactions::TransactionsResult,
) -> Vec<DashboardSummaryRow> {
    let scope = match (result.active_kind.is_empty(), result.active_team.is_empty()) {
        (true, true) => result.season_pretty.clone(),
        (false, true) => format!("{} · {}", result.season_pretty, result.active_kind),
        (true, false) => format!("{} · {}", result.season_pretty, result.active_team),
        (false, false) => format!(
            "{} · {} · {}",
            result.season_pretty, result.active_team, result.active_kind
        ),
    };
    let mut rows = vec![summary_row(
        "Transactions",
        result.total.to_string(),
        if result.out_of_coverage {
            format!("coverage starts {}", result.earliest_season_pretty)
        } else {
            scope
        },
    )];
    rows.extend(result.rows.iter().take(2).map(transaction_summary_row));
    rows
}

fn transaction_summary_row(row: &TransactionRow) -> DashboardSummaryRow {
    summary_row(
        row.kind_pretty.clone(),
        if row.team.is_empty() {
            row.date.clone()
        } else {
            format!("{} · {}", row.date, row.team)
        },
        row.description.clone(),
    )
}

async fn playoffs_workspace_summary(state: &WebState) -> Vec<DashboardSummaryRow> {
    let result = super::playoffs::build_playoffs_result(state).await;
    playoffs_summary_rows(&result)
}

fn playoffs_summary_rows(result: &super::playoffs::PlayoffsResult) -> Vec<DashboardSummaryRow> {
    if let Some(error) = &result.fetch_error {
        return vec![summary_row("Playoffs", "Unavailable", error.clone())];
    }
    let series_count: usize = result.rounds.iter().map(|round| round.series.len()).sum();
    let mut rows = vec![summary_row(
        "Playoffs",
        series_count.to_string(),
        format!("{} · {}", result.season_pretty, result.source_label),
    )];
    if result.empty {
        rows[0].value = "No bracket".to_string();
    }
    rows.extend(
        result
            .rounds
            .iter()
            .flat_map(|round| round.series.iter())
            .take(2)
            .map(playoff_series_summary_row),
    );
    rows
}

fn playoff_series_summary_row(series: &PlayoffsSeriesView) -> DashboardSummaryRow {
    summary_row(
        series.conference.clone(),
        format!("{} vs {}", series.top_abbrev, series.bottom_abbrev),
        series.summary.clone(),
    )
}

async fn favorites_workspace_summary(state: &WebState) -> Vec<DashboardSummaryRow> {
    let context = dashboard_view_context(state).await;
    let members = super::favorites_data::read_group_members("Favorites");
    let view = FavoritesView::from_members(
        context,
        "Favorites".to_string(),
        favorite_member_inputs(&members),
        std::collections::HashMap::new(),
    );
    favorites_summary_rows(&view)
}

async fn watchlist_workspace_summary(state: &WebState) -> Vec<DashboardSummaryRow> {
    let context = dashboard_view_context(state).await;
    let members = super::favorites_data::read_group_members("Watchlist");
    let notes = super::favorites_data::read_watch_notes();
    let alerts = super::favorites_data::read_watch_alert_events(3);
    let view = WatchlistView::from_members(
        context,
        "Watchlist".to_string(),
        favorite_member_inputs(&members),
        notes
            .into_iter()
            .map(|(key, note)| {
                (
                    key,
                    WatchNoteInput {
                        reason: note.reason,
                        source: note.source,
                        updated_at: note.updated_at,
                    },
                )
            })
            .collect(),
    );
    watchlist_summary_rows(&view, alerts.len())
}

async fn dashboard_view_context(state: &WebState) -> ViewContext {
    let cfg = state.config.read().await;
    let season = cfg
        .active_season
        .parse::<u32>()
        .map(Season)
        .unwrap_or(Season(icelines_core::CURRENT_SEASON));
    let season_type = SeasonType::parse_lossy(&cfg.active_season_type);
    ViewContext::new(ViewWindow::new(season, season_type))
}

fn favorite_member_inputs(members: &[(String, String)]) -> Vec<FavoriteMemberInput> {
    members
        .iter()
        .map(|(kind, key)| FavoriteMemberInput {
            kind: kind.clone(),
            key: key.clone(),
        })
        .collect()
}

fn favorites_summary_rows(view: &FavoritesView) -> Vec<DashboardSummaryRow> {
    let mut rows = vec![summary_row(
        "Favorites",
        view.rows.len().to_string(),
        format!("{} players · {} teams", view.player_count, view.team_count),
    )];
    if view.rows.is_empty() {
        rows[0].value = "No rows".to_string();
        rows[0].detail = view
            .empty_state
            .as_ref()
            .and_then(|state| state.detail.clone())
            .unwrap_or_else(|| "No favorites saved yet".to_string());
        return rows;
    }
    rows.extend(view.rows.iter().take(2).map(|row| {
        summary_row(
            row.kind.clone(),
            row.key.clone(),
            row.stat_line.clone().unwrap_or_default(),
        )
    }));
    rows
}

fn watchlist_summary_rows(view: &WatchlistView, alert_count: usize) -> Vec<DashboardSummaryRow> {
    let mut rows = vec![summary_row(
        "Watchlist",
        view.rows.len().to_string(),
        format!(
            "{} players · {} teams · {} alerts",
            view.player_count, view.team_count, alert_count
        ),
    )];
    if view.rows.is_empty() {
        rows[0].value = "No rows".to_string();
        rows[0].detail = view
            .empty_state
            .as_ref()
            .and_then(|state| state.detail.clone())
            .unwrap_or_else(|| "No watchlist entries saved yet".to_string());
        return rows;
    }
    rows.extend(view.rows.iter().take(2).map(|row| {
        summary_row(
            row.kind.clone(),
            row.key.clone(),
            row.reason.clone().unwrap_or_else(|| {
                row.source
                    .clone()
                    .unwrap_or_else(|| "No watch note yet".to_string())
            }),
        )
    }));
    rows
}

async fn career_workspace_summary(path: &str) -> Vec<DashboardSummaryRow> {
    let q = career_query_from_workspace(path);
    match super::career::build_view(&q) {
        Ok(view) => career_summary_rows(&view),
        Err(error) => vec![summary_row("Career", "Unavailable", error)],
    }
}

fn career_query_from_workspace(path: &str) -> super::career::CareerQuery {
    let mut q = super::career::CareerQuery {
        league: Some("OHL".to_owned()),
        season: None,
        sort: Some("points".to_owned()),
        top: Some(DASHBOARD_PREVIEW_N),
    };
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return q;
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = value.replace('+', " ");
        match key {
            "league" => q.league = Some(value),
            "season" => q.season = Some(value),
            "sort" => q.sort = Some(value),
            "top" => q.top = value.parse::<usize>().ok(),
            _ => {}
        }
    }
    q.top = Some(q.top.unwrap_or(DASHBOARD_PREVIEW_N).clamp(1, 10));
    q
}

fn career_summary_rows(view: &CareerView) -> Vec<DashboardSummaryRow> {
    let mut rows = vec![summary_row(
        "Cohort",
        format!("{} {}", view.league, view.season),
        format!(
            "{} of {} rows - sorted by {}",
            view.rows.len(),
            view.total,
            view.sort.as_str()
        ),
    )];

    if view.rows.is_empty() {
        rows[0].value = "No rows".to_owned();
        rows[0].detail = "No matching career-history rows for this cohort".to_owned();
        return rows;
    }

    rows.extend(view.rows.iter().take(2).map(|row| {
        summary_row(
            format!("#{}", row.rank),
            row.name.clone(),
            format!(
                "{} - {} GP - {} PTS",
                row.team,
                row.gp,
                row.points
                    .map(|points| points.to_string())
                    .unwrap_or_else(|| "-".to_owned())
            ),
        )
    }));
    rows
}

async fn team_season_workspace_summary(
    state: &WebState,
    team_raw: &str,
) -> Vec<DashboardSummaryRow> {
    let Ok(team) = TeamAbbr::parse(team_raw) else {
        return Vec::new();
    };
    let (season_str, season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(icelines_core::CURRENT_SEASON)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let client = super::nhl_client();
    let standings = client
        .fetch_standings_now()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.to_team_standing_input())
        .collect();
    let games = client
        .fetch_team_season_schedule(&team.0, &season_str)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(super::schedule::scheduled_game_input)
        .collect();
    let view = TeamSeasonView::from_games_and_standings(
        ViewContext::new(ViewWindow::new(season, season_type)),
        season_str,
        team.0.to_string(),
        games,
        standings,
    );
    team_season_summary_rows(&view)
}

async fn team_depth_workspace_summary(
    state: &WebState,
    team_raw: &str,
) -> Vec<DashboardSummaryRow> {
    let Ok(team) = TeamAbbr::parse(team_raw) else {
        return Vec::new();
    };
    let (season, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season
                .parse::<u32>()
                .map(Season)
                .unwrap_or(Season(icelines_core::CURRENT_SEASON)),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let repo = state.repo.read().await;
    let view = TeamDepthView::from_repository(&repo, team, season, season_type);
    team_depth_summary_rows(&view)
}

fn team_depth_summary_rows(view: &TeamDepthView) -> Vec<DashboardSummaryRow> {
    let forward_slots = forward_slot_count(view);
    let defense_slots = defense_slot_count(view);
    let rostered = forward_slots + defense_slots + view.goalies.len() + view.extras.len();
    if rostered == 0 {
        return vec![summary_row(
            "Roster",
            "No rows",
            "No roster rows loaded for this team and season",
        )];
    }

    let mut rows = vec![summary_row(
        "Roster",
        rostered.to_string(),
        format!(
            "{} F · {} D · {} G",
            forward_slots,
            defense_slots,
            view.goalies.len()
        ),
    )];

    if let Some(line) = view.forward_lines.first() {
        let names = [
            line.left.as_ref(),
            line.center.as_ref(),
            line.right.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|slot| slot.display_name.as_str())
        .collect::<Vec<_>>();
        if !names.is_empty() {
            rows.push(summary_row(
                "Top Line",
                names.join(" / "),
                "estimated forward deployment",
            ));
        }
    }

    if let Some(pair) = view.defense_pairs.first() {
        let names = [pair.left.as_ref(), pair.right.as_ref()]
            .into_iter()
            .flatten()
            .map(|slot| slot.display_name.as_str())
            .collect::<Vec<_>>();
        if !names.is_empty() {
            rows.push(summary_row(
                "Top Pair",
                names.join(" / "),
                "estimated defense deployment",
            ));
        }
    }

    if !view.goalies.is_empty() {
        rows.push(summary_row(
            "Goalies",
            view.goalies
                .iter()
                .take(2)
                .map(|slot| slot.display_name.as_str())
                .collect::<Vec<_>>()
                .join(" / "),
            view.goalies
                .iter()
                .take(2)
                .map(|slot| slot.role.as_str())
                .collect::<Vec<_>>()
                .join(" · "),
        ));
    }

    if !view.extras.is_empty() {
        rows.push(summary_row(
            "Extras",
            view.extras.len().to_string(),
            view.extras
                .iter()
                .take(3)
                .map(|slot| slot.display_name.as_str())
                .collect::<Vec<_>>()
                .join(" / "),
        ));
    }

    rows
}

fn forward_slot_count(view: &TeamDepthView) -> usize {
    view.forward_lines
        .iter()
        .map(|line| {
            [
                line.left.as_ref(),
                line.center.as_ref(),
                line.right.as_ref(),
            ]
            .into_iter()
            .flatten()
            .count()
        })
        .sum()
}

fn defense_slot_count(view: &TeamDepthView) -> usize {
    view.defense_pairs
        .iter()
        .map(|pair| {
            [pair.left.as_ref(), pair.right.as_ref()]
                .into_iter()
                .flatten()
                .count()
        })
        .sum()
}

fn team_season_summary_rows(view: &TeamSeasonView) -> Vec<DashboardSummaryRow> {
    let mut rows = vec![
        summary_row(
            "Record",
            record_label(view.headline.record),
            format!(
                "{} pts · Pts% {:.3}",
                view.headline.points, view.headline.points_percentage
            ),
        ),
        summary_row(
            "Goal Diff",
            signed_i32(view.headline.goal_differential),
            format!(
                "GF-GA {}-{}",
                view.headline.goals_for, view.headline.goals_against
            ),
        ),
        summary_row(
            "SOS",
            pct_or_dash(view.schedule_strength.faced_average_points_percentage),
            format!(
                "remaining {}",
                pct_or_dash(view.schedule_strength.remaining_average_points_percentage)
            ),
        ),
        summary_row(
            "Ledger",
            format!("QW {}", view.quality_ledger.quality_wins),
            format!(
                "bad losses {} · missed pts {}",
                view.quality_ledger.bad_losses, view.quality_ledger.missed_points
            ),
        ),
    ];
    if let Some(standings) = &view.standings {
        rows.insert(
            1,
            summary_row(
                "Standings",
                standings.playoff_position_label.clone(),
                cutline_detail(
                    standings.points_above_cutline,
                    standings.points_behind_cutline,
                ),
            ),
        );
    }
    rows
}

fn summary_row(
    label: impl Into<String>,
    value: impl Into<String>,
    detail: impl Into<String>,
) -> DashboardSummaryRow {
    DashboardSummaryRow {
        label: label.into(),
        value: value.into(),
        detail: detail.into(),
        href: String::new(),
    }
}

trait DashboardSummaryRowExt {
    fn with_href(self, href: impl Into<String>) -> Self;
}

impl DashboardSummaryRowExt for DashboardSummaryRow {
    fn with_href(mut self, href: impl Into<String>) -> Self {
        self.href = href.into();
        self
    }
}

fn record_label(record: ScheduleRecord) -> String {
    format!(
        "{}-{}-{}",
        record.wins, record.losses, record.overtime_losses
    )
}

fn signed_i32(value: i32) -> String {
    format!("{value:+}")
}

fn pct_or_dash(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

fn cutline_detail(above: Option<i32>, behind: Option<i32>) -> String {
    if let Some(above) = above {
        format!("{above} pts above cutline")
    } else if let Some(behind) = behind {
        format!("{behind} pts behind cutline")
    } else {
        String::new()
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

async fn dashboard_favorites_entities(state: &WebState) -> Vec<DashboardEntityRow> {
    let context = dashboard_view_context(state).await;
    let members = super::favorites_data::read_group_members("Favorites");
    let stat_lines = super::favorites::compute_player_stat_lines(&members).await;
    let view = FavoritesView::from_members(
        context,
        "Favorites".to_string(),
        favorite_member_inputs(&members),
        stat_lines,
    );
    favorite_entity_rows(&view)
}

async fn dashboard_watchlist_entities(state: &WebState) -> Vec<DashboardEntityRow> {
    let context = dashboard_view_context(state).await;
    let members = super::favorites_data::read_group_members("Watchlist");
    let notes = super::favorites_data::read_watch_notes();
    let view = WatchlistView::from_members(
        context,
        "Watchlist".to_string(),
        favorite_member_inputs(&members),
        notes
            .into_iter()
            .map(|(key, note)| {
                (
                    key,
                    WatchNoteInput {
                        reason: note.reason,
                        source: note.source,
                        updated_at: note.updated_at,
                    },
                )
            })
            .collect(),
    );
    watchlist_entity_rows(&view)
}

fn favorite_entity_rows(view: &FavoritesView) -> Vec<DashboardEntityRow> {
    view.rows
        .iter()
        .take(8)
        .map(|row| {
            let route = match row.kind.as_str() {
                "team" => format!("/team/{}", row.key),
                _ => format!("/leaders?filter=name%3D{}", url_component(&row.key)),
            };
            DashboardEntityRow {
                label: row.key.clone(),
                href: dashboard_workspace_href(&route),
                kind: row
                    .stat_line
                    .clone()
                    .filter(|line| !line.is_empty())
                    .unwrap_or_else(|| row.kind.clone()),
            }
        })
        .collect()
}

fn watchlist_entity_rows(view: &WatchlistView) -> Vec<DashboardEntityRow> {
    view.rows
        .iter()
        .take(8)
        .map(|row| {
            let route = match row.kind.as_str() {
                "team" => format!("/team/{}", row.key),
                _ => format!("/leaders?filter=name%3D{}", url_component(&row.key)),
            };
            DashboardEntityRow {
                label: row.key.clone(),
                href: dashboard_workspace_href(&route),
                kind: row
                    .reason
                    .clone()
                    .or_else(|| row.source.clone())
                    .unwrap_or_else(|| row.kind.clone()),
            }
        })
        .collect()
}

fn dashboard_composition(q: &DashboardQuery, active: Option<WorkbenchId>) -> DashboardComposition {
    let experience = q
        .experience
        .as_deref()
        .and_then(web_experience_by_slug)
        .filter(|experience| active == Some(experience.center));
    let default_left = experience
        .and_then(|experience| experience.left_pane)
        .filter(|id| web_pane_binding_for_zone(*id, WorkbenchZone::LeftPane).is_some())
        .unwrap_or(DEFAULT_LEFT_PANE);
    let default_right = experience
        .and_then(|experience| experience.right_pane)
        .filter(|id| web_pane_binding_for_zone(*id, WorkbenchZone::RightPane).is_some())
        .unwrap_or(DEFAULT_RIGHT_PANE);
    let left_id = normalize_pane_binding(q.left.as_deref(), WorkbenchZone::LeftPane, default_left);
    let right_id =
        normalize_pane_binding(q.right.as_deref(), WorkbenchZone::RightPane, default_right);

    DashboardComposition {
        experience,
        left: web_pane_binding_for_zone(left_id, WorkbenchZone::LeftPane)
            .expect("normalized left pane binding must exist"),
        right: web_pane_binding_for_zone(right_id, WorkbenchZone::RightPane)
            .expect("normalized right pane binding must exist"),
    }
}

fn normalize_pane_binding(
    raw: Option<&str>,
    zone: WorkbenchZone,
    fallback: WorkbenchPaneBindingId,
) -> WorkbenchPaneBindingId {
    raw.and_then(|slug| {
        crate::workbench::web_pane_bindings_for_zone(zone)
            .find(|binding| binding.id.slug() == slug)
            .map(|binding| binding.id)
    })
    .unwrap_or(fallback)
}

fn web_pane_binding_for_zone(
    id: WorkbenchPaneBindingId,
    zone: WorkbenchZone,
) -> Option<&'static WorkbenchPaneBinding> {
    crate::workbench::web_pane_bindings_for_zone(zone).find(|binding| binding.id == id)
}

fn web_experience_by_slug(slug: &str) -> Option<&'static WorkbenchExperience> {
    crate::workbench::web_bound_experiences().find(|experience| experience.id.slug() == slug)
}

fn dashboard_catalog_groups(
    active: Option<WorkbenchId>,
    composition: &DashboardComposition,
) -> Vec<DashboardCatalogGroup> {
    let ready: Vec<_> = crate::workbench::dashboard_ready_workbenches().collect();
    [
        WorkbenchGroup::League,
        WorkbenchGroup::Analytics,
        WorkbenchGroup::Teams,
        WorkbenchGroup::Players,
        WorkbenchGroup::Live,
        WorkbenchGroup::MyBench,
        WorkbenchGroup::Fantasy,
        WorkbenchGroup::Reports,
        WorkbenchGroup::System,
    ]
    .into_iter()
    .filter_map(|group| {
        let entries = ready
            .iter()
            .filter_map(|(id, route)| {
                let entry = workbench_entry(*id)?;
                (entry.group == group)
                    .then(|| dashboard_catalog_entry(entry, route, active, composition))
            })
            .collect::<Vec<_>>();
        (!entries.is_empty()).then(|| DashboardCatalogGroup {
            label: workbench_group_label(group).to_owned(),
            entries,
        })
    })
    .collect()
}

fn dashboard_catalog_entry(
    entry: &WorkbenchEntry,
    route: &str,
    active: Option<WorkbenchId>,
    composition: &DashboardComposition,
) -> DashboardCatalogEntry {
    DashboardCatalogEntry {
        label: entry.label.to_owned(),
        href: dashboard_href(route, composition.left.id, composition.right.id, None),
        detail: format!(
            "{} · {} · {}",
            workbench_zone_label(entry.default_zone),
            document_kind_label(entry.document_kind),
            route
        ),
        is_active: active == Some(entry.id),
    }
}

fn dashboard_experience_tabs(
    active: Option<WorkbenchId>,
    active_experience: Option<&WorkbenchExperience>,
) -> Vec<DashboardExperienceTab> {
    WORKBENCH_EXPERIENCES
        .iter()
        .filter_map(|experience| dashboard_experience_tab(experience, active, active_experience))
        .collect()
}

fn dashboard_experience_tab(
    experience: &WorkbenchExperience,
    active: Option<WorkbenchId>,
    active_experience: Option<&WorkbenchExperience>,
) -> Option<DashboardExperienceTab> {
    let route = crate::workbench::route_for_workbench(experience.center)?;
    let left = experience
        .left_pane
        .map(workbench_pane_binding_label)
        .unwrap_or("no left pane");
    let right = experience
        .right_pane
        .map(workbench_pane_binding_label)
        .unwrap_or("no right pane");
    Some(DashboardExperienceTab {
        label: experience.label.to_owned(),
        href: dashboard_href(
            route,
            experience.left_pane.unwrap_or(DEFAULT_LEFT_PANE),
            experience.right_pane.unwrap_or(DEFAULT_RIGHT_PANE),
            Some(experience),
        ),
        detail: format!("{left} + {right}"),
        is_active: active_experience
            .map(|active| active.id == experience.id)
            .unwrap_or(active == Some(experience.center)),
    })
}

fn active_dashboard_fields(
    active: Option<WorkbenchId>,
    experience: Option<&WorkbenchExperience>,
) -> Vec<DashboardFieldRow> {
    if let Some(experience) = experience {
        return experience
            .fields
            .iter()
            .copied()
            .map(dashboard_field_row)
            .collect();
    }

    active
        .and_then(workbench_entry)
        .map(|entry| {
            entry
                .fields
                .iter()
                .copied()
                .map(dashboard_field_row)
                .collect()
        })
        .unwrap_or_default()
}

fn dashboard_pane_options(
    zone: WorkbenchZone,
    left: WorkbenchPaneBindingId,
    right: WorkbenchPaneBindingId,
    workspace_url: &str,
) -> Vec<DashboardPaneBindingRow> {
    crate::workbench::web_pane_bindings_for_zone(zone)
        .map(|binding| {
            let next_left = if zone == WorkbenchZone::LeftPane {
                binding.id
            } else {
                left
            };
            let next_right = if zone == WorkbenchZone::RightPane {
                binding.id
            } else {
                right
            };
            dashboard_pane_binding_row(
                binding,
                dashboard_href(workspace_url, next_left, next_right, None),
                binding.id
                    == if zone == WorkbenchZone::LeftPane {
                        left
                    } else {
                        right
                    },
            )
        })
        .collect()
}

fn dashboard_pane_binding_row(
    binding: &WorkbenchPaneBinding,
    href: String,
    is_active: bool,
) -> DashboardPaneBindingRow {
    DashboardPaneBindingRow {
        id: binding.id.slug().to_owned(),
        label: binding.label.to_owned(),
        href,
        kind: workbench_pane_binding_kind_label(binding).to_owned(),
        detail: binding
            .fields
            .iter()
            .copied()
            .map(workbench_field_label)
            .collect::<Vec<_>>()
            .join(", "),
        interaction: workbench_pane_interaction_label(binding.interaction).to_owned(),
        is_active,
    }
}

fn active_dashboard_pane_models(active: Option<WorkbenchId>) -> Vec<DashboardPaneModelRow> {
    active
        .and_then(workbench_entry)
        .map(|entry| {
            entry
                .pane_models
                .iter()
                .copied()
                .map(dashboard_pane_model_row)
                .collect()
        })
        .unwrap_or_default()
}

fn dashboard_pane_model_row(id: WorkbenchPaneModelId) -> DashboardPaneModelRow {
    let Some(pane) = workbench_pane_model(id) else {
        return DashboardPaneModelRow {
            label: id.slug().to_owned(),
            kind: "Pane".to_owned(),
            detail: "Unknown pane model".to_owned(),
        };
    };
    DashboardPaneModelRow {
        label: pane.label.to_owned(),
        kind: workbench_pane_kind_label(pane.kind).to_owned(),
        detail: pane
            .fields
            .iter()
            .copied()
            .map(workbench_field_label)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn dashboard_field_row(id: WorkbenchFieldId) -> DashboardFieldRow {
    let Some(field) = workbench_field(id) else {
        return DashboardFieldRow {
            label: id.slug().to_owned(),
            value_kind: "value".to_owned(),
            detail: "Unknown field".to_owned(),
        };
    };
    DashboardFieldRow {
        label: field.label.to_owned(),
        value_kind: workbench_value_kind_label(field.value_kind).to_owned(),
        detail: format!(
            "{} · {}",
            workbench_field_source_label(field.source),
            workbench_field_summary_label(field.summary)
        ),
    }
}

fn workbench_id_for_workspace(path: &str) -> Option<WorkbenchId> {
    if path == "/leaders" || path == "/" {
        return Some(WorkbenchId::Stats);
    }
    if let Some(id) = crate::workbench::dashboard_ready_workbenches()
        .find_map(|(id, route)| (path == route).then_some(id))
    {
        return Some(id);
    }
    crate::workbench::dashboard_ready_workbenches().find_map(|(id, route)| {
        (workspace_route_key(path) == workspace_route_key(route)).then_some(id)
    })
}

fn workbench_pane_binding_label(id: WorkbenchPaneBindingId) -> &'static str {
    workbench_pane_binding(id)
        .map(|binding| binding.label)
        .unwrap_or_else(|| id.slug())
}

fn workbench_pane_binding_kind_label(binding: &WorkbenchPaneBinding) -> &'static str {
    workbench_pane_model(binding.pane_model)
        .map(|pane| workbench_pane_kind_label(pane.kind))
        .unwrap_or("Pane")
}

fn workbench_pane_interaction_label(interaction: WorkbenchPaneInteraction) -> &'static str {
    match interaction {
        WorkbenchPaneInteraction::ReadOnly => "Read-only",
        WorkbenchPaneInteraction::LocalState => "Local state",
        WorkbenchPaneInteraction::PostBackedActionStatus => "POST-backed actions",
    }
}

fn workbench_field_label(id: WorkbenchFieldId) -> &'static str {
    workbench_field(id)
        .map(|field| field.label)
        .unwrap_or_else(|| id.slug())
}

fn workbench_group_label(group: WorkbenchGroup) -> &'static str {
    match group {
        WorkbenchGroup::League => "League",
        WorkbenchGroup::Analytics => "Analytics",
        WorkbenchGroup::Teams => "Teams",
        WorkbenchGroup::Players => "Players",
        WorkbenchGroup::Live => "Live",
        WorkbenchGroup::MyBench => "My Bench",
        WorkbenchGroup::Fantasy => "Fantasy",
        WorkbenchGroup::Reports => "Reports",
        WorkbenchGroup::System => "System",
    }
}

fn workbench_zone_label(zone: icelines_core::WorkbenchZone) -> &'static str {
    match zone {
        icelines_core::WorkbenchZone::ActivityRail => "Activity rail",
        icelines_core::WorkbenchZone::Center => "Center workspace",
        icelines_core::WorkbenchZone::LeftPane => "Left pane",
        icelines_core::WorkbenchZone::RightPane => "Right pane",
        icelines_core::WorkbenchZone::TopRibbon => "Top ribbon",
        icelines_core::WorkbenchZone::BottomStatus => "Command/status",
        icelines_core::WorkbenchZone::Overlay => "Overlay",
    }
}

fn document_kind_label(kind: icelines_core::WorkbenchDocumentKind) -> &'static str {
    match kind {
        icelines_core::WorkbenchDocumentKind::Main => "main",
        icelines_core::WorkbenchDocumentKind::Drilldown => "drilldown",
        icelines_core::WorkbenchDocumentKind::Context => "context",
        icelines_core::WorkbenchDocumentKind::Admin => "admin",
        icelines_core::WorkbenchDocumentKind::Docs => "docs",
    }
}

fn workbench_pane_kind_label(kind: WorkbenchPaneKind) -> &'static str {
    match kind {
        WorkbenchPaneKind::Navigator => "Navigator",
        WorkbenchPaneKind::Inspector => "Inspector",
        WorkbenchPaneKind::Filter => "Filter/dimension",
        WorkbenchPaneKind::Summary => "Summary/KPI",
        WorkbenchPaneKind::Timeline => "Timeline",
        WorkbenchPaneKind::Compare => "Compare",
        WorkbenchPaneKind::Queue => "Queue/checklist",
        WorkbenchPaneKind::SourceState => "Source/data state",
        WorkbenchPaneKind::ActionStatus => "Action/status",
        WorkbenchPaneKind::Help => "Help",
    }
}

fn workbench_value_kind_label(kind: WorkbenchValueKind) -> &'static str {
    match kind {
        WorkbenchValueKind::Bool => "boolean",
        WorkbenchValueKind::Integer => "integer",
        WorkbenchValueKind::Decimal => "decimal",
        WorkbenchValueKind::Text => "text",
        WorkbenchValueKind::Date => "date",
        WorkbenchValueKind::EntityRef => "entity",
        WorkbenchValueKind::Enum => "enum",
        WorkbenchValueKind::Route => "route",
    }
}

fn workbench_field_source_label(source: WorkbenchFieldSource) -> &'static str {
    match source {
        WorkbenchFieldSource::ViewModel => "view model",
        WorkbenchFieldSource::RouteSummary => "route summary",
        WorkbenchFieldSource::Catalog => "catalog",
        WorkbenchFieldSource::CommandResult => "command result",
    }
}

fn workbench_field_summary_label(summary: WorkbenchFieldSummary) -> &'static str {
    match summary {
        WorkbenchFieldSummary::None => "raw",
        WorkbenchFieldSummary::Count => "count",
        WorkbenchFieldSummary::MinMax => "range",
        WorkbenchFieldSummary::Latest => "latest",
        WorkbenchFieldSummary::Status => "status",
        WorkbenchFieldSummary::Sparkline => "sparkline",
    }
}

fn schedule_links() -> Vec<DashboardLinkRow> {
    vec![
        DashboardLinkRow {
            label: "Scores".to_owned(),
            href: dashboard_workspace_href("/scores"),
            detail: "today's games".to_owned(),
        },
        DashboardLinkRow {
            label: "Schedule".to_owned(),
            href: dashboard_workspace_href("/schedule"),
            detail: "date and team views".to_owned(),
        },
        DashboardLinkRow {
            label: "Playoffs".to_owned(),
            href: dashboard_workspace_href("/playoffs"),
            detail: "bracket context".to_owned(),
        },
    ]
}

fn workspace_links(active: &str) -> Vec<DashboardLinkRow> {
    let route = workspace_route_key(active);
    let mut rows = contextual_workspace_links(route);
    rows.extend(
        crate::workbench::dashboard_ready_workbenches().filter_map(|(id, href)| {
            let entry = workbench_entry(id)?;
            Some(DashboardLinkRow {
                label: entry.label.to_owned(),
                href: dashboard_workspace_href(href),
                detail: if route == workspace_route_key(href) {
                    format!("{} - active", workbench_pane_detail(entry))
                } else {
                    workbench_pane_detail(entry)
                },
            })
        }),
    );
    rows
}

fn workbench_pane_detail(entry: &WorkbenchEntry) -> String {
    let pane_count = entry.pane_models.len();
    let field_count = entry.fields.len();
    format!("{pane_count} pane models · {field_count} fields")
}

fn contextual_workspace_links(route: &str) -> Vec<DashboardLinkRow> {
    match route {
        "/fantasy" => vec![
            workspace_action_link(
                "Goals + shots gaps",
                "/fantasy?category=goals,shots&top=8",
                "waiver-gap view",
            ),
            workspace_action_link("Four-week sim", "/fantasy?weeks=4", "league projection"),
            workspace_action_link(
                "Imported free agents",
                "/poach?availability=imported-available&top=12",
                "poacher board",
            ),
        ],
        "/poach" => vec![
            workspace_action_link(
                "Imported free agents",
                "/poach?availability=imported-available&top=12",
                "waiver-wire candidates",
            ),
            workspace_action_link(
                "Watched players",
                "/poach?availability=watched",
                "watchlist filter",
            ),
            workspace_action_link(
                "Category gaps",
                "/fantasy?category=goals,shots&top=8",
                "fantasy roster needs",
            ),
            workspace_action_link(
                "Poach report",
                "/reports/poach?availability=imported-available&top=12",
                "shareable waiver board",
            ),
            workspace_action_link(
                "Weekly prep",
                "/reports/weekly?availability=imported-available&top=12",
                "watchlist-aware report",
            ),
        ],
        _ => Vec::new(),
    }
}

fn workspace_action_link(label: &str, href: &str, detail: &str) -> DashboardLinkRow {
    DashboardLinkRow {
        label: label.to_owned(),
        href: dashboard_workspace_href(href),
        detail: detail.to_owned(),
    }
}

fn dashboard_href(
    path: &str,
    left: WorkbenchPaneBindingId,
    right: WorkbenchPaneBindingId,
    experience: Option<&WorkbenchExperience>,
) -> String {
    let mut href = format!(
        "/dashboard?workspace={}&left={}&right={}",
        url_component(path),
        url_component(left.slug()),
        url_component(right.slug())
    );
    if let Some(experience) = experience {
        href.push_str("&experience=");
        href.push_str(&url_component(experience.id.slug()));
    }
    href
}

fn dashboard_workspace_href(path: &str) -> String {
    format!("/dashboard?workspace={}", url_component(path))
}

fn url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            _ => {
                let hex = format!("%{byte:02X}");
                hex.chars().collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_dashboard_workspace_rejects_external_or_internal_api_paths() {
        assert_eq!(normalize_workspace(Some("/poach")), "/poach");
        assert_eq!(normalize_workspace(Some("//evil.example")), "/leaders");
        assert_eq!(normalize_workspace(Some("/api/v1/leaders")), "/leaders");
        assert_eq!(normalize_workspace(Some("/static/style.css")), "/leaders");
        assert_eq!(normalize_workspace(Some("/admin")), "/admin");
        assert_eq!(normalize_workspace(Some("/dashboard")), "/leaders");
        assert_eq!(normalize_workspace(Some("/favorites/add")), "/leaders");
        assert_eq!(
            normalize_workspace(Some("/player/8478402")),
            "/player/8478402"
        );
        assert_eq!(
            normalize_workspace(Some("/player/8478402/awards")),
            "/leaders"
        );
        assert_eq!(normalize_workspace(Some("/team/EDM")), "/team/EDM");
        assert_eq!(
            normalize_workspace(Some("/team/EDM/season")),
            "/team/EDM/season"
        );
        assert_eq!(normalize_workspace(Some("/team/EDM/streaks")), "/leaders");
        assert_eq!(
            normalize_workspace(Some("/records/player/8478402")),
            "/leaders"
        );
        assert_eq!(
            normalize_workspace(Some("/season-type/playoff")),
            "/leaders"
        );
        assert_eq!(
            normalize_workspace(Some("https://evil.example")),
            "/leaders"
        );
    }

    #[test]
    fn l0_dashboard_workspace_labels_known_routes() {
        assert_eq!(workspace_label("/team/EDM"), "Team Depth");
        assert_eq!(workspace_label("/team/EDM/season"), "Team Season");
        assert_eq!(workspace_label("/player/8478402"), "Player Card");
        assert_eq!(workspace_label("/career?league=OHL"), "Career Cohorts");
        assert_eq!(workspace_label("/reports/poach"), "Poach Report");
        assert_eq!(
            workspace_label("/reports/weekly?category=shots"),
            "Weekly Report"
        );
        assert_eq!(
            workspace_label("/poach?availability=imported-available"),
            "Poach"
        );
        assert_eq!(workspace_label("/docs"), "Docs");
    }

    #[test]
    fn l0_dashboard_workspace_links_preserve_dashboard_state() {
        let links = workspace_links("/poach?availability=imported-available");
        let poach = links
            .iter()
            .find(|row| row.label == "Poach")
            .expect("poach workspace link");
        assert_eq!(poach.href, "/dashboard?workspace=%2Fpoach");
        assert_eq!(poach.detail, "1 pane models · 3 fields - active");

        assert_eq!(
            dashboard_workspace_href("/poach?availability=imported-available"),
            "/dashboard?workspace=%2Fpoach%3Favailability%3Dimported-available"
        );

        let career = links
            .iter()
            .find(|row| row.label == "Career cohorts")
            .expect("career workspace link");
        assert_eq!(career.href, "/dashboard?workspace=%2Fcareer");
    }

    #[test]
    fn l0_dashboard_catalog_uses_shared_workbench_adapter() {
        let q = DashboardQuery::default();
        let composition = dashboard_composition(&q, Some(WorkbenchId::Stats));
        let groups = dashboard_catalog_groups(Some(WorkbenchId::Stats), &composition);
        let entries: Vec<_> = groups
            .iter()
            .flat_map(|group| group.entries.iter())
            .collect();

        let stats = entries
            .iter()
            .find(|entry| entry.label == "Stats")
            .expect("shared Stats catalog entry");
        assert_eq!(
            stats.href,
            "/dashboard?workspace=%2Fleaders&left=favorites-left&right=schedule-right"
        );
        assert!(stats.is_active);

        let admin = entries
            .iter()
            .find(|entry| entry.label == "Admin")
            .expect("shared Admin catalog entry");
        assert_eq!(
            admin.href,
            "/dashboard?workspace=%2Fadmin&left=favorites-left&right=schedule-right"
        );
        assert!(
            entries
                .iter()
                .all(|entry| entry.href.starts_with("/dashboard?workspace=")),
            "catalog links must stay inside dashboard GET navigation"
        );
    }

    #[test]
    fn l0_dashboard_bound_experience_tabs_are_composed_layouts() {
        let active = web_experience_by_slug("tonight-bench");
        let tabs = dashboard_experience_tabs(Some(WorkbenchId::Scores), active);
        let tonight = tabs
            .iter()
            .find(|tab| tab.label == "Tonight bench")
            .expect("Tonight bench tab");

        assert!(tonight.is_active);
        assert_eq!(
            tonight.href,
            "/dashboard?workspace=%2Fscores&left=favorites-left&right=schedule-right&experience=tonight-bench"
        );
        assert!(tonight.detail.contains("Favorites"));
        assert!(tonight.detail.contains("Schedule"));
    }

    #[test]
    fn l0_dashboard_active_fields_and_panes_use_shared_metadata() {
        let fields = active_dashboard_fields(Some(WorkbenchId::Scores), None);
        assert!(fields.iter().any(|field| field.label == "Date"));
        assert!(fields.iter().any(|field| field.label == "Game state"));

        let panes = active_dashboard_pane_models(Some(WorkbenchId::Scores));
        assert!(panes
            .iter()
            .any(|pane| pane.label == "Favorites navigator" && pane.kind == "Navigator"));
        assert!(panes
            .iter()
            .any(|pane| pane.label == "Schedule inspector" && pane.kind == "Timeline"));
    }

    #[test]
    fn l0_dashboard_composition_query_is_allowlisted() {
        let q = DashboardQuery {
            workspace: Some("/scores".to_owned()),
            left: Some("watchlist-left".to_owned()),
            right: Some("data-source-right".to_owned()),
            experience: Some("tonight-bench".to_owned()),
            ..Default::default()
        };

        let composition = dashboard_composition(&q, Some(WorkbenchId::Scores));

        assert_eq!(composition.left.id, WorkbenchPaneBindingId::WatchlistLeft);
        assert_eq!(
            composition.right.id,
            WorkbenchPaneBindingId::DataSourceRight
        );
        assert_eq!(
            composition
                .experience
                .map(|experience| experience.id.slug()),
            Some("tonight-bench")
        );

        let unsafe_q = DashboardQuery {
            left: Some("//evil.example".to_owned()),
            right: Some("/admin".to_owned()),
            experience: Some("admin-room".to_owned()),
            ..Default::default()
        };
        let fallback = dashboard_composition(&unsafe_q, Some(WorkbenchId::Scores));
        assert_eq!(fallback.left.id, WorkbenchPaneBindingId::FavoritesLeft);
        assert_eq!(fallback.right.id, WorkbenchPaneBindingId::ScheduleRight);
        assert!(fallback.experience.is_none());
    }

    #[test]
    fn l0_dashboard_pane_options_are_safe_get_navigation() {
        let options = dashboard_pane_options(
            WorkbenchZone::RightPane,
            WorkbenchPaneBindingId::FavoritesLeft,
            WorkbenchPaneBindingId::ScheduleRight,
            "/scores",
        );
        let data_source = options
            .iter()
            .find(|row| row.id == "data-source-right")
            .expect("data/source option");

        assert_eq!(data_source.label, "Data/source");
        assert_eq!(data_source.interaction, "POST-backed actions");
        assert!(data_source
            .href
            .starts_with("/dashboard?workspace=%2Fscores"));
        assert!(data_source.href.contains("&right=data-source-right"));
        assert!(!data_source.href.contains("/command"));
    }

    #[test]
    fn l0_dashboard_workspace_links_add_fantasy_poach_actions() {
        let fantasy_links = workspace_links("/fantasy");
        let sim = fantasy_links
            .iter()
            .find(|row| row.label == "Four-week sim")
            .expect("fantasy simulation quick action");
        assert_eq!(sim.href, "/dashboard?workspace=%2Ffantasy%3Fweeks%3D4");

        let poach_links = workspace_links("/poach");
        let imported = poach_links
            .iter()
            .find(|row| row.label == "Imported free agents")
            .expect("poach imported-free-agents quick action");
        assert_eq!(
            imported.href,
            "/dashboard?workspace=%2Fpoach%3Favailability%3Dimported-available%26top%3D12"
        );
        let weekly = poach_links
            .iter()
            .find(|row| row.label == "Weekly prep")
            .expect("poach weekly report quick action");
        assert_eq!(
            weekly.href,
            "/dashboard?workspace=%2Freports%2Fweekly%3Favailability%3Dimported-available%26top%3D12"
        );

        let leaders_links = workspace_links("/leaders");
        assert!(
            leaders_links.iter().all(|row| row.label != "Four-week sim"),
            "contextual fantasy actions should not appear on generic workspaces"
        );
    }

    #[test]
    fn l0_dashboard_team_season_summary_projects_viewmodel() {
        let view = TeamSeasonView::from_games(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "20252026".to_string(),
            "EDM".to_string(),
            Vec::new(),
        );

        let rows = team_season_summary_rows(&view);

        assert!(rows.iter().any(|row| row.label == "Record"));
        assert!(rows.iter().any(|row| row.label == "SOS"));
        assert!(rows.iter().any(|row| row.label == "Ledger"));
    }

    #[test]
    fn l0_dashboard_career_summary_projects_viewmodel() {
        let view = CareerView {
            context: ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            league: "OHL".to_owned(),
            season: 20142015,
            sort: icelines_core::CareerSortKey::Points,
            rows: vec![icelines_core::CareerRow {
                rank: 1,
                player_id: 8478402,
                name: "Connor McDavid".to_owned(),
                team: "ER".to_owned(),
                gp: 47,
                goals: Some(44),
                assists: Some(76),
                points: Some(120),
                points_per_game: Some(2.55),
            }],
            count: 1,
            total: 12,
            warnings: Vec::new(),
            empty_state: None,
        };

        let rows = career_summary_rows(&view);

        assert_eq!(rows[0].label, "Cohort");
        assert_eq!(rows[0].value, "OHL 20142015");
        assert!(rows[0].detail.contains("1 of 12 rows"));
        assert_eq!(rows[1].label, "#1");
        assert_eq!(rows[1].value, "Connor McDavid");
    }

    #[test]
    fn l0_dashboard_team_depth_summary_projects_empty_viewmodel() {
        let view = TeamDepthView::from_player_views(
            TeamAbbr("EDM".to_string()),
            Season(20252026),
            SeasonType::Regular,
            &[],
        );

        let rows = team_depth_summary_rows(&view);

        assert_eq!(rows[0].label, "Roster");
        assert_eq!(rows[0].value, "No rows");
        assert_eq!(forward_slot_count(&view), 0);
        assert_eq!(defense_slot_count(&view), 0);
    }

    #[test]
    fn l0_dashboard_depth_summary_projects_league_viewmodel() {
        let view = DepthLeagueView {
            context: ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            scoring_mode: "Pts/82".to_string(),
            rows: vec![DepthTeamStrengthRow {
                team: TeamAbbr("EDM".to_string()),
                c_score: 88.0,
                lw_score: 74.0,
                rw_score: 72.0,
                d_score: 80.0,
                total: 314.0,
                c_top: "Connor McDavid".to_string(),
                lw_top: "Ryan Nugent-Hopkins".to_string(),
                rw_top: "Zach Hyman".to_string(),
                d_top: "Evan Bouchard".to_string(),
            }],
            warnings: Vec::new(),
        };

        let rows = depth_summary_rows(&view);

        assert_eq!(rows[0].label, "#1");
        assert_eq!(rows[0].value, "EDM");
        assert!(rows[0].detail.contains("total 314"));
        assert!(rows[0].detail.contains("Connor McDavid"));
    }

    #[test]
    fn l0_dashboard_poach_summary_projects_empty_viewmodel() {
        let query = icelines_core::view_model::PoachQuery::new(
            Season(20252026),
            SeasonType::Regular,
            "yahoo-standard",
        );
        let view = PoachBoardView::new(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            query,
            "yahoo-standard",
        );

        let rows = poach_summary_rows(&view);

        assert_eq!(rows[0].label, "Candidates");
        assert_eq!(rows[0].value, "No rows");
        assert!(rows[0].detail.contains("No poach candidates"));
    }

    #[test]
    fn l0_dashboard_poach_workspace_query_preserves_filters() {
        let query = poach_query_from_workspace(
            "/poach?availability=imported_available&category=hits,blocks&team=SEA&top=99",
        );

        assert_eq!(query.availability.as_deref(), Some("imported_available"));
        assert_eq!(query.categories.as_deref(), Some("hits,blocks"));
        assert_eq!(query.team.as_deref(), Some("SEA"));
        assert_eq!(query.top, Some(10));
    }

    #[test]
    fn l0_dashboard_fantasy_workspace_query_preserves_filters() {
        let query = fantasy_query_from_workspace(
            "/fantasy?league=home&category=goals,shots&add_player=Player+One&drop_player=Bench&top=99&weeks=3",
        );

        assert_eq!(query.league.as_deref(), Some("home"));
        assert_eq!(query.categories.as_deref(), Some("goals,shots"));
        assert_eq!(query.add_player.as_deref(), Some("Player One"));
        assert_eq!(query.drop_player.as_deref(), Some("Bench"));
        assert_eq!(query.top, Some(10));
        assert_eq!(query.weeks, Some(3));
    }

    #[test]
    fn l0_dashboard_fantasy_summary_reports_unavailable_import() {
        let rows = fantasy_summary_rows(None, None, Some("fantasy db missing".to_string()));

        assert_eq!(rows[0].label, "Fantasy");
        assert_eq!(rows[0].value, "Unavailable");
        assert_eq!(rows[0].detail, "fantasy db missing");
    }

    #[test]
    fn l0_dashboard_scores_summary_counts_game_states() {
        let result = super::super::scores::ScoresResult {
            active_label: "25-26 · Regular".to_string(),
            active_date: "2026-05-13".to_string(),
            prev_date: "2026-05-06".to_string(),
            next_date: "2026-05-20".to_string(),
            today_date: "2026-05-13".to_string(),
            range: "day".to_string(),
            days: vec![crate::templates::ScoresDay {
                date: "2026-05-13".to_string(),
                date_pretty: "Wed, May 13".to_string(),
                rows: vec![ScoreRow {
                    game_id: 2025020001,
                    away_abbrev: "EDM".to_string(),
                    away_name: "Oilers".to_string(),
                    home_abbrev: "SEA".to_string(),
                    home_name: "Kraken".to_string(),
                    away_score_str: "3".to_string(),
                    home_score_str: "2".to_string(),
                    state_label: "FINAL".to_string(),
                    state_class: "final".to_string(),
                    start_time_label: String::new(),
                    is_playoff: false,
                    series_context: String::new(),
                }],
            }],
            total_games: 1,
            fetch_error: None,
        };

        let rows = scores_summary_rows(&result);

        assert_eq!(rows[0].label, "Slate");
        assert_eq!(rows[0].value, "1");
        assert!(rows[0].detail.contains("final 1"));
        assert_eq!(rows[0].href, "/scores?date=2026-05-13");
        assert_eq!(rows[1].value, "EDM @ SEA");
        assert_eq!(rows[1].href, "/game/2025020001");
    }

    #[test]
    fn l0_dashboard_leader_rows_link_to_player_cards() {
        let row = summary_row("#1", "Connor McDavid", "EDM 120 pts · 44 G")
            .with_href(format!("/player/{}", PlayerId(8478402).0));

        assert_eq!(row.href, "/player/8478402");
    }

    #[test]
    fn l0_dashboard_schedule_query_and_summary_preserve_team() {
        let query = schedule_query_from_workspace("/schedule?team=SEA&date=2026-05-13");
        assert_eq!(query.team.as_deref(), Some("SEA"));
        assert_eq!(query.date.as_deref(), Some("2026-05-13"));

        let result = super::super::schedule::ScheduleResult {
            active_label: "25-26 · Regular".to_string(),
            season_pretty: "2025-26".to_string(),
            active_team: "SEA".to_string(),
            active_date: None,
            team_chips: Vec::new(),
            rows: vec![ScheduleRow {
                game_id: 2025020001,
                date: "2026-05-13".to_string(),
                away_abbrev: "EDM".to_string(),
                home_abbrev: "SEA".to_string(),
                away_score_str: String::new(),
                home_score_str: String::new(),
                state_label: "FUT".to_string(),
                home_or_away: "home".to_string(),
                opponent_abbrev: "EDM".to_string(),
                is_playoff: false,
            }],
            total: 1,
            fetch_error: None,
        };

        let rows = schedule_summary_rows(&result);

        assert_eq!(rows[0].label, "Schedule");
        assert_eq!(rows[0].detail, "SEA · 2025-26");
        assert_eq!(rows[0].href, "/schedule");
        assert_eq!(rows[1].value, "EDM @ SEA");
        assert_eq!(rows[1].href, "/game/2025020001");
    }

    #[test]
    fn l0_dashboard_player_summary_handles_no_active_row() {
        let view = PlayerCardView {
            context: ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            player_id: PlayerId(8478402),
            display_name: "Connor McDavid".to_string(),
            headshot_url: None,
            active: None,
            career: Vec::new(),
            pre_nhl_career: Vec::new(),
            warnings: Vec::new(),
            empty_state: None,
        };

        let rows = player_summary_rows(&view);

        assert_eq!(rows[0].label, "Player");
        assert_eq!(rows[0].value, "Connor McDavid");
        assert!(rows[0].detail.contains("No active-season row"));
    }

    #[test]
    fn l0_dashboard_game_summary_projects_detail_view() {
        let view = super::super::game::GameDetailView {
            game_id: 2025020001,
            away_abbrev: "EDM".to_string(),
            home_abbrev: "SEA".to_string(),
            away_score: 4,
            home_score: 3,
            state_label: "FINAL/OT".to_string(),
            is_live: false,
            auto_refresh: false,
            goalies: Vec::new(),
            goals: vec![super::super::game::GameGoalView {
                period: 4,
                period_type: "OT".to_string(),
                time_in_period: "02:11".to_string(),
                scorer_team: "EDM".to_string(),
                scorer_name: "Leon Draisaitl".to_string(),
                assist1_name: None,
                assist2_name: None,
                away_score: 4,
                home_score: 3,
            }],
            away_top_skaters: vec![super::super::game::GameSkaterView {
                player_id: 8478402,
                player_name: "Connor McDavid".to_string(),
                position: "C".to_string(),
                goals: 1,
                assists: 2,
                points: 3,
                plus_minus: 1,
            }],
            home_top_skaters: Vec::new(),
        };

        let rows = game_summary_rows(&view);

        assert_eq!(rows[0].label, "Game");
        assert_eq!(rows[0].value, "EDM @ SEA");
        assert_eq!(rows[1].value, "Leon Draisaitl");
        assert_eq!(rows[2].value, "Connor McDavid");
    }

    #[test]
    fn l0_dashboard_transactions_query_and_summary_preserve_filters() {
        let query = transactions_query_from_workspace("/transactions?kind=trade&team=SEA");
        assert_eq!(query.kind.as_deref(), Some("trade"));
        assert_eq!(query.team.as_deref(), Some("SEA"));

        let result = super::super::transactions::TransactionsResult {
            active_label: "25-26 · Regular".to_string(),
            season_pretty: "2025-26".to_string(),
            rows: vec![TransactionRow {
                date: "2026-02-01".to_string(),
                team: "SEA".to_string(),
                kind_label: "trade".to_string(),
                kind_pretty: "Trade".to_string(),
                description: "SEA acquired a scorer".to_string(),
            }],
            total: 1,
            empty_unfiltered: false,
            active_kind: "trade".to_string(),
            active_team: "SEA".to_string(),
            out_of_coverage: false,
            earliest_season_pretty: "2023-24".to_string(),
        };

        let rows = transactions_summary_rows(&result);

        assert_eq!(rows[0].label, "Transactions");
        assert_eq!(rows[0].detail, "2025-26 · SEA · trade");
        assert_eq!(rows[1].value, "2026-02-01 · SEA");
    }

    #[test]
    fn l0_dashboard_playoffs_summary_projects_series() {
        let result = super::super::playoffs::PlayoffsResult {
            active_label: "25-26 · Playoffs".to_string(),
            season_pretty: "2025-26".to_string(),
            source_label: "historical bundle".to_string(),
            rounds: vec![crate::templates::PlayoffsRoundView {
                round_number: 1,
                label: "Round 1".to_string(),
                series: vec![PlayoffsSeriesView {
                    top_abbrev: "EDM".to_string(),
                    top_name: "Oilers".to_string(),
                    top_wins: 4,
                    bottom_abbrev: "SEA".to_string(),
                    bottom_name: "Kraken".to_string(),
                    bottom_wins: 2,
                    summary: "EDM wins 4-2".to_string(),
                    is_complete: true,
                    conference: "Western".to_string(),
                }],
            }],
            empty: false,
            fetch_error: None,
        };

        let rows = playoffs_summary_rows(&result);

        assert_eq!(rows[0].label, "Playoffs");
        assert_eq!(rows[0].value, "1");
        assert_eq!(rows[1].value, "EDM vs SEA");
    }

    #[test]
    fn l0_dashboard_favorites_summary_projects_viewmodel() {
        let view = FavoritesView::from_members(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "Favorites".to_string(),
            vec![
                FavoriteMemberInput {
                    kind: "player".to_string(),
                    key: "connor mcdavid".to_string(),
                },
                FavoriteMemberInput {
                    kind: "team".to_string(),
                    key: "SEA".to_string(),
                },
            ],
            std::collections::HashMap::new(),
        );

        let rows = favorites_summary_rows(&view);

        assert_eq!(rows[0].label, "Favorites");
        assert_eq!(rows[0].value, "2");
        assert_eq!(rows[0].detail, "1 players · 1 teams");
    }

    #[test]
    fn l0_dashboard_poach_report_summary_projects_viewmodel() {
        let report = PoachReportView {
            context: icelines_core::ReportContext {
                kind: icelines_core::ReportKind::Weekly,
                view_context: ViewContext::new(ViewWindow::new(
                    Season(20252026),
                    SeasonType::Regular,
                )),
                report_id: "weekly-main".to_string(),
                title: "Weekly fantasy prep".to_string(),
                sections: vec![icelines_core::ReportSectionRef {
                    id: "streamers".to_string(),
                    title: "Streamers".to_string(),
                }],
            },
            scoring_scheme: "yahoo-standard".to_string(),
            scoring_categories: vec!["shots".to_string(), "hits".to_string()],
            window: icelines_core::view_model::PoachWindow::Days14,
            source_state: Vec::new(),
            warnings: Vec::new(),
            omissions: Vec::new(),
            sections: vec![icelines_core::view_model::PoachReportSection {
                id: "streamers".to_string(),
                title: "Streamers".to_string(),
                rows: Vec::new(),
            }],
        };

        let rows = poach_report_summary_rows(&report);

        assert_eq!(rows[0].label, "Report");
        assert_eq!(rows[0].value, "Weekly fantasy prep");
        assert_eq!(rows[1].label, "Categories");
        assert_eq!(rows[1].value, "shots, hits");
        assert_eq!(rows[2].label, "Streamers");
    }

    #[test]
    fn l0_dashboard_favorites_pane_projects_viewmodel_rows() {
        let view = FavoritesView::from_members(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "Favorites".to_string(),
            vec![FavoriteMemberInput {
                kind: "player".to_string(),
                key: "Connor McDavid".to_string(),
            }],
            std::collections::HashMap::from([(
                "Connor McDavid".to_string(),
                "2 G, 1 A".to_string(),
            )]),
        );

        let rows = favorite_entity_rows(&view);

        assert_eq!(rows[0].label, "Connor McDavid");
        assert_eq!(rows[0].kind, "2 G, 1 A");
        assert_eq!(
            rows[0].href,
            "/dashboard?workspace=%2Fleaders%3Ffilter%3Dname%253DConnor%2BMcDavid"
        );
    }

    #[test]
    fn l0_dashboard_watchlist_summary_projects_viewmodel_alert_count() {
        let view = WatchlistView::from_members(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "Watchlist".to_string(),
            vec![FavoriteMemberInput {
                kind: "player".to_string(),
                key: "matty beniers".to_string(),
            }],
            std::collections::HashMap::from([(
                "player:matty beniers".to_string(),
                WatchNoteInput {
                    reason: "deployment watch".to_string(),
                    source: "manual".to_string(),
                    updated_at: "2026-05-13T00:00:00Z".to_string(),
                },
            )]),
        );

        let rows = watchlist_summary_rows(&view, 2);

        assert_eq!(rows[0].label, "Watchlist");
        assert_eq!(rows[0].detail, "1 players · 0 teams · 2 alerts");
        assert_eq!(rows[1].detail, "deployment watch");
    }

    #[test]
    fn l0_dashboard_watchlist_pane_projects_notes_from_viewmodel() {
        let view = WatchlistView::from_members(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "Watchlist".to_string(),
            vec![FavoriteMemberInput {
                kind: "player".to_string(),
                key: "Matthew Knies".to_string(),
            }],
            std::collections::HashMap::from([(
                "player:Matthew Knies".to_string(),
                WatchNoteInput {
                    reason: "PP1 promotion watch".to_string(),
                    source: "manual".to_string(),
                    updated_at: "2026-05-13T00:00:00Z".to_string(),
                },
            )]),
        );

        let rows = watchlist_entity_rows(&view);

        assert_eq!(rows[0].label, "Matthew Knies");
        assert_eq!(rows[0].kind, "PP1 promotion watch");
        assert_eq!(
            rows[0].href,
            "/dashboard?workspace=%2Fleaders%3Ffilter%3Dname%253DMatthew%2BKnies"
        );
    }

    #[test]
    fn l0_dashboard_labels_leaders_and_goalies_workspace_summaries() {
        assert_eq!(workspace_label("/leaders"), "Leaders");
        assert_eq!(workspace_label("/goalies"), "Goalies");
        assert_eq!(DASHBOARD_PREVIEW_N, 3);
        assert_eq!(DASHBOARD_GOALIE_GP_REGULAR, 5);
    }
}
