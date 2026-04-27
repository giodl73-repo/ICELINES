#![allow(dead_code)]
use icelines_core::model::Player;
use crate::tui::event::Action;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Team(String),     // team abbreviation
    Player(usize),    // index into loaded players
    Search,
    Tonight,
    Projections,
    Groups,
    Fetch,
    Help,
}

pub struct App {
    pub screen:       Screen,
    pub prev_screen:  Option<Screen>,
    pub no_color:     bool,
    pub players:      Vec<Player>,
    pub load_state:   crate::tui::loader::LoadState,
    pub selected:     usize,
    pub search_query: String,
    pub status:       String,
    pub show_help:    bool,
}

impl App {
    pub fn new(no_color: bool) -> Self {
        Self {
            screen:       Screen::Home,
            prev_screen:  None,
            no_color,
            players:      Vec::new(),
            load_state:   crate::tui::loader::LoadState::new(),
            selected:     0,
            search_query: String::new(),
            status:       "Loading data… · Press ? for help · q to quit".to_owned(),
            show_help:    false,
        }
    }

    /// Handle an action. Returns true if the app should quit.
    pub fn handle(&mut self, action: Action) -> bool {
        if self.show_help {
            // Any key dismisses help
            self.show_help = false;
            return false;
        }

        match action {
            Action::Quit => return true,
            Action::Help => self.show_help = true,
            Action::Back | Action::Escape => self.go_back(),
            Action::Down  => self.selected = self.selected.saturating_add(1),
            Action::Up    => self.selected = self.selected.saturating_sub(1),
            Action::Enter => self.activate_selected(),
            Action::Search => {
                self.prev_screen = Some(self.screen.clone());
                self.screen = Screen::Search;
                self.search_query.clear();
                self.selected = 0;
            }
            Action::Char(c) => {
                if self.screen == Screen::Search {
                    self.search_query.push(c);
                    self.selected = 0;
                }
            }
            Action::Backspace => {
                if self.screen == Screen::Search {
                    self.search_query.pop();
                    self.selected = 0;
                }
            }
            Action::Tab => self.cycle_screen(),
            Action::Refresh => {
                self.status = "Refreshing… (run icelines fetch all)".to_owned();
            }
        }
        false
    }

    fn go_back(&mut self) {
        if let Some(prev) = self.prev_screen.take() {
            self.screen = prev;
        } else {
            self.screen = Screen::Home;
        }
        self.selected = 0;
    }

    fn activate_selected(&mut self) {
        match &self.screen {
            Screen::Home => {
                // Navigate to team screen for the selected team
                let teams = crate::tui::screens::home::RANKED_TEAMS;
                if let Some(abbrev) = teams.get(self.selected) {
                    self.prev_screen = Some(Screen::Home);
                    self.screen = Screen::Team(abbrev.to_string());
                    self.selected = 0;
                }
            }
            Screen::Search => {
                // Navigate to player screen for selected search result
                self.prev_screen = Some(Screen::Search);
                self.screen = Screen::Player(self.selected);
                self.selected = 0;
            }
            _ => {}
        }
    }

    fn cycle_screen(&mut self) {
        self.screen = match &self.screen {
            Screen::Home        => Screen::Tonight,
            Screen::Tonight     => Screen::Projections,
            Screen::Projections => Screen::Groups,
            Screen::Groups      => Screen::Fetch,
            Screen::Fetch       => Screen::Home,
            _                   => Screen::Home,
        };
        self.selected = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_tui_app_initial_screen_is_home() {
        let app = App::new(false);
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn l0_tui_quit_action_returns_true() {
        let mut app = App::new(false);
        assert!(app.handle(Action::Quit));
    }

    #[test]
    fn l0_tui_help_toggle() {
        let mut app = App::new(false);
        app.handle(Action::Help);
        assert!(app.show_help);
        app.handle(Action::Char('x')); // any key dismisses
        assert!(!app.show_help);
    }

    #[test]
    fn l0_tui_tab_cycles_screens() {
        let mut app = App::new(false);
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Tonight);
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Projections);
    }
}
