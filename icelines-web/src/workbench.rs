use icelines_core::{
    WorkbenchExperience, WorkbenchId, WorkbenchPaneBinding, WorkbenchSurface, WorkbenchZone,
    WORKBENCH_CATALOG, WORKBENCH_EXPERIENCES, WORKBENCH_PANE_BINDINGS,
};

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

pub fn web_pane_bindings_for_zone(
    zone: WorkbenchZone,
) -> impl Iterator<Item = &'static WorkbenchPaneBinding> {
    WORKBENCH_PANE_BINDINGS.iter().filter(move |binding| {
        binding.zone == zone && binding.supported_surfaces.contains(&WorkbenchSurface::Web)
    })
}

pub fn web_bound_experiences() -> impl Iterator<Item = &'static WorkbenchExperience> {
    WORKBENCH_EXPERIENCES.iter().filter(|experience| {
        experience
            .supported_surfaces
            .contains(&WorkbenchSurface::Web)
            && route_for_workbench(experience.center).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{WorkbenchExperienceId, WorkbenchPaneBindingId};
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

    #[test]
    fn l0_web_workbench_adapter_exposes_pane_bindings_for_both_side_zones() {
        let left: HashSet<_> = web_pane_bindings_for_zone(WorkbenchZone::LeftPane)
            .map(|binding| binding.id)
            .collect();
        let right: HashSet<_> = web_pane_bindings_for_zone(WorkbenchZone::RightPane)
            .map(|binding| binding.id)
            .collect();

        assert!(left.contains(&WorkbenchPaneBindingId::FavoritesLeft));
        assert!(left.contains(&WorkbenchPaneBindingId::WatchlistLeft));
        assert!(right.contains(&WorkbenchPaneBindingId::ScheduleRight));
        assert!(right.contains(&WorkbenchPaneBindingId::DataSourceRight));
    }

    #[test]
    fn l0_web_workbench_adapter_experience_centers_are_dashboard_ready() {
        let experiences: HashSet<_> = web_bound_experiences()
            .map(|experience| experience.id)
            .collect();

        for id in [
            WorkbenchExperienceId::TonightBench,
            WorkbenchExperienceId::ScoringRoom,
            WorkbenchExperienceId::TeamRoom,
            WorkbenchExperienceId::FantasyRoom,
            WorkbenchExperienceId::AdminRoom,
        ] {
            assert!(experiences.contains(&id), "missing web experience {id:?}");
        }

        for experience in web_bound_experiences() {
            let route = route_for_workbench(experience.center)
                .expect("web experience center must have dashboard route");
            assert!(!route.contains("/command"));
        }
    }
}
