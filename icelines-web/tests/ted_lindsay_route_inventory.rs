//! Ted Lindsay route inventory gate.
//!
//! Every mounted web route must be represented in the surface parity matrix.
//! This keeps `icelines_web::router` and `design/specs/surface-parity.md` from
//! drifting as HTML/API routes are added.

const ROUTER_ROUTES: &[&str] = &[
    "GET /",
    "GET /dashboard",
    "POST /dashboard/command",
    "GET /static/:asset",
    "GET /leaders",
    "GET /api/v1/leaders",
    "GET /player/:id",
    "GET /api/v1/player/:id",
    "GET /player/:id/scoring",
    "GET /api/v1/player/:id/scoring",
    "GET /scouting/:id",
    "GET /api/v1/scouting/:id",
    "GET /compare",
    "GET /api/v1/compare",
    "GET /goalies",
    "GET /api/v1/goalies",
    "GET /team/:abbrev",
    "GET /team/:abbrev/season",
    "GET /team/:abbrev/streaks",
    "GET /team/:abbrev/scoring",
    "GET /records/player/:id",
    "GET /records/team/:abbrev",
    "GET /api/v1/team/:abbrev",
    "GET /api/v1/team/:abbrev/season",
    "GET /api/v1/team/:abbrev/streaks",
    "GET /api/v1/team/:abbrev/scoring",
    "GET /api/v1/records/player/:id",
    "GET /api/v1/records/team/:abbrev",
    "GET /depth",
    "GET /api/v1/depth",
    "GET /poach",
    "GET /reports/poach",
    "GET /reports/weekly",
    "GET /reports/analytics-cache",
    "GET /api/v1/poach",
    "GET /api/v1/reports/analytics-cache",
    "GET /coach/dashboard",
    "GET /api/v1/coach/dashboard",
    "GET /player/evidence-card",
    "GET /api/v1/player/evidence-card",
    "GET /lines/explorer",
    "GET /api/v1/lines/explorer",
    "GET /goalies/readiness",
    "GET /api/v1/goalies/readiness",
    "GET /practice/focus",
    "GET /api/v1/practice/focus",
    "GET /postgame/review",
    "GET /api/v1/postgame/review",
    "GET /postgame/adjustments",
    "GET /api/v1/postgame/adjustments",
    "GET /scout/opponent",
    "GET /api/v1/scout/opponent",
    "GET /api/v1/watch-rules",
    "POST /api/v1/watch-rules/set-enabled",
    "POST /watch-rules/set-enabled",
    "POST /watch-rules/create",
    "POST /watch-rules/delete",
    "GET /career",
    "GET /api/v1/career",
    "GET /docs",
    "POST /season-type/:kind",
    "GET /scores",
    "GET /api/v1/scores",
    "GET /schedule",
    "GET /api/v1/schedule",
    "GET /playoffs",
    "GET /api/v1/playoffs",
    "GET /favorites",
    "GET /api/v1/favorites",
    "GET /tonight/intel",
    "GET /api/v1/tonight/intel",
    "GET /watchlist",
    "GET /api/v1/watchlist",
    "GET /game/:id",
    "GET /api/v1/game/:id",
    "GET /game/:id/scoring",
    "GET /api/v1/game/:id/scoring",
    "POST /favorites/add",
    "POST /favorites/remove",
    "POST /api/v1/favorites/add",
    "POST /api/v1/favorites/remove",
    "GET /transactions",
    "GET /api/v1/transactions",
    "GET /fantasy",
    "GET /api/v1/fantasy/gaps",
    "GET /api/v1/fantasy/simulate",
    "GET /api/v1/fantasy/daily",
    "GET /api/v1/fantasy/matchup",
    "GET /api/v1/fantasy/roster-shape",
    "GET /admin",
    "GET /api/v1/admin/data-status",
    "GET /api/v1/admin/snapshots",
    "GET /api/v1/admin/config",
    "POST /api/v1/admin/game-cache/load",
    "POST /admin/game-cache/load",
    "POST /api/v1/admin/game-cache/load-favorites",
    "POST /admin/game-cache/load-favorites",
];

const DASHBOARD_PANEL_READY_WORKSPACES: &[&str] = &[
    "/",
    "/leaders",
    "/goalies",
    "/depth",
    "/team/:abbrev",
    "/team/:abbrev/season",
    "/team/:abbrev/streaks",
    "/records/player/:id",
    "/records/team/:abbrev",
    "/player/:id",
    "/player/:id/scoring",
    "/scores",
    "/schedule",
    "/game/:id",
    "/poach",
    "/fantasy",
    "/transactions",
    "/playoffs",
    "/favorites",
    "/tonight/intel",
    "/watchlist",
    "/career",
    "/reports/poach",
    "/reports/weekly",
    "/docs",
];

#[test]
fn every_router_route_is_in_surface_parity_matrix() {
    let matrix = include_str!("../../design/specs/surface-parity.md");
    let missing: Vec<&str> = ROUTER_ROUTES
        .iter()
        .copied()
        .filter(|route| !matrix.contains(&format!("`{route}`")))
        .collect();

    assert!(
        missing.is_empty(),
        "surface-parity.md is missing mounted route(s): {missing:?}"
    );
}

#[test]
fn dashboard_panel_ready_workspaces_are_documented() {
    let matrix = include_str!("../../design/specs/surface-parity.md");
    let missing: Vec<&str> = DASHBOARD_PANEL_READY_WORKSPACES
        .iter()
        .copied()
        .filter(|workspace| !matrix.contains(&format!("| `{workspace}` |")))
        .collect();

    assert!(
        missing.is_empty(),
        "surface-parity.md is missing dashboard panel-ready workspace(s): {missing:?}"
    );
}

#[test]
fn route_inventory_has_no_duplicate_entries() {
    let mut sorted = ROUTER_ROUTES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    assert_eq!(
        sorted.len(),
        ROUTER_ROUTES.len(),
        "Ted Lindsay route inventory contains duplicate route entries"
    );
}
