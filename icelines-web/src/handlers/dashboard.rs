use crate::state::WebState;
use crate::templates::{
    DashboardEntityRow, DashboardLinkRow, DashboardSummaryRow, DashboardTemplate,
    DashboardWorkspaceTemplate,
};
use askama::Template;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    HomeView, ScheduleRecord, TeamAbbr, TeamDepthView, TeamSeasonView, ViewContext, ViewWindow,
};
use serde::Deserialize;

const DASHBOARD_PREVIEW_N: usize = 3;
const DASHBOARD_GOALIE_GP_REGULAR: u32 = 5;
const DASHBOARD_GOALIE_GP_PLAYOFF: u32 = 1;

#[derive(Debug, Deserialize, Default)]
pub struct DashboardQuery {
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub partial: Option<String>,
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
    let workspace_url = normalize_workspace(q.workspace.as_deref());
    let workspace_label = workspace_label(&workspace_url);
    let workspace_links = workspace_links(&workspace_url);
    let workspace_summary = workspace_summary(&state, &workspace_url).await;

    if matches!(q.partial.as_deref(), Some("workspace")) {
        return render_template(DashboardWorkspaceTemplate {
            workspace_url,
            workspace_label,
            workspace_summary,
            workspace_links,
        });
    }

    if q.partial.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Html("unknown dashboard partial".to_owned()),
        )
            .into_response();
    }

    let tmpl = DashboardTemplate {
        active_label,
        workspace_url: workspace_url.clone(),
        workspace_label,
        workspace_summary,
        scores_summary: "Live, final, and scheduled games stay one click away.".to_owned(),
        favorites: dashboard_entities("Favorites"),
        watchlist: dashboard_entities("Watchlist"),
        schedule_links: schedule_links(),
        workspace_links,
    };

    render_template(tmpl)
}

pub async fn post_dashboard_command(
    headers: HeaderMap,
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
            crate::dashboard_command::DashboardMutationIntent::WatchPlayer { player, .. },
        )) => {
            super::poach::post_watch_rule_create_form(Form(super::poach::WatchRuleCreateForm {
                player,
                trigger: "available".to_owned(),
            }))
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

fn normalize_workspace(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|path| is_workspace_route(path))
        .unwrap_or("/leaders")
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
            | "/docs"
    ) || route.starts_with("/player/")
        || route.starts_with("/team/")
        || route.starts_with("/game/")
}

fn workspace_route_key(path: &str) -> &str {
    let route = path.split('?').next().unwrap_or(path);
    if route.len() > 1 {
        route.trim_end_matches('/')
    } else {
        route
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

fn dashboard_entities(group: &str) -> Vec<DashboardEntityRow> {
    super::favorites_data::read_group_members(group)
        .into_iter()
        .take(8)
        .map(|(kind, key)| {
            let href = match kind.as_str() {
                "team" => format!("/team/{key}"),
                _ => format!("/leaders?filter=name%3D{}", url_component(&key)),
            };
            DashboardEntityRow {
                label: key,
                href,
                kind,
            }
        })
        .collect()
}

fn schedule_links() -> Vec<DashboardLinkRow> {
    vec![
        DashboardLinkRow {
            label: "Scores".to_owned(),
            href: "/scores".to_owned(),
            detail: "today's games".to_owned(),
        },
        DashboardLinkRow {
            label: "Schedule".to_owned(),
            href: "/schedule".to_owned(),
            detail: "date and team views".to_owned(),
        },
        DashboardLinkRow {
            label: "Playoffs".to_owned(),
            href: "/playoffs".to_owned(),
            detail: "bracket context".to_owned(),
        },
    ]
}

fn workspace_links(active: &str) -> Vec<DashboardLinkRow> {
    let mut rows = vec![
        ("Leaders", "/leaders", "skater leaderboard and filters"),
        ("Goalies", "/goalies", "goalie leaderboard"),
        ("Depth", "/depth", "cross-team depth rankings"),
        ("Poach", "/poach", "fantasy free-agent board"),
        ("Fantasy", "/fantasy", "roster gaps and simulations"),
        ("Transactions", "/transactions", "league movement feed"),
    ]
    .into_iter()
    .map(|(label, href, detail)| DashboardLinkRow {
        label: label.to_owned(),
        href: dashboard_workspace_href(href),
        detail: if workspace_route_key(active) == href {
            format!("{detail} - active")
        } else {
            detail.to_owned()
        },
    })
    .collect::<Vec<_>>();
    rows.push(DashboardLinkRow {
        label: "Docs".to_owned(),
        href: dashboard_workspace_href("/docs"),
        detail: "command reference".to_owned(),
    });
    rows
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
        assert_eq!(normalize_workspace(Some("/admin")), "/leaders");
        assert_eq!(normalize_workspace(Some("/dashboard")), "/leaders");
        assert_eq!(normalize_workspace(Some("/favorites/add")), "/leaders");
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
        assert_eq!(
            workspace_label("/poach?availability=imported-available"),
            "Poach"
        );
    }

    #[test]
    fn l0_dashboard_workspace_links_preserve_dashboard_state() {
        let links = workspace_links("/poach?availability=imported-available");
        let poach = links
            .iter()
            .find(|row| row.label == "Poach")
            .expect("poach workspace link");
        assert_eq!(poach.href, "/dashboard?workspace=%2Fpoach");
        assert_eq!(poach.detail, "fantasy free-agent board - active");

        assert_eq!(
            dashboard_workspace_href("/poach?availability=imported-available"),
            "/dashboard?workspace=%2Fpoach%3Favailability%3Dimported-available"
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
    fn l0_dashboard_labels_leaders_and_goalies_workspace_summaries() {
        assert_eq!(workspace_label("/leaders"), "Leaders");
        assert_eq!(workspace_label("/goalies"), "Goalies");
        assert_eq!(DASHBOARD_PREVIEW_N, 3);
        assert_eq!(DASHBOARD_GOALIE_GP_REGULAR, 5);
    }
}
