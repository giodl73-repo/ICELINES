use icelines_core::{WorkbenchId, WORKBENCH_CATALOG};

use crate::tui::app::Screen;

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
