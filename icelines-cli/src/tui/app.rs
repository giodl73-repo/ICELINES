#![allow(dead_code)]
use icelines_core::model::Player;
use crate::tui::event::Action;
use crate::tui::loader::InstallState;

#[derive(Debug, Clone, PartialEq)]
pub enum QueryMode {
    Build,      // normal — editing fields, viewing results
    SaveName,   // typing a name to save the current query
    LoadList,   // browsing saved queries to load
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Team(String),     // team abbreviation
    Player(usize),    // index into loaded players
    Search,
    Tonight,
    Projections,
    Queries,          // interactive query builder
    Groups,
    Fetch,
    Help,
}

pub struct App {
    pub screen:              Screen,
    pub prev_screen:         Option<Screen>,
    pub no_color:            bool,
    pub players:             Vec<Player>,
    pub load_state:          crate::tui::loader::LoadState,
    pub install_state:       InstallState,
    pub tick:                u64,
    pub selected:            usize,
    pub search_query:        String,
    pub status:              String,
    pub show_help:           bool,
    // Headshot ASCII cache
    pub headshot_cache:      crate::tui::headshot::HeadshotCache,
    // Query manager state
    pub query_fields:        Vec<crate::tui::screens::queries::QueryField>,
    pub query_field_idx:     usize,       // which field row is active
    pub query_result_scroll: usize,       // scroll offset in results panel
    pub query_mode:          QueryMode,   // build | save-name | load-list
    pub query_save_name:     String,      // name being typed for save
    pub query_saved_list:    Vec<(String, String)>, // (name, json) loaded from DB
}

impl App {
    pub fn new(no_color: bool) -> Self {
        Self {
            screen:              Screen::Home,
            prev_screen:         None,
            no_color,
            players:             Vec::new(),
            load_state:          crate::tui::loader::LoadState::new(),
            install_state:       InstallState::new(),
            tick:                0,
            selected:            0,
            search_query:        String::new(),
            status:              "Loading data… · Press ? for help · q to quit".to_owned(),
            show_help:           false,
            query_fields:        crate::tui::screens::queries::default_fields(),
            query_field_idx:     0,
            query_result_scroll: 0,
            headshot_cache:      crate::tui::headshot::HeadshotCache::new(),
            query_mode:          QueryMode::Build,
            query_save_name:     String::new(),
            query_saved_list:    Vec::new(),
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
            Action::Back | Action::Escape => {
                // If in save/load mode, cancel back to Build mode first
                if self.screen == Screen::Queries && self.query_mode != QueryMode::Build {
                    self.query_mode = QueryMode::Build;
                    self.status = "Cancelled  ·  s=save  l=load  r=reset".to_owned();
                } else {
                    self.go_back();
                }
            }
            Action::Down => {
                if self.screen == Screen::Queries {
                    // In query screen: ↓ moves between field rows
                    let n = self.query_fields.len();
                    self.query_field_idx = (self.query_field_idx + 1).min(n - 1);
                } else {
                    self.selected = self.selected.saturating_add(1);
                }
            }
            Action::Up => {
                if self.screen == Screen::Queries {
                    self.query_field_idx = self.query_field_idx.saturating_sub(1);
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            Action::Right => {
                if self.screen == Screen::Queries {
                    if let Some(f) = self.query_fields.get_mut(self.query_field_idx) {
                        f.next();
                    }
                    self.query_result_scroll = 0;
                }
            }
            Action::Left => {
                if self.screen == Screen::Queries {
                    if let Some(f) = self.query_fields.get_mut(self.query_field_idx) {
                        f.prev();
                    }
                    self.query_result_scroll = 0;
                }
            }
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
                } else if self.screen == Screen::Queries {
                    match &self.query_mode {
                        QueryMode::SaveName => {
                            // Typing the save name
                            self.query_save_name.push(c);
                        }
                        QueryMode::Build if c == 's' => {
                            // Start save-name mode
                            self.query_mode = QueryMode::SaveName;
                            self.query_save_name.clear();
                            self.status = "Save query as: (type name, Enter to save, Esc to cancel)".to_owned();
                        }
                        QueryMode::Build if c == 'l' => {
                            // Load saved queries list
                            self.query_saved_list = crate::db::GroupDb::open()
                                .ok()
                                .and_then(|db| db.list_saved_queries().ok())
                                .unwrap_or_default();
                            self.query_mode = QueryMode::LoadList;
                            self.selected = 0;
                            self.status = "Saved queries — ↑↓ select · Enter to load · Del to delete · Esc to cancel".to_owned();
                        }
                        _ => {}
                    }
                }
            }
            Action::Backspace => {
                if self.screen == Screen::Search {
                    self.search_query.pop();
                    self.selected = 0;
                } else if self.screen == Screen::Queries && self.query_mode == QueryMode::SaveName {
                    self.query_save_name.pop();
                }
            }
            Action::Tab => self.cycle_screen(),
            Action::Refresh => {
                if self.screen == Screen::Queries {
                    // Reset all query fields to defaults
                    self.query_fields = crate::tui::screens::queries::default_fields();
                    self.query_field_idx = 0;
                    self.query_result_scroll = 0;
                    self.status = "Query fields reset.".to_owned();
                } else {
                    self.status = "Refreshing… (run icelines fetch all)".to_owned();
                }
            }
            Action::Install => {
                if self.screen == Screen::Fetch {
                    use crate::tui::screens::misc::ALL_SEASONS;
                    if let Some(&(season_id, _)) = ALL_SEASONS.get(self.selected) {
                        // Don't re-install if already downloading or done
                        match self.install_state.phase() {
                            crate::tui::loader::InstallPhase::Downloading(_) => {
                                self.status = "Install already in progress…".to_owned();
                            }
                            _ => {
                                self.status = format!("Installing {season_id}…");
                                crate::tui::loader::spawn_install(
                                    season_id.to_string(),
                                    self.install_state.clone(),
                                );
                            }
                        }
                    }
                }
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
                let teams = crate::tui::screens::home::RANKED_TEAMS;
                if let Some(abbrev) = teams.get(self.selected) {
                    self.prev_screen = Some(Screen::Home);
                    self.screen = Screen::Team(abbrev.to_string());
                    self.selected = 0;
                }
            }
            Screen::Team(abbrev) => {
                // Select a player from the team roster → open their player card
                let abbrev = abbrev.clone();
                let global_idx = self.players.iter()
                    .enumerate()
                    .filter(|(_, p)| p.team.as_str() == abbrev.as_str())
                    .nth(self.selected)
                    .map(|(i, _)| i);
                if let Some(idx) = global_idx {
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Player(idx);
                    self.selected = 0;
                }
            }
            Screen::Queries => {
                match self.query_mode {
                    QueryMode::SaveName => {
                        // Save the current query with the typed name
                        let name = self.query_save_name.trim().to_owned();
                        if !name.is_empty() {
                            let json = crate::tui::screens::queries::fields_to_json(&self.query_fields);
                            if let Ok(db) = crate::db::GroupDb::open() {
                                let _ = db.save_query(&name, &json);
                                self.status = format!("Saved query '{name}'  ·  l=load  s=save  r=reset");
                            }
                        }
                        self.query_mode = QueryMode::Build;
                    }
                    QueryMode::LoadList => {
                        // Load the selected saved query
                        if let Some((name, json)) = self.query_saved_list.get(self.selected) {
                            crate::tui::screens::queries::apply_saved_json(&mut self.query_fields, json);
                            self.status = format!("Loaded query '{name}'  ·  ←→ to adjust  s=save  r=reset");
                            self.query_mode = QueryMode::Build;
                            self.query_result_scroll = 0;
                        }
                    }
                    QueryMode::Build => {
                        // Enter on a result row → player card
                        let results = crate::tui::screens::queries::run_query(&self.players, &self.query_fields);
                        let row_idx = self.query_result_scroll + self.selected.min(results.len().saturating_sub(1));
                        if let Some((_, p)) = results.get(row_idx) {
                            if let Some(global_idx) = self.players.iter().position(|pl| pl.nhl_id == p.nhl_id) {
                                self.prev_screen = Some(self.screen.clone());
                                self.screen = Screen::Player(global_idx);
                                self.selected = 0;
                            }
                        }
                    }
                }
            }
            Screen::Projections => {
                // Enter on a projection row → player card
                // The sorted order matches render order — find the Nth rankable player
                let mut sorted_indices: Vec<usize> = self.players.iter()
                    .enumerate()
                    .filter(|(_, p)| p.pace_score.is_some())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .enumerate()
                    .map(|(rank_pos, (global_idx, p))| (rank_pos, global_idx, p.pace_score.map(|s| s.pace_82).unwrap_or(0.0)))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(_, gi, _)| gi)
                    .collect();
                // Sort by pace descending (same order as render)
                sorted_indices.sort_by(|&a, &b| {
                    let pa = self.players[a].pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                    let pb = self.players[b].pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                    pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
                });
                if let Some(&global_idx) = sorted_indices.get(self.selected) {
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Player(global_idx);
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
            Screen::Home        => Screen::Queries,
            Screen::Queries     => Screen::Projections,
            Screen::Projections => Screen::Tonight,
            Screen::Tonight     => Screen::Groups,
            Screen::Groups      => Screen::Fetch,
            Screen::Fetch       => Screen::Home,
            _                   => Screen::Home,
        };
        self.selected = 0;
        self.query_result_scroll = 0;
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
        assert_eq!(app.screen, Screen::Queries, "Home→Queries");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Projections, "Queries→Projections");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Tonight, "Projections→Tonight");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Groups, "Tonight→Groups");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Fetch, "Groups→Fetch");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Home, "Fetch→Home (wraps)");
    }

    #[test]
    fn l0_tui_search_key_switches_to_search_screen() {
        let mut app = App::new(false);
        app.handle(Action::Search);
        assert_eq!(app.screen, Screen::Search);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn l0_tui_esc_returns_to_home_when_no_history() {
        let mut app = App::new(false);
        app.handle(Action::Tab); // go to Tonight
        app.handle(Action::Back); // Esc — prev_screen is None, go to Home
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn l0_tui_back_restores_prev_screen() {
        let mut app = App::new(false);
        // Simulate navigating to Search then back
        app.handle(Action::Search);
        assert_eq!(app.screen, Screen::Search);
        app.handle(Action::Back);
        assert_eq!(app.screen, Screen::Home, "Back from Search returns to Home");
    }

    #[test]
    fn l0_tui_home_enter_navigates_to_team() {
        let mut app = App::new(false);
        // selected=0 → first team in RANKED_TEAMS
        app.handle(Action::Enter);
        let first_team = crate::tui::screens::home::RANKED_TEAMS[0];
        assert_eq!(app.screen, Screen::Team(first_team.to_string()));
        assert_eq!(app.prev_screen, Some(Screen::Home));
    }

    #[test]
    fn l0_tui_team_enter_without_players_does_not_crash() {
        let mut app = App::new(false);
        // Navigate to a team with no players loaded
        app.screen = Screen::Team("SEA".to_string());
        app.selected = 0;
        app.handle(Action::Enter); // should not panic even with empty player list
        // With no players, selected player not found — stays on Team screen
        assert!(matches!(app.screen, Screen::Team(_) | Screen::Player(_)));
    }

    #[test]
    fn l0_tui_down_up_selection() {
        let mut app = App::new(false);
        assert_eq!(app.selected, 0);
        app.handle(Action::Down);
        assert_eq!(app.selected, 1);
        app.handle(Action::Down);
        assert_eq!(app.selected, 2);
        app.handle(Action::Up);
        assert_eq!(app.selected, 1);
        app.handle(Action::Up);
        app.handle(Action::Up); // can't go below 0
        assert_eq!(app.selected, 0);
    }
}
