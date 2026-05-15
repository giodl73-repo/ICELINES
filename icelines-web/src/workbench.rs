use icelines_core::{WorkbenchId, WORKBENCH_CATALOG};

pub fn route_for_workbench(id: WorkbenchId) -> Option<&'static str> {
    match id {
        WorkbenchId::League | WorkbenchId::Stats => Some("/leaders"),
        WorkbenchId::Goalies => Some("/goalies"),
        WorkbenchId::Depth => Some("/depth"),
        WorkbenchId::Scores => Some("/scores"),
        WorkbenchId::Schedule => Some("/schedule"),
        WorkbenchId::Transactions => Some("/transactions"),
        WorkbenchId::Playoffs => Some("/playoffs"),
        WorkbenchId::Favorites => Some("/favorites"),
        WorkbenchId::Watchlist => Some("/watchlist"),
        WorkbenchId::Fantasy => Some("/fantasy"),
        WorkbenchId::Simulate => Some("/fantasy?weeks=4"),
        WorkbenchId::Poach => Some("/poach"),
        WorkbenchId::Reports => Some("/reports/weekly"),
        WorkbenchId::Career => Some("/career"),
        WorkbenchId::Docs => Some("/docs"),
        WorkbenchId::Admin => Some("/admin"),
        WorkbenchId::Team | WorkbenchId::Player | WorkbenchId::Game | WorkbenchId::Records => None,
        WorkbenchId::Groups => None,
    }
}

pub fn dashboard_ready_workbenches() -> impl Iterator<Item = (WorkbenchId, &'static str)> {
    WORKBENCH_CATALOG
        .iter()
        .filter_map(|entry| route_for_workbench(entry.id).map(|route| (entry.id, route)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn l0_web_workbench_adapter_covers_dashboard_ready_routes() {
        let routes: HashSet<_> = dashboard_ready_workbenches()
            .map(|(_id, route)| route)
            .collect();

        for route in [
            "/leaders",
            "/goalies",
            "/depth",
            "/scores",
            "/schedule",
            "/transactions",
            "/playoffs",
            "/favorites",
            "/watchlist",
            "/fantasy",
            "/fantasy?weeks=4",
            "/poach",
            "/reports/weekly",
            "/career",
            "/docs",
            "/admin",
        ] {
            assert!(routes.contains(route), "missing dashboard route {route}");
        }
    }

    #[test]
    fn l0_web_workbench_adapter_defers_parameterized_routes() {
        for id in [
            WorkbenchId::Team,
            WorkbenchId::Player,
            WorkbenchId::Game,
            WorkbenchId::Records,
        ] {
            assert!(
                route_for_workbench(id).is_none(),
                "{id:?} requires route params and must not lower to a fake URL"
            );
        }
    }

    #[test]
    fn l0_web_workbench_routes_are_internal_and_get_readonly() {
        for (_id, route) in dashboard_ready_workbenches() {
            assert!(route.starts_with('/'), "{route} is not an internal route");
            assert!(!route.starts_with("//"), "{route} is scheme-relative");
            assert!(
                !route.contains("/command"),
                "{route} must not target mutation command endpoints"
            );
        }
    }
}
