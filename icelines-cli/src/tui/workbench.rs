use icelines_core::{
    WorkbenchExperience, WorkbenchId, WorkbenchPaneBinding, WorkbenchSurface, WorkbenchZone,
    WORKBENCH_CATALOG, WORKBENCH_EXPERIENCES, WORKBENCH_PANE_BINDINGS,
};

use crate::tui::app::Screen;

pub fn screen_for_workbench(id: WorkbenchId) -> Option<Screen> {
    match id {
        WorkbenchId::League => Some(Screen::Home),
        WorkbenchId::Stats => Some(Screen::Queries),
        WorkbenchId::Goalies => Some(Screen::Goalies),
        WorkbenchId::Depth => Some(Screen::Depth),
        WorkbenchId::Scores => Some(Screen::Tonight),
        WorkbenchId::Schedule => Some(Screen::Schedule),
        WorkbenchId::Transactions => Some(Screen::Transactions),
        WorkbenchId::Playoffs => Some(Screen::Playoffs),
        WorkbenchId::Favorites => Some(Screen::Favorites),
        WorkbenchId::Groups => Some(Screen::Groups),
        WorkbenchId::Fantasy => Some(Screen::FantasyGaps),
        WorkbenchId::Simulate => Some(Screen::FantasySim),
        WorkbenchId::Poach => Some(Screen::Poach),
        WorkbenchId::Docs => Some(Screen::Help),
        WorkbenchId::Admin => Some(Screen::Fetch),
        WorkbenchId::Team
        | WorkbenchId::Player
        | WorkbenchId::Game
        | WorkbenchId::Watchlist
        | WorkbenchId::Reports
        | WorkbenchId::Records
        | WorkbenchId::Career => None,
    }
}

#[allow(dead_code)]
pub fn no_arg_workbench_screens() -> impl Iterator<Item = (WorkbenchId, Screen)> {
    WORKBENCH_CATALOG
        .iter()
        .filter_map(|entry| screen_for_workbench(entry.id).map(|screen| (entry.id, screen)))
}

pub fn workbench_for_screen(screen: &Screen) -> Option<WorkbenchId> {
    match screen {
        Screen::Home => Some(WorkbenchId::League),
        Screen::Queries | Screen::Projections | Screen::Search => Some(WorkbenchId::Stats),
        Screen::Goalies | Screen::GoalieDetailById(_) => Some(WorkbenchId::Goalies),
        Screen::Depth | Screen::DepthTeam(_) => Some(WorkbenchId::Depth),
        Screen::Team(_) => Some(WorkbenchId::Team),
        Screen::PlayerById(_) => Some(WorkbenchId::Player),
        Screen::Tonight => Some(WorkbenchId::Scores),
        Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _) => {
            Some(WorkbenchId::Schedule)
        }
        Screen::Transactions => Some(WorkbenchId::Transactions),
        Screen::Playoffs | Screen::SeriesDetail(_) => Some(WorkbenchId::Playoffs),
        Screen::GameDetail(_) => Some(WorkbenchId::Game),
        Screen::Favorites => Some(WorkbenchId::Favorites),
        Screen::Groups | Screen::GroupDetail(_) => Some(WorkbenchId::Groups),
        Screen::FantasyGaps => Some(WorkbenchId::Fantasy),
        Screen::FantasySim => Some(WorkbenchId::Simulate),
        Screen::Poach => Some(WorkbenchId::Poach),
        Screen::Help => Some(WorkbenchId::Docs),
        Screen::Fetch => Some(WorkbenchId::Admin),
        Screen::PlayerRecordsById(_) => Some(WorkbenchId::Records),
        Screen::PlayerAwardsById(_) | Screen::PlayerStreaksById(_) | Screen::CompsById(_) => {
            Some(WorkbenchId::Player)
        }
    }
}

#[allow(dead_code)] // Pulse 02 adapter seam; Pulse 03 wires visible TUI pane controls.
pub fn tui_pane_bindings_for_zone(
    zone: WorkbenchZone,
) -> impl Iterator<Item = &'static WorkbenchPaneBinding> {
    WORKBENCH_PANE_BINDINGS.iter().filter(move |binding| {
        binding.zone == zone && binding.supported_surfaces.contains(&WorkbenchSurface::Tui)
    })
}

#[allow(dead_code)] // Pulse 02 adapter seam; Pulse 03 applies bound experiences.
pub fn tui_bound_experiences() -> impl Iterator<Item = &'static WorkbenchExperience> {
    WORKBENCH_EXPERIENCES.iter().filter(|experience| {
        experience
            .supported_surfaces
            .contains(&WorkbenchSurface::Tui)
            && screen_for_workbench(experience.center).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{WorkbenchExperienceId, WorkbenchPaneBindingId};

    #[test]
    fn l0_tui_workbench_adapter_covers_main_no_arg_screens() {
        let mapped: Vec<_> = no_arg_workbench_screens()
            .map(|(_id, screen)| screen)
            .collect();

        for screen in [
            Screen::Home,
            Screen::Queries,
            Screen::Goalies,
            Screen::Depth,
            Screen::Tonight,
            Screen::Schedule,
            Screen::Transactions,
            Screen::Playoffs,
            Screen::Favorites,
            Screen::Groups,
            Screen::Poach,
            Screen::FantasyGaps,
            Screen::FantasySim,
            Screen::Help,
            Screen::Fetch,
        ] {
            assert!(mapped.contains(&screen), "missing TUI screen {screen:?}");
        }
    }

    #[test]
    fn l0_tui_workbench_adapter_defers_argument_targets() {
        for id in [
            WorkbenchId::Team,
            WorkbenchId::Player,
            WorkbenchId::Game,
            WorkbenchId::Records,
            WorkbenchId::Career,
        ] {
            assert!(
                screen_for_workbench(id).is_none(),
                "{id:?} requires an argument and must not lower to a fake screen"
            );
        }
    }

    #[test]
    fn l0_tui_workbench_adapter_maps_active_screen_to_catalog_id() {
        assert_eq!(
            workbench_for_screen(&Screen::Home),
            Some(WorkbenchId::League)
        );
        assert_eq!(
            workbench_for_screen(&Screen::Queries),
            Some(WorkbenchId::Stats)
        );
        assert_eq!(
            workbench_for_screen(&Screen::Tonight),
            Some(WorkbenchId::Scores)
        );
        assert_eq!(
            workbench_for_screen(&Screen::FantasySim),
            Some(WorkbenchId::Simulate)
        );
    }

    #[test]
    fn l0_tui_workbench_adapter_exposes_tui_safe_pane_bindings() {
        let left: Vec<_> = tui_pane_bindings_for_zone(WorkbenchZone::LeftPane)
            .map(|binding| binding.id)
            .collect();
        let right: Vec<_> = tui_pane_bindings_for_zone(WorkbenchZone::RightPane)
            .map(|binding| binding.id)
            .collect();

        assert_eq!(
            left,
            vec![
                WorkbenchPaneBindingId::FavoritesLeft,
                WorkbenchPaneBindingId::SavedQueriesLeft
            ]
        );
        assert_eq!(
            right,
            vec![
                WorkbenchPaneBindingId::ScheduleRight,
                WorkbenchPaneBindingId::DataSourceRight,
                WorkbenchPaneBindingId::DocsHelpRight
            ]
        );
    }

    #[test]
    fn l0_tui_workbench_adapter_exposes_supported_experiences_with_screens() {
        let experiences: Vec<_> = tui_bound_experiences()
            .map(|experience| experience.id)
            .collect();

        assert_eq!(experiences, vec![WorkbenchExperienceId::TonightBench]);
    }
}
