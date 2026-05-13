use crate::state::WebState;
use crate::templates::{DashboardEntityRow, DashboardLinkRow, DashboardTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct DashboardQuery {
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

    let tmpl = DashboardTemplate {
        active_label,
        workspace_url: workspace_url.clone(),
        workspace_label,
        scores_summary: "Live, final, and scheduled games stay one click away.".to_owned(),
        favorites: dashboard_entities("Favorites"),
        watchlist: dashboard_entities("Watchlist"),
        schedule_links: schedule_links(),
        workspace_links: workspace_links(&workspace_url),
    };

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
        .filter(|path| {
            path.starts_with('/')
                && !path.starts_with("//")
                && !path.starts_with("/api/")
                && !path.starts_with("/static/")
        })
        .unwrap_or("/leaders")
        .to_owned()
}

fn workspace_label(path: &str) -> String {
    match path.split('?').next().unwrap_or(path) {
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
        href: href.to_owned(),
        detail: if active == href {
            format!("{detail} - active")
        } else {
            detail.to_owned()
        },
    })
    .collect::<Vec<_>>();
    rows.push(DashboardLinkRow {
        label: "Docs".to_owned(),
        href: "/docs".to_owned(),
        detail: "command reference".to_owned(),
    });
    rows
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
}
