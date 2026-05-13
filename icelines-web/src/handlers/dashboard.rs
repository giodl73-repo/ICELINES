use crate::state::WebState;
use crate::templates::{
    DashboardEntityRow, DashboardLinkRow, DashboardTemplate, DashboardWorkspaceTemplate,
};
use askama::Template;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

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

    if matches!(q.partial.as_deref(), Some("workspace")) {
        return render_template(DashboardWorkspaceTemplate {
            workspace_url,
            workspace_label,
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
        other if other.starts_with("/team/") => "Team Depth",
        other if other.starts_with("/game/") => "Game Detail",
        _ => "Workspace",
    }
    .to_owned()
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
}
