#![allow(dead_code)]
use icelines_core::model::Player;
use crate::tui::event::Action;
use crate::tui::loader::InstallState;

/// Auto-refresh interval for the live Scores tab. Spec: `scores.md` §Auto-Refresh.
pub(crate) const SCORES_AUTO_REFRESH_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(30);

/// Pure decision: should the live-Scores auto-refresh fire on this tick?
///
/// Rules (from `scores.md`):
/// - Only on the Scores tab (`Screen::Tonight`)
/// - Only on the live date (`scores_date.is_empty()`)
/// - Only when at least `interval` has elapsed since the last refresh
/// - Never when `last` is `None` — `maybe_fetch_scores` handles the initial
///   fetch on tab entry; the polling timer waits one interval after that.
pub(crate) fn should_auto_refresh(
    screen:      &Screen,
    scores_date: &str,
    last:        Option<std::time::Instant>,
    now:         std::time::Instant,
    interval:    std::time::Duration,
) -> bool {
    if !matches!(screen, Screen::Tonight) {
        return false;
    }
    if !scores_date.is_empty() {
        return false;
    }
    match last {
        Some(t) => now.duration_since(t) >= interval,
        None    => false,
    }
}

/// Parse the Scores date-picker input. Accepts:
/// - `YYYY-MM-DD`  (full ISO date)
/// - `YYYY/MM/DD`  (slash variant)
/// - `MM/DD`       (current calendar year inferred)
/// - `MM-DD`       (current calendar year inferred)
///
/// Returns the canonical `YYYY-MM-DD` string on success, or a user-facing
/// error message describing what went wrong.
pub(crate) fn parse_picker_date(raw: &str) -> Result<String, String> {
    use chrono::{Datelike, NaiveDate, Utc};
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Date is empty".to_owned());
    }
    // Try full ISO first
    let isoish = trimmed.replace('/', "-");
    if let Ok(d) = NaiveDate::parse_from_str(&isoish, "%Y-%m-%d") {
        return Ok(d.format("%Y-%m-%d").to_string());
    }
    // MM-DD with year inferred from today
    let today = Utc::now().date_naive();
    let candidate = format!("{}-{}", today.year(), isoish);
    if let Ok(d) = NaiveDate::parse_from_str(&candidate, "%Y-%m-%d") {
        return Ok(d.format("%Y-%m-%d").to_string());
    }
    Err(format!(
        "Could not parse '{trimmed}'. Try YYYY-MM-DD or MM/DD."
    ))
}

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
    Queries,              // interactive query builder
    Groups,
    GroupDetail(String),  // viewing members of a named group
    Fetch,
    Help,
    Comps(usize),         // similar-player comps for player at index
    Depth,                // league-wide team depth rankings
    DepthTeam(String),    // one team's depth chart with fit coloring
    Schedule,                            // weekly view with team / matchup search
    ScheduleTeam(String),                // full-season schedule for one team
    ScheduleMatchup(String, String),     // head-to-head game log between two teams
    Playoffs,                            // list-style bracket — rounds × series
    SeriesDetail(String),                // one series — keyed by series letter
    GameDetail(u64),                     // boxscore for one game — keyed by game_id
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
    // Group picker (shown as overlay on player card or team roster)
    pub group_picker_open:   bool,
    pub group_picker_list:   Vec<String>,           // group names
    pub group_picker_player: Option<(String, String)>, // (normalized, full_name)
    // Depth chart tab
    pub depth_mode:          icelines_core::cross_team::ScoringMode,
    pub show_admin:          bool,
    // Season time-travel
    pub active_season:       String,
    pub show_season_picker:  bool,
    pub picker_selected:     usize,
    // Scores (live schedule)
    pub tonight_cache:       crate::tui::tonight::TonightCache,
    pub boxscore_cache:      crate::tui::tonight::BoxscoreCache,
    pub scores_date:         String,   // "YYYY-MM-DD", empty = today
    pub scores_selected:     usize,    // selected game row
    pub scores_picker_open:  bool,     // d-key date picker visible
    pub scores_picker_input: String,   // text being typed in date picker
    pub scores_picker_err:   Option<String>,  // validation error to display
    /// When the most recent live-Scores auto-refresh was triggered. `None`
    /// means the auto-refresh timer is dormant (e.g. user has not opened the
    /// Scores tab on a live date yet). The polling loop sets this on every
    /// tick that fires; the renderer uses it to draw "Updated Xs ago".
    pub last_auto_refresh:   Option<std::time::Instant>,
    // Schedule tab — weekly view, search, team / matchup sub-views
    pub schedule_week_cache: crate::tui::schedule::WeekCache,
    pub schedule_team_cache: crate::tui::schedule::TeamSeasonCache,
    pub schedule_week:       String,   // Monday "YYYY-MM-DD" of the week being viewed
    pub schedule_query:      String,   // current text in the search bar
    pub schedule_search_mode:bool,     // true while the search bar is open
    pub schedule_filter:     crate::tui::schedule::SearchFilter, // applied filter
    pub schedule_filter_err: Option<String>,                     // search validation error
    pub schedule_selected:   usize,    // selected row on schedule
    // Playoffs tab — bracket + series detail
    pub playoffs_cache:      crate::tui::playoffs::PlayoffsCache,
    pub playoffs_round:      usize,   // round index (0-based)
    pub playoffs_series:     usize,   // series index within the current round
    // Query manager state
    pub query_fields:        Vec<crate::tui::screens::queries::QueryField>,
    pub query_field_idx:     usize,       // which field row is active
    pub query_result_scroll: usize,       // scroll offset in results panel
    pub query_mode:          QueryMode,   // build | save-name | load-list
    pub query_results_focused: bool,      // Tab toggles focus between field editor and result list
    pub query_save_name:     String,      // name being typed for save
    pub query_saved_list:    Vec<(String, String)>, // (name, json) loaded from DB
    /// Phase 8j: lazy-compiled proof dashboard panel for the player card.
    /// Only consulted when `crate::config::dashboards_enabled()` is true.
    pub dashboard_panel:     crate::tui::dashboard_panel::CompiledPanel,
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
            depth_mode:          icelines_core::cross_team::ScoringMode::Fantasy,
            show_admin:          false,
            active_season:       icelines_core::CURRENT_SEASON_STR.to_owned(),
            show_season_picker:  false,
            picker_selected:     0,
            tonight_cache:       crate::tui::tonight::new_cache(),
            boxscore_cache:      crate::tui::tonight::new_boxscore_cache(),
            scores_date:         String::new(),
            scores_selected:     0,
            scores_picker_open:  false,
            scores_picker_input: String::new(),
            scores_picker_err:   None,
            last_auto_refresh:   None,
            schedule_week_cache: crate::tui::schedule::new_week_cache(),
            schedule_team_cache: crate::tui::schedule::new_team_cache(),
            schedule_week:       crate::tui::schedule::monday_of(
                                     &crate::tui::schedule::today_iso()
                                 ).unwrap_or_else(crate::tui::schedule::today_iso),
            schedule_query:      String::new(),
            schedule_search_mode:false,
            schedule_filter:     crate::tui::schedule::SearchFilter::None,
            schedule_filter_err: None,
            schedule_selected:   0,
            playoffs_cache:      crate::tui::playoffs::new_cache(),
            playoffs_round:      0,
            playoffs_series:     0,
            group_picker_open:   false,
            group_picker_list:   Vec::new(),
            group_picker_player: None,
            headshot_cache:      crate::tui::headshot::HeadshotCache::new(),
            query_mode:          QueryMode::Build,
            query_results_focused: false,
            query_save_name:     String::new(),
            query_saved_list:    Vec::new(),
            dashboard_panel:     crate::tui::dashboard_panel::CompiledPanel::new(),
        }
    }

    /// Handle an action. Returns true if the app should quit.
    pub fn handle(&mut self, action: Action) -> bool {
        if self.show_help {
            self.show_help = false;
            return false;
        }

        if self.show_admin {
            match action {
                Action::Quit => return true,
                Action::Back | Action::Escape => self.show_admin = false,
                _ => {}
            }
            return false;
        }

        if self.show_season_picker {
            return self.handle_season_picker(action);
        }

        // Schedule search bar consumes all character-bearing actions while open.
        if self.screen == Screen::Schedule && self.schedule_search_mode {
            return self.handle_schedule_search(action);
        }

        // Scores date picker consumes input similarly.
        if self.screen == Screen::Tonight && self.scores_picker_open {
            return self.handle_scores_date_picker(action);
        }

        match action {
            Action::Quit => return true,
            Action::Help => self.show_help = true,
            Action::Back | Action::Escape => {
                if self.group_picker_open {
                    self.group_picker_open = false;
                    self.group_picker_player = None;
                    self.selected = 0;
                    self.status = "  g = add to group from any player card or team roster".to_owned();
                } else if self.screen == Screen::Queries && self.query_mode != QueryMode::Build {
                    self.query_mode = QueryMode::Build;
                    self.status = "Cancelled  ·  s=save  l=load  r=reset".to_owned();
                } else {
                    self.go_back();
                }
            }
            Action::Down => {
                if self.screen == Screen::Tonight {
                    self.scores_selected = self.scores_selected.saturating_add(1);
                } else if matches!(self.screen, Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _)) {
                    self.schedule_selected = self.schedule_selected.saturating_add(1);
                } else if self.screen == Screen::Playoffs {
                    self.playoffs_series = self.playoffs_series.saturating_add(1);
                } else if self.screen == Screen::Queries {
                    if self.query_results_focused {
                        let results = crate::tui::screens::queries::run_query(&self.players, &self.query_fields);
                        let visible: usize = 20;
                        if self.selected + 1 < visible {
                            self.selected = (self.selected + 1).min(results.len().saturating_sub(1));
                        } else {
                            let max_scroll = results.len().saturating_sub(visible);
                            self.query_result_scroll = (self.query_result_scroll + 1).min(max_scroll);
                        }
                    } else {
                        let n = self.query_fields.len();
                        if self.query_field_idx + 1 < n {
                            self.query_field_idx += 1;
                        } else {
                            self.query_results_focused = true;
                            self.selected = 0;
                            self.query_result_scroll = 0;
                        }
                    }
                } else if self.screen == Screen::Home {
                    let n = crate::tui::screens::home::RANKED_TEAMS.len();
                    self.selected = if self.selected + 1 >= n { 0 } else { self.selected + 1 };
                } else {
                    self.selected = self.selected.saturating_add(1);
                }
            }
            Action::Up => {
                if self.screen == Screen::Tonight {
                    self.scores_selected = self.scores_selected.saturating_sub(1);
                } else if matches!(self.screen, Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _)) {
                    self.schedule_selected = self.schedule_selected.saturating_sub(1);
                } else if self.screen == Screen::Playoffs {
                    self.playoffs_series = self.playoffs_series.saturating_sub(1);
                } else if self.screen == Screen::Queries {
                    if self.query_results_focused {
                        if self.selected > 0 {
                            self.selected -= 1;
                        } else if self.query_result_scroll > 0 {
                            self.query_result_scroll -= 1;
                        } else {
                            self.query_results_focused = false;
                            self.query_field_idx = self.query_fields.len().saturating_sub(1);
                        }
                    } else {
                        self.query_field_idx = self.query_field_idx.saturating_sub(1);
                    }
                } else if self.screen == Screen::Home {
                    let n = crate::tui::screens::home::RANKED_TEAMS.len();
                    self.selected = if self.selected == 0 { n - 1 } else { self.selected - 1 };
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            Action::Space => {
                if self.screen == Screen::Queries && self.query_mode == QueryMode::Build {
                    self.query_results_focused = !self.query_results_focused;
                    self.selected = 0;
                    if !self.query_results_focused {
                        self.query_field_idx = 0;
                    }
                }
            }
            Action::Right | Action::Left => {
                match &self.screen {
                    Screen::Queries if !self.query_results_focused => {
                        if let Some(f) = self.query_fields.get_mut(self.query_field_idx) {
                            if matches!(action, Action::Right) { f.next(); } else { f.prev(); }
                        }
                        self.query_result_scroll = 0;
                    }
                    // Scores: ←/→ moves the date by one day
                    Screen::Tonight => {
                        let from = if self.scores_date.is_empty() {
                            crate::tui::schedule::today_iso()
                        } else {
                            self.scores_date.clone()
                        };
                        let delta = if matches!(action, Action::Right) { 1 } else { -1 };
                        if let Some(new_date) = crate::tui::schedule::add_days(&from, delta) {
                            self.scores_date = new_date.clone();
                            self.scores_selected = 0;
                            crate::tui::tonight::maybe_fetch(
                                self.tonight_cache.clone(), new_date.clone(),
                            );
                            // Past dates don't poll — clear the auto-refresh timer.
                            self.last_auto_refresh = None;
                            self.status = format!("Scores · {new_date}");
                        }
                    }
                    // Schedule: ←/→ moves between weeks (overrides global sub-view nav)
                    Screen::Schedule => {
                        let delta = if matches!(action, Action::Right) { 7 } else { -7 };
                        if let Some(new_week) = crate::tui::schedule::add_days(&self.schedule_week, delta) {
                            self.schedule_week = new_week.clone();
                            self.schedule_selected = 0;
                            crate::tui::schedule::maybe_fetch_week(
                                self.schedule_week_cache.clone(), new_week.clone(),
                            );
                            self.status = format!(
                                "Week of {}",
                                crate::tui::schedule::week_label(&new_week)
                            );
                        }
                    }
                    // Playoffs: ←/→ moves between rounds
                    Screen::Playoffs => {
                        let n_rounds = self.playoffs_round_count();
                        if n_rounds > 0 {
                            self.playoffs_round = if matches!(action, Action::Right) {
                                (self.playoffs_round + 1).min(n_rounds - 1)
                            } else {
                                self.playoffs_round.saturating_sub(1)
                            };
                            self.playoffs_series = 0;
                        }
                    }
                    // Sub-view switching: League ↔ Depth
                    Screen::Home => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Depth;
                        self.selected = 0;
                    }
                    Screen::Depth => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Home;
                        self.selected = 0;
                    }
                    // Sub-view switching: Projections ↔ Queries
                    Screen::Projections => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Queries;
                        self.selected = 0;
                        self.query_results_focused = false;
                    }
                    Screen::Queries if self.query_results_focused => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Projections;
                        self.selected = 0;
                    }
                    _ => {}
                }
            }
            Action::Enter => self.activate_selected(),
            Action::Search => {
                // On Schedule, '/' opens the in-tab search bar instead of the
                // global player Search screen.
                if self.screen == Screen::Schedule {
                    self.schedule_search_mode = true;
                    self.schedule_query.clear();
                    self.schedule_filter_err = None;
                    self.status = "Search: type team (SEA) or matchup (NYR WSH) — Enter, Esc cancel".to_owned();
                } else {
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Search;
                    self.search_query.clear();
                    self.selected = 0;
                }
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
                } else if let Screen::Player(idx) = self.screen {
                    if c == 'c' {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Comps(idx);
                        self.selected = 0;
                    }
                } else if matches!(self.screen, Screen::Depth | Screen::DepthTeam(_)) && c == 's' {
                    self.depth_mode = self.depth_mode.toggle();
                    self.status = format!("Scoring: {}", self.depth_mode.label());
                } else if self.screen == Screen::Schedule && c == 't' {
                    // Jump to today's week
                    let today = crate::tui::schedule::today_iso();
                    if let Some(monday) = crate::tui::schedule::monday_of(&today) {
                        self.schedule_week = monday.clone();
                        self.schedule_selected = 0;
                        crate::tui::schedule::maybe_fetch_week(
                            self.schedule_week_cache.clone(), monday.clone(),
                        );
                        self.status = format!(
                            "Today — week of {}",
                            crate::tui::schedule::week_label(&monday)
                        );
                    }
                } else if self.screen == Screen::Tonight && c == 'd' {
                    // Open the scores date picker overlay
                    self.scores_picker_open = true;
                    self.scores_picker_input.clear();
                    self.scores_picker_err = None;
                    self.status = "Go to date — type YYYY-MM-DD or MM/DD, Enter applies, Esc cancels".to_owned();
                } else if self.screen == Screen::Tonight && c == 't' {
                    // 't' on Scores jumps back to today (live)
                    self.scores_date.clear();
                    self.scores_selected = 0;
                    crate::tui::tonight::maybe_fetch(
                        self.tonight_cache.clone(), String::new(),
                    );
                    // Re-arm the auto-refresh timer for the live date.
                    self.last_auto_refresh = Some(std::time::Instant::now());
                    self.status = "Scores · Today".to_owned();
                } else if c == 'F' {
                    self.show_admin = !self.show_admin;
                } else if c == 'y' {
                    self.show_season_picker = true;
                    // Start picker on current active season
                    let season_list = crate::tui::screens::misc::PICKER_SEASONS;
                    self.picker_selected = season_list.iter()
                        .position(|(id, _, _)| *id == self.active_season.as_str())
                        .unwrap_or(0);
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
                    self.query_fields = crate::tui::screens::queries::default_fields();
                    self.query_field_idx = 0;
                    self.query_result_scroll = 0;
                    self.status = "Query fields reset.".to_owned();
                } else if self.screen == Screen::Tonight {
                    // Force refresh scores for the active date
                    crate::tui::tonight::force_fetch(
                        self.tonight_cache.clone(),
                        self.scores_date.clone(),
                    );
                    self.status = "Refreshing scores…".to_owned();
                } else if self.screen == Screen::Schedule {
                    crate::tui::schedule::force_fetch_week(
                        self.schedule_week_cache.clone(),
                        self.schedule_week.clone(),
                    );
                    self.status = format!("Retrying {}…", crate::tui::schedule::week_label(&self.schedule_week));
                } else if matches!(self.screen, Screen::Playoffs | Screen::SeriesDetail(_)) {
                    if let Some(year) = crate::tui::playoffs::playoff_year_for_season(&self.active_season) {
                        crate::tui::playoffs::force_fetch_bracket(
                            self.playoffs_cache.clone(), year, &self.active_season,
                        );
                        self.status = format!("Retrying playoff bracket {year}…");
                    }
                } else {
                    self.status = "Refreshing… (run icelines fetch all)".to_owned();
                }
            }
            Action::AddToGroup => {
                let target_player = self.get_selected_player();

                if let Some(player) = target_player {
                    self.group_picker_list = crate::db::GroupDb::open()
                        .ok()
                        .and_then(|db| db.list_groups().ok())
                        .map(|gs| gs.into_iter().map(|g| g.name).collect())
                        .unwrap_or_default();
                    if self.group_picker_list.is_empty() {
                        self.status = "No groups — create one with `icelines group create`".to_owned();
                    } else {
                        self.group_picker_player = Some(player);
                        self.group_picker_open = true;
                        self.selected = 0;
                        self.status = "Add to group — ↑↓ select · Enter · Esc cancel".to_owned();
                    }
                }
            }
            Action::AddToFavorites => {
                // Instant add to "Favorites" — no picker, one key
                // Reuse the same player-detection logic as AddToGroup
                let target = self.get_selected_player();
                if let Some((norm, full)) = target {
                    if let Ok(db) = crate::db::GroupDb::open() {
                        match db.add_member("Favorites", &norm) {
                            Ok(true)  => self.status = format!("★ Added {} to Favorites", full),
                            Ok(false) => self.status = format!("★ {} is already in Favorites", full),
                            Err(e)    => self.status = format!("Error: {e}"),
                        }
                    }
                }
            }

            Action::GoToTab(n) => {
                // 1–6: League, Stats, Scores, Schedule, Groups, Playoffs
                let tabs = [
                    Screen::Home,
                    Screen::Projections,
                    Screen::Tonight,
                    Screen::Schedule,
                    Screen::Groups,
                    Screen::Playoffs,
                ];
                if let Some(screen) = tabs.get(n) {
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = screen.clone();
                    self.selected = 0;
                    self.query_result_scroll = 0;
                    self.query_results_focused = false;
                    self.group_picker_open = false;
                    self.schedule_selected = 0;
                    self.maybe_fetch_scores();
                    self.maybe_fetch_schedule();
                    self.maybe_fetch_playoffs();
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

    /// Trigger a background schedule fetch if on the Scores tab and cache is Idle.
    /// Also (re)arms the auto-refresh timer so the next 30s tick starts from now.
    fn maybe_fetch_scores(&mut self) {
        if self.screen == Screen::Tonight {
            crate::tui::tonight::maybe_fetch(
                self.tonight_cache.clone(),
                self.scores_date.clone(),
            );
            // Arm the timer only when on a live date — past dates are
            // permanent (final scores don't change) and don't need polling.
            self.last_auto_refresh = if self.scores_date.is_empty() {
                Some(std::time::Instant::now())
            } else {
                None
            };
        }
    }

    /// Run one tick of the auto-refresh timer. Called by the TUI event loop
    /// every poll cycle (~10 fps). When the conditions in `should_auto_refresh`
    /// are met, triggers a silent `force_fetch` and resets the timer.
    /// Phase 8f.1: also short-circuits when live feeds are disabled so the
    /// timer doesn't repeatedly write the "live disabled" error to the cache.
    pub fn tick_auto_refresh(&mut self) {
        if !crate::config::live_feeds_enabled() { return; }
        let now = std::time::Instant::now();
        if should_auto_refresh(
            &self.screen,
            &self.scores_date,
            self.last_auto_refresh,
            now,
            SCORES_AUTO_REFRESH_INTERVAL,
        ) {
            crate::tui::tonight::force_fetch(
                self.tonight_cache.clone(),
                self.scores_date.clone(),
            );
            self.last_auto_refresh = Some(now);
        }
    }

    /// game_id of the currently-highlighted game on Scores, if any.
    pub fn selected_game_id(&self) -> Option<u64> {
        use crate::tui::tonight::{lookup, TonightState};
        let state = lookup(&self.tonight_cache, &self.scores_date);
        match state {
            TonightState::Loaded(games) => {
                let idx = self.scores_selected.min(games.len().saturating_sub(1));
                games.get(idx).map(|g| g.game_id)
            }
            _ => None,
        }
    }

    /// Apply the date typed into the d-key picker. Accepts `YYYY-MM-DD` and
    /// `MM/DD` (current year inferred). Empty input clears back to "today".
    fn apply_scores_date_picker(&mut self) {
        let raw = self.scores_picker_input.trim();
        if raw.is_empty() {
            self.scores_date.clear();
            self.scores_picker_open = false;
            self.scores_picker_err  = None;
            self.scores_selected    = 0;
            crate::tui::tonight::maybe_fetch(
                self.tonight_cache.clone(), String::new(),
            );
            // Empty date = live → arm the timer
            self.last_auto_refresh = Some(std::time::Instant::now());
            self.status = "Scores · Today".to_owned();
            return;
        }
        match parse_picker_date(raw) {
            Ok(iso) => {
                self.scores_date         = iso.clone();
                self.scores_picker_open  = false;
                self.scores_picker_err   = None;
                self.scores_picker_input.clear();
                self.scores_selected     = 0;
                crate::tui::tonight::maybe_fetch(
                    self.tonight_cache.clone(), iso.clone(),
                );
                // Specific date → no auto-refresh (final scores don't change)
                self.last_auto_refresh = None;
                self.status = format!("Scores · {iso}");
            }
            Err(msg) => {
                self.scores_picker_err = Some(msg.clone());
                self.status = format!("⚠ {msg}");
            }
        }
    }

    /// Handle key events when the Scores date picker overlay is open.
    fn handle_scores_date_picker(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                self.scores_picker_open = false;
                self.scores_picker_input.clear();
                self.scores_picker_err  = None;
                self.status = "Date picker cancelled.".to_owned();
            }
            Action::Enter      => self.apply_scores_date_picker(),
            Action::Backspace  => { self.scores_picker_input.pop(); self.scores_picker_err = None; }
            Action::Char(c)    => self.scores_picker_input.push(c),
            // Map non-text actions back to their characters so digits/letters
            // typed at the picker behave naturally.
            Action::Refresh        => self.scores_picker_input.push('r'),
            Action::Install        => self.scores_picker_input.push('i'),
            Action::AddToGroup     => self.scores_picker_input.push('g'),
            Action::AddToFavorites => self.scores_picker_input.push('f'),
            Action::GoToTab(n)     => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.scores_picker_input.push(ch);
            }
            _ => {}
        }
        false
    }

    /// Number of rounds in the currently-cached playoff bracket (0 if unloaded).
    pub fn playoffs_round_count(&self) -> usize {
        let year = match crate::tui::playoffs::playoff_year_for_season(&self.active_season) {
            Some(y) => y,
            None    => return 0,
        };
        let map = self.playoffs_cache.lock().unwrap();
        match map.get(&year) {
            Some(crate::tui::playoffs::PlayoffsState::Loaded(b)) => b.rounds.len(),
            _ => 0,
        }
    }

    /// Letter of the currently-selected series (used as SeriesDetail key).
    pub fn selected_series_letter(&self) -> Option<String> {
        let year = crate::tui::playoffs::playoff_year_for_season(&self.active_season)?;
        let map  = self.playoffs_cache.lock().unwrap();
        match map.get(&year) {
            Some(crate::tui::playoffs::PlayoffsState::Loaded(b)) => {
                let round = b.rounds.get(self.playoffs_round)?;
                let series = round.series.get(self.playoffs_series)?;
                series.letter.clone()
            }
            _ => None,
        }
    }

    /// Trigger a background bracket fetch when entering the Playoffs tab.
    fn maybe_fetch_playoffs(&mut self) {
        if matches!(self.screen, Screen::Playoffs | Screen::SeriesDetail(_)) {
            if let Some(year) = crate::tui::playoffs::playoff_year_for_season(&self.active_season) {
                crate::tui::playoffs::maybe_fetch_bracket(
                    self.playoffs_cache.clone(), year, &self.active_season,
                );
            }
        }
    }

    /// Pre-fetch current week + 2 weeks ahead when entering the Schedule tab,
    /// or fetch the active sub-view's data.
    fn maybe_fetch_schedule(&mut self) {
        match &self.screen {
            Screen::Schedule => {
                crate::tui::schedule::prefetch_around(
                    self.schedule_week_cache.clone(),
                    &self.schedule_week,
                );
            }
            Screen::ScheduleTeam(team) => {
                crate::tui::schedule::maybe_fetch_team(
                    self.schedule_team_cache.clone(),
                    team.clone(),
                    self.active_season.clone(),
                );
            }
            Screen::ScheduleMatchup(t1, _t2) => {
                // Matchup view derives from one team's full season schedule
                crate::tui::schedule::maybe_fetch_team(
                    self.schedule_team_cache.clone(),
                    t1.clone(),
                    self.active_season.clone(),
                );
            }
            _ => {}
        }
    }

    /// Apply current `schedule_query` text — validate teams, set filter, exit search mode.
    fn apply_schedule_query(&mut self) {
        match crate::tui::schedule::parse_search(&self.schedule_query) {
            Ok(filter) => {
                self.schedule_filter = filter;
                self.schedule_filter_err = None;
                self.schedule_search_mode = false;
                self.schedule_selected = 0;
                self.status = match &self.schedule_filter {
                    crate::tui::schedule::SearchFilter::None => {
                        "Filter cleared.".to_owned()
                    }
                    crate::tui::schedule::SearchFilter::Team(t) => {
                        format!("Filter: {t} — Enter to view full schedule")
                    }
                    crate::tui::schedule::SearchFilter::Matchup(a, b) => {
                        format!("Filter: {a} vs {b} — Enter for head-to-head")
                    }
                };
            }
            Err(msg) => {
                // Keep search mode open so the user can correct the input
                self.schedule_filter_err = Some(msg.clone());
                self.status = format!("⚠ {msg}");
            }
        }
    }

    /// Handle key events while the Schedule search bar is active.
    fn handle_schedule_search(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                // Cancel — exit search mode, clear pending input + error
                self.schedule_search_mode = false;
                self.schedule_query.clear();
                self.schedule_filter_err = None;
                self.status = "Search cancelled.".to_owned();
            }
            Action::Enter => self.apply_schedule_query(),
            Action::Backspace => { self.schedule_query.pop(); self.schedule_filter_err = None; }
            Action::Char(c)         => self.schedule_query.push(c),
            Action::Space           => self.schedule_query.push(' '),
            // While in search mode, hotkeys are treated as text input so
            // queries like "nyr" can be typed without firing Refresh/Install/etc.
            Action::Refresh         => self.schedule_query.push('r'),
            Action::Install         => self.schedule_query.push('i'),
            Action::AddToGroup      => self.schedule_query.push('g'),
            Action::AddToFavorites  => self.schedule_query.push('f'),
            Action::GoToTab(n)      => {
                // Map digit-tabs back to their numeric character
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.schedule_query.push(ch);
            }
            // '/' while already in search mode — ignore (don't reopen, don't insert)
            Action::Search => {}
            // Up/Down/Left/Right/Tab/Help — ignored in search mode for now
            _ => {}
        }
        false
    }

    /// Handle key events when the season picker overlay is open.
    /// Returns true only if user pressed q (quit).
    fn handle_season_picker(&mut self, action: Action) -> bool {
        use crate::tui::screens::misc::PICKER_SEASONS;
        let n = PICKER_SEASONS.len();
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                self.show_season_picker = false;
            }
            Action::Down => {
                self.picker_selected = (self.picker_selected + 1).min(n.saturating_sub(1));
            }
            Action::Up => {
                self.picker_selected = self.picker_selected.saturating_sub(1);
            }
            Action::Enter => {
                if let Some(&(season_id, _, is_lockout)) = PICKER_SEASONS.get(self.picker_selected) {
                    if is_lockout {
                        self.status = "No season data — lockout year (2004-05).".to_owned();
                    } else {
                        let is_bundled = icelines_fetch::bundled::BUNDLED_SEASONS.contains(&season_id);
                        let is_installed = icelines_fetch::bundled::is_installed(season_id);
                        if is_bundled || is_installed {
                            self.reload_for_season(season_id);
                            self.show_season_picker = false;
                        } else {
                            self.status = format!(
                                "Season {} not installed. Press 'i' to install, or run `icelines data install {}`.",
                                season_id, season_id
                            );
                        }
                    }
                }
            }
            Action::Char('i') => {
                if let Some(&(season_id, _, is_lockout)) = PICKER_SEASONS.get(self.picker_selected) {
                    if is_lockout {
                        self.status = "Cannot install — lockout year has no data.".to_owned();
                    } else if icelines_fetch::bundled::is_installed(season_id) {
                        self.status = format!("Season {} is already installed.", season_id);
                    } else {
                        let season = season_id.to_owned();
                        let state = self.install_state.clone();
                        crate::tui::loader::spawn_install(season, state);
                        self.status = format!("Installing {}…", season_id);
                    }
                }
            }
            _ => {}
        }
        false
    }

    /// Reload app.players from the given season (bundled or installed).
    fn reload_for_season(&mut self, season_id: &str) {
        use icelines_fetch::{bundled, player_builder};
        use std::collections::HashMap;

        let bios = bundled::get_bios(season_id)
            .or_else(|| bundled::get_bios_installed(season_id));
        let stats = bundled::get_stats(season_id)
            .or_else(|| bundled::get_stats_installed(season_id));

        let players = if let Some(bios) = bios {
            let stats_idx = stats.as_ref()
                .map(|s| player_builder::index_stats(s))
                .unwrap_or_default();
            player_builder::build_players_from_bios(
                &bios, &stats_idx,
                &HashMap::new(), &HashMap::new(), &HashMap::new(),
                icelines_core::model::Season(
                    season_id.parse().unwrap_or(icelines_core::CURRENT_SEASON)
                ),
            )
        } else {
            Vec::new()
        };

        self.active_season = season_id.to_owned();
        self.players = players;
        self.selected = 0;

        if season_id == icelines_core::CURRENT_SEASON_STR {
            self.status = "Current season loaded.".to_owned();
        } else {
            let label = crate::tui::screens::misc::PICKER_SEASONS.iter()
                .find(|(id, _, _)| *id == season_id)
                .map(|(_, label, _)| *label)
                .unwrap_or(season_id);
            self.status = format!("[{}] — historical season. Live features unavailable.", label);
        }
    }

    fn go_back(&mut self) {
        self.screen = if let Some(prev) = self.prev_screen.take() {
            prev
        } else {
            // Sensible parent for each drill-down screen when prev_screen is unset
            match &self.screen {
                Screen::DepthTeam(_)        => Screen::Depth,
                Screen::Team(_)             => Screen::Home,
                Screen::Player(_)           => Screen::Home,
                Screen::Comps(_)            => Screen::Home,
                Screen::GroupDetail(_)      => Screen::Groups,
                Screen::ScheduleTeam(_)     => Screen::Schedule,
                Screen::ScheduleMatchup(..) => Screen::Schedule,
                Screen::SeriesDetail(_)     => Screen::Playoffs,
                Screen::GameDetail(_)       => Screen::Tonight,
                _                           => Screen::Home,
            }
        };
        self.selected = 0;
        self.query_results_focused = false;
    }

    /// Return the (normalized_name, full_name) of the currently highlighted player
    /// on whichever screen is active. Returns None on screens with no player list.
    fn get_selected_player(&self) -> Option<(String, String)> {
        match &self.screen {
            Screen::Player(idx) => self.players.get(*idx)
                .map(|p| (p.name_normalized.clone(), p.full_name.clone())),

            Screen::Team(abbrev) => {
                let abbrev = abbrev.clone();
                self.players.iter()
                    .filter(|p| p.team.as_str() == abbrev.as_str())
                    .nth(self.selected)
                    .map(|p| (p.name_normalized.clone(), p.full_name.clone()))
            }

            Screen::Projections => {
                let mut sorted: Vec<&icelines_core::model::Player> = self.players.iter()
                    .filter(|p| p.pace_score.is_some()).collect();
                sorted.sort_by(|a, b| {
                    let sa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                    let sb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
                    sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted.get(self.selected)
                    .map(|p| (p.name_normalized.clone(), p.full_name.clone()))
            }

            Screen::Search => {
                let norm = icelines_core::name::normalize_name(&self.search_query);
                let filtered: Vec<&icelines_core::model::Player> = self.players.iter()
                    .filter(|p| p.name_normalized.contains(&norm))
                    .collect();
                filtered.get(self.selected)
                    .map(|p| (p.name_normalized.clone(), p.full_name.clone()))
            }

            Screen::Queries => {
                let results = crate::tui::screens::queries::run_query(&self.players, &self.query_fields);
                let row_idx = self.query_result_scroll
                    + self.selected.min(results.len().saturating_sub(1));
                results.get(row_idx)
                    .map(|(_, p)| (p.name_normalized.clone(), p.full_name.clone()))
            }

            Screen::GroupDetail(group_name) => {
                let gn = group_name.clone();
                crate::db::GroupDb::open().ok()
                    .and_then(|db| db.list_members(&gn).ok())
                    .and_then(|members| members.get(self.selected).cloned()
                        .and_then(|norm| self.players.iter()
                            .find(|p| p.name_normalized.contains(&norm))
                            .map(|p| (p.name_normalized.clone(), p.full_name.clone()))
                        )
                    )
            }

            Screen::Comps(target_idx) => {
                let target_idx = *target_idx;
                self.players.get(target_idx).and_then(|target| {
                    crate::tui::screens::comps::find_comps(&self.players, target)
                        .get(self.selected)
                        .map(|p| (p.name_normalized.clone(), p.full_name.clone()))
                })
            }

            _ => None,
        }
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
            // Group picker overlay (shown on player card OR team roster)
            _ if self.group_picker_open => {
                if let Some(group_name) = self.group_picker_list.get(self.selected).cloned() {
                    if let Some((norm, full)) = self.group_picker_player.take() {
                        if let Ok(db) = crate::db::GroupDb::open() {
                            match db.add_member(&group_name, &norm) {
                                Ok(true)  => self.status = format!("✓ Added {} to '{}'", full, group_name),
                                Ok(false) => self.status = format!("'{}' is already in '{}'", full, group_name),
                                Err(e)    => self.status = format!("Error: {e}"),
                            }
                        }
                    }
                    self.group_picker_open = false;
                    self.selected = 0;
                }
            }
            Screen::Groups => {
                // Enter on a group row → open group detail view
                let groups = crate::db::GroupDb::open()
                    .ok()
                    .and_then(|db| db.list_groups().ok())
                    .unwrap_or_default();
                if let Some(g) = groups.get(self.selected) {
                    self.prev_screen = Some(Screen::Groups);
                    self.screen = Screen::GroupDetail(g.name.clone());
                    self.selected = 0;
                }
            }
            Screen::GroupDetail(_) => {
                // Enter on a member row → player card
                if let Screen::GroupDetail(ref group_name) = self.screen.clone() {
                    let members = crate::db::GroupDb::open()
                        .ok()
                        .and_then(|db| db.list_members(group_name).ok())
                        .unwrap_or_default();
                    if let Some(norm) = members.get(self.selected) {
                        if let Some(global_idx) = self.players.iter().position(|p| p.name_normalized.contains(norm.as_str())) {
                            self.prev_screen = Some(self.screen.clone());
                            self.screen = Screen::Player(global_idx);
                            self.selected = 0;
                        }
                    }
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
            Screen::Depth => {
                let strength = icelines_core::cross_team::compute_team_strength(
                    &self.players, self.depth_mode
                );
                let mut ranked: Vec<String> = strength.keys().cloned().collect();
                ranked.sort_by(|a, b| {
                    strength[b].total.partial_cmp(&strength[a].total)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                if let Some(team) = ranked.get(self.selected) {
                    self.prev_screen = Some(Screen::Depth);
                    self.screen = Screen::DepthTeam(team.clone());
                    self.selected = 0;
                }
            }
            Screen::Comps(target_idx) => {
                let target_idx = *target_idx;
                if let Some(target) = self.players.get(target_idx) {
                    let comps = crate::tui::screens::comps::find_comps(&self.players, target);
                    if let Some(comp) = comps.get(self.selected) {
                        if let Some(global_idx) = self.players.iter().position(|p| p.nhl_id == comp.nhl_id) {
                            self.prev_screen = Some(self.screen.clone());
                            self.screen = Screen::Player(global_idx);
                            self.selected = 0;
                        }
                    }
                }
            }
            // Schedule: Enter opens the team or matchup detail view if a filter is active.
            // With no filter, Enter is a no-op (the row-level player card has no analogue here).
            Screen::Schedule => {
                let next = match &self.schedule_filter {
                    crate::tui::schedule::SearchFilter::Team(t) => {
                        Some(Screen::ScheduleTeam(t.clone()))
                    }
                    crate::tui::schedule::SearchFilter::Matchup(a, b) => {
                        Some(Screen::ScheduleMatchup(a.clone(), b.clone()))
                    }
                    crate::tui::schedule::SearchFilter::None => None,
                };
                if let Some(target) = next {
                    self.prev_screen = Some(Screen::Schedule);
                    self.screen = target;
                    self.schedule_selected = 0;
                    self.maybe_fetch_schedule();
                }
            }
            // Playoffs: Enter on a series row opens SeriesDetail keyed by series letter.
            Screen::Playoffs => {
                if let Some(letter) = self.selected_series_letter() {
                    self.prev_screen = Some(Screen::Playoffs);
                    self.screen = Screen::SeriesDetail(letter);
                }
            }
            // Scores: Enter on a game row opens GameDetail keyed by game_id.
            Screen::Tonight => {
                if let Some(game_id) = self.selected_game_id() {
                    self.prev_screen = Some(Screen::Tonight);
                    self.screen = Screen::GameDetail(game_id);
                    crate::tui::tonight::maybe_fetch_boxscore(
                        self.boxscore_cache.clone(), game_id,
                    );
                }
            }
            _ => {}
        }
    }

    fn cycle_screen(&mut self) {
        self.query_results_focused = false;
        let next = match &self.screen {
            // League tab → Stats tab
            Screen::Home | Screen::Depth | Screen::DepthTeam(_)
            | Screen::Team(_) | Screen::Player(_) | Screen::Comps(_) => Screen::Projections,
            // Stats tab → Scores tab
            Screen::Projections | Screen::Queries | Screen::Search  => Screen::Tonight,
            // Scores → Schedule → Groups → Playoffs → League
            Screen::Tonight | Screen::GameDetail(_) => Screen::Schedule,
            Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(..) => Screen::Groups,
            Screen::Groups | Screen::GroupDetail(_) => Screen::Playoffs,
            Screen::Playoffs | Screen::SeriesDetail(_) => Screen::Home,
            _                 => Screen::Home,
        };
        self.screen = next;
        self.selected = 0;
        self.schedule_selected = 0;
        self.query_result_scroll = 0;
        self.maybe_fetch_scores();
        self.maybe_fetch_schedule();
        self.maybe_fetch_playoffs();
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
        // v2: 6 tabs — League→Stats→Scores→Schedule→Groups→Playoffs→League
        let mut app = App::new(false);
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Projections, "Home→Stats(Projections)");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Tonight, "Stats→Scores");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Schedule, "Scores→Schedule");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Groups, "Schedule→Groups");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Playoffs, "Groups→Playoffs");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Home, "Playoffs→League (wraps)");
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
        // Use Groups screen — no wrap, linear selection
        app.screen = Screen::Groups;
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

    // ── Schedule (Phase 7d) ──────────────────────────────────────────────────

    use crate::tui::schedule::SearchFilter;

    #[test]
    fn l0_tui_schedule_search_opens_search_mode() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        assert!(app.schedule_search_mode, "search mode should be open");
        assert_eq!(app.screen, Screen::Schedule, "stays on Schedule, not the global Search screen");
        assert!(app.schedule_query.is_empty());
    }

    #[test]
    fn l0_tui_schedule_search_single_team() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        for c in "SEA".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Enter);
        assert!(!app.schedule_search_mode);
        assert_eq!(app.schedule_filter, SearchFilter::Team("SEA".to_owned()));
    }

    #[test]
    fn l0_tui_schedule_search_matchup() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        for c in "NYR".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Space);
        for c in "WSH".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Enter);
        assert_eq!(
            app.schedule_filter,
            SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned()),
        );
    }

    #[test]
    fn l0_tui_schedule_invalid_team_error() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        for c in "XYZ".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Enter);
        // Search bar stays open on validation failure so user can correct
        assert!(app.schedule_search_mode);
        assert!(app.schedule_filter_err.as_deref().unwrap_or("").contains("Unknown team"));
        // Filter unchanged from default
        assert_eq!(app.schedule_filter, SearchFilter::None);
    }

    #[test]
    fn l0_tui_schedule_left_right_changes_week() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        let initial = app.schedule_week.clone();
        app.handle(Action::Right);
        let after_right = app.schedule_week.clone();
        assert_ne!(initial, after_right, "week should advance");
        app.handle(Action::Left);
        assert_eq!(app.schedule_week, initial, "left should restore");
    }

    #[test]
    fn l0_tui_schedule_search_consumes_r_as_text() {
        // 'r' lowercase fires Action::Refresh globally; while the schedule
        // search bar is open it should append 'r' to the query instead.
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        app.handle(Action::Char('N'));
        app.handle(Action::Char('Y'));
        app.handle(Action::Refresh);    // mapped from lowercase 'r'
        assert_eq!(app.schedule_query, "NYr");
    }

    #[test]
    fn l0_tui_schedule_team_filter_enter_opens_team_view() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.schedule_filter = SearchFilter::Team("SEA".to_owned());
        app.handle(Action::Enter);
        assert_eq!(app.screen, Screen::ScheduleTeam("SEA".to_owned()));
    }

    #[test]
    fn l0_tui_schedule_matchup_filter_enter_opens_matchup_view() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.schedule_filter = SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned());
        app.handle(Action::Enter);
        assert_eq!(
            app.screen,
            Screen::ScheduleMatchup("NYR".to_owned(), "WSH".to_owned()),
        );
    }

    #[test]
    fn l0_tui_schedule_esc_cancels_search_mode() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        app.handle(Action::Char('S'));
        app.handle(Action::Escape);
        assert!(!app.schedule_search_mode);
        assert!(app.schedule_query.is_empty());
    }

    #[test]
    fn l0_tui_schedule_t_jumps_to_today() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        // Move two weeks forward, then 't' should snap back to today's Monday
        let today_monday = crate::tui::schedule::monday_of(
            &crate::tui::schedule::today_iso()
        ).unwrap();
        app.handle(Action::Right);
        app.handle(Action::Right);
        assert_ne!(app.schedule_week, today_monday);
        app.handle(Action::Char('t'));
        assert_eq!(app.schedule_week, today_monday);
    }

    #[test]
    fn l0_tui_schedule_back_from_team_returns_to_schedule() {
        let mut app = App::new(false);
        app.screen = Screen::ScheduleTeam("SEA".to_owned());
        app.handle(Action::Back);
        assert_eq!(app.screen, Screen::Schedule);
    }

    // ── Playoffs (Phase 7e) ──────────────────────────────────────────────────

    use crate::tui::playoffs::PlayoffsState;
    use icelines_fetch::nhl_api::{PlayoffBracket, PlayoffRound, PlayoffSeries};

    fn fixture_series(letter: &str, top: &str, bot: &str, top_w: u8, bot_w: u8) -> PlayoffSeries {
        PlayoffSeries {
            letter:             Some(letter.to_owned()),
            top_seed_abbrev:    top.to_owned(),
            top_seed_name:      top.to_owned(),
            top_seed_wins:      top_w,
            top_seed_rank:      None,
            bottom_seed_abbrev: bot.to_owned(),
            bottom_seed_name:   bot.to_owned(),
            bottom_seed_wins:   bot_w,
            bottom_seed_rank:   None,
            winner_abbrev:      if top_w == 4 { Some(top.to_owned()) }
                                else if bot_w == 4 { Some(bot.to_owned()) }
                                else { None },
            conference:         None,
            games:              Vec::new(),
        }
    }

    fn seed_bracket(app: &mut App, year: u16, rounds: Vec<PlayoffRound>) {
        let bracket = PlayoffBracket {
            season:        app.active_season.clone(),
            current_round: None,
            rounds,
        };
        app.playoffs_cache.lock().unwrap()
            .insert(year, PlayoffsState::Loaded(bracket));
    }

    #[test]
    fn l0_tui_playoffs_round_count_zero_when_unloaded() {
        let app = App::new(false);
        assert_eq!(app.playoffs_round_count(), 0);
    }

    #[test]
    fn l0_tui_playoffs_round_count_reflects_cache() {
        let mut app = App::new(false);
        let r1 = PlayoffRound { round_number: 1, label: "First Round".into(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2)] };
        let r2 = PlayoffRound { round_number: 2, label: "Second Round".into(),
            series: vec![] };
        seed_bracket(&mut app, 2026, vec![r1, r2]);
        assert_eq!(app.playoffs_round_count(), 2);
    }

    #[test]
    fn l0_tui_playoffs_left_right_changes_round() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        let r1 = PlayoffRound { round_number: 1, label: "First Round".into(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2)] };
        let r2 = PlayoffRound { round_number: 2, label: "Second Round".into(),
            series: vec![fixture_series("I", "FLA", "WSH", 1, 0)] };
        seed_bracket(&mut app, 2026, vec![r1, r2]);

        assert_eq!(app.playoffs_round, 0);
        app.handle(Action::Right);
        assert_eq!(app.playoffs_round, 1, "→ should advance to round 2");
        // At the last round, → clamps (no wrap)
        app.handle(Action::Right);
        assert_eq!(app.playoffs_round, 1);
        // ← walks back
        app.handle(Action::Left);
        assert_eq!(app.playoffs_round, 0);
        // At round 0, ← clamps at 0
        app.handle(Action::Left);
        assert_eq!(app.playoffs_round, 0);
    }

    #[test]
    fn l0_tui_playoffs_round_change_resets_series_cursor() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        let r1 = PlayoffRound { round_number: 1, label: "First Round".into(), series: vec![
            fixture_series("A", "FLA", "TBL", 4, 2),
            fixture_series("B", "WSH", "NYR", 4, 3),
        ] };
        let r2 = PlayoffRound { round_number: 2, label: "Second Round".into(),
            series: vec![fixture_series("I", "FLA", "WSH", 1, 0)] };
        seed_bracket(&mut app, 2026, vec![r1, r2]);

        // Move down to series 1, then change rounds — cursor resets to 0
        app.handle(Action::Down);
        assert_eq!(app.playoffs_series, 1);
        app.handle(Action::Right);
        assert_eq!(app.playoffs_round, 1);
        assert_eq!(app.playoffs_series, 0, "switching rounds resets the series cursor");
    }

    #[test]
    fn l0_tui_playoffs_enter_opens_series_detail() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        let r1 = PlayoffRound { round_number: 1, label: "First Round".into(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2)] };
        seed_bracket(&mut app, 2026, vec![r1]);
        // Cursor on first series → Enter
        app.handle(Action::Enter);
        assert_eq!(app.screen, Screen::SeriesDetail("A".to_owned()));
    }

    #[test]
    fn l0_tui_playoffs_enter_with_no_data_is_no_op() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        // Cache is empty → no series letter to select
        app.handle(Action::Enter);
        assert_eq!(app.screen, Screen::Playoffs, "Enter must not change screen when bracket isn't loaded");
    }

    #[test]
    fn l0_tui_series_detail_back_returns_to_playoffs() {
        let mut app = App::new(false);
        app.screen = Screen::SeriesDetail("A".to_owned());
        app.handle(Action::Back);
        assert_eq!(app.screen, Screen::Playoffs);
    }

    #[test]
    fn l0_tui_playoffs_left_right_no_op_when_unloaded() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        let initial = app.playoffs_round;
        app.handle(Action::Right);
        assert_eq!(app.playoffs_round, initial, "no rounds loaded → no movement");
    }

    #[test]
    fn l0_tui_tab_cycles_through_playoffs_subscreen() {
        // Tab from SeriesDetail wraps to League (same as Playoffs)
        let mut app = App::new(false);
        app.screen = Screen::SeriesDetail("A".to_owned());
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Home);
    }

    // ── Scores date nav / d picker / Enter detail (Phase 7c gap-fix) ─────────

    use crate::tui::tonight::TonightState;
    use icelines_fetch::nhl_api::ScheduledGame;

    fn fixture_scheduled_game(id: u64, away: &str, home: &str) -> ScheduledGame {
        ScheduledGame {
            game_id:        id,
            date:           "2026-04-28".to_owned(),
            game_type:      2,
            away_abbrev:    away.to_owned(),
            away_name:      away.to_owned(),
            home_abbrev:    home.to_owned(),
            home_name:      home.to_owned(),
            start_time_utc: "2026-04-28T23:00:00Z".to_owned(),
            away_score:     None,
            home_score:     None,
            game_state:     None,
            last_period:    None,
            series_game:    None,
            away_wins:      None,
            home_wins:      None,
        }
    }

    fn seed_scores(app: &mut App, date_key: &str, games: Vec<ScheduledGame>) {
        app.tonight_cache.lock().unwrap()
            .insert(date_key.to_owned(), TonightState::Loaded(games));
    }

    #[test]
    fn l0_tui_parse_picker_date_iso() {
        assert_eq!(super::parse_picker_date("2026-04-28").unwrap(), "2026-04-28");
        assert_eq!(super::parse_picker_date("2026/04/28").unwrap(), "2026-04-28");
    }

    #[test]
    fn l0_tui_parse_picker_date_mm_dd_uses_current_year() {
        let parsed = super::parse_picker_date("04/28").unwrap();
        // The year is whatever Utc::now() returns — assert prefix and structure
        assert!(parsed.ends_with("-04-28"), "must end with month-day, got: {parsed}");
        assert_eq!(parsed.len(), 10);
    }

    #[test]
    fn l0_tui_parse_picker_date_rejects_garbage() {
        assert!(super::parse_picker_date("not-a-date").is_err());
        assert!(super::parse_picker_date("").is_err());
        assert!(super::parse_picker_date("13/45").is_err()); // invalid month/day
    }

    #[test]
    fn l0_tui_scores_left_right_changes_date() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        // Start on today (empty)
        assert!(app.scores_date.is_empty());
        app.handle(Action::Right);
        assert!(!app.scores_date.is_empty(), "Right should set explicit date");
        let after_right = app.scores_date.clone();
        app.handle(Action::Left);
        let after_left = app.scores_date.clone();
        assert_ne!(after_right, after_left, "Left should move backwards");
    }

    #[test]
    fn l0_tui_scores_t_jumps_to_today() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.scores_date = "2026-01-01".to_owned();
        app.handle(Action::Char('t'));
        assert!(app.scores_date.is_empty(), "t must clear scores_date back to live");
    }

    #[test]
    fn l0_tui_scores_d_opens_picker() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        assert!(!app.scores_picker_open);
        app.handle(Action::Char('d'));
        assert!(app.scores_picker_open);
        assert!(app.scores_picker_input.is_empty());
    }

    #[test]
    fn l0_tui_scores_picker_enter_applies_iso() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.handle(Action::Char('d'));
        for c in "2026-04-28".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Enter);
        assert!(!app.scores_picker_open, "picker should close on apply");
        assert_eq!(app.scores_date, "2026-04-28");
    }

    #[test]
    fn l0_tui_scores_picker_invalid_keeps_open_with_error() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.handle(Action::Char('d'));
        for c in "garbage".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Enter);
        assert!(app.scores_picker_open, "invalid input must keep picker open for correction");
        assert!(app.scores_picker_err.is_some());
    }

    #[test]
    fn l0_tui_scores_picker_esc_cancels() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.handle(Action::Char('d'));
        for c in "abc".chars() { app.handle(Action::Char(c)); }
        app.handle(Action::Escape);
        assert!(!app.scores_picker_open);
        assert!(app.scores_picker_input.is_empty());
    }

    #[test]
    fn l0_tui_scores_picker_consumes_r_as_text() {
        // 'r' fires Action::Refresh globally; must append while picker open.
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.handle(Action::Char('d'));
        app.handle(Action::Char('2'));
        app.handle(Action::Refresh);   // → 'r'
        // 'r' isn't a valid date character but should be in the buffer
        assert_eq!(app.scores_picker_input, "2r");
    }

    #[test]
    fn l0_tui_scores_enter_opens_game_detail() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        seed_scores(&mut app, "", vec![
            fixture_scheduled_game(2025020100, "SEA", "VGK"),
            fixture_scheduled_game(2025020101, "NYR", "WSH"),
        ]);
        // Default selection (0) → first game
        app.handle(Action::Enter);
        assert_eq!(app.screen, Screen::GameDetail(2025020100));
    }

    #[test]
    fn l0_tui_game_detail_back_returns_to_tonight() {
        let mut app = App::new(false);
        app.screen = Screen::GameDetail(2025020100);
        app.handle(Action::Back);
        assert_eq!(app.screen, Screen::Tonight);
    }

    #[test]
    fn l0_tui_scores_enter_with_no_games_is_no_op() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        // Cache empty → no game_id selectable
        app.handle(Action::Enter);
        assert_eq!(app.screen, Screen::Tonight);
    }

    // ── Admin overlay (Phase 8a.2) ───────────────────────────────────────────

    #[test]
    fn l0_admin_overlay_opens_on_capital_f_key() {
        let mut app = App::new(false);
        assert!(!app.show_admin, "default state must be closed");
        app.handle(Action::Char('F'));
        assert!(app.show_admin, "capital F must open the admin overlay");
    }

    #[test]
    fn l0_admin_overlay_closes_on_esc() {
        let mut app = App::new(false);
        app.show_admin = true;
        app.handle(Action::Escape);
        assert!(!app.show_admin, "Esc must close the admin overlay");
    }

    #[test]
    fn l0_admin_overlay_blocks_other_keys() {
        let mut app = App::new(false);
        app.show_admin = true;
        let initial = app.screen.clone();
        // Tab would normally cycle screens — must be suppressed
        app.handle(Action::Tab);
        assert_eq!(app.screen, initial, "Tab while admin open must not change screen");
        assert!(app.show_admin, "Tab must not close the overlay");
        // Same for number-key tab jumps
        app.handle(Action::GoToTab(2));
        assert_eq!(app.screen, initial, "GoToTab while admin open must not change screen");
        assert!(app.show_admin);
    }

    #[test]
    fn l0_admin_overlay_does_not_open_on_lowercase_f() {
        // Lowercase 'f' is mapped to AddToFavorites in event.rs and must not
        // be confused with the capital-F admin trigger.
        let mut app = App::new(false);
        app.handle(Action::AddToFavorites);
        assert!(!app.show_admin,
            "lowercase f (AddToFavorites action) must not open the admin overlay");
    }

    // ── Scores auto-refresh (Phase 8b) ───────────────────────────────────────

    #[test]
    fn l0_scores_auto_refresh_fires_when_due() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(31);
        assert!(super::should_auto_refresh(
            &Screen::Tonight, "", Some(last), now,
            super::SCORES_AUTO_REFRESH_INTERVAL,
        ), "30s elapsed on live Scores tab must fire");
    }

    #[test]
    fn l0_scores_auto_refresh_holds_off_within_interval() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(10);
        assert!(!super::should_auto_refresh(
            &Screen::Tonight, "", Some(last), now,
            super::SCORES_AUTO_REFRESH_INTERVAL,
        ), "10s after last refresh must hold off");
    }

    #[test]
    fn l0_scores_auto_refresh_paused_off_tab() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(60);
        // Off the Scores tab → must not fire even if the interval passed.
        for screen in [Screen::Home, Screen::Schedule, Screen::Playoffs, Screen::Groups,
                       Screen::Projections, Screen::GameDetail(1234)] {
            assert!(!super::should_auto_refresh(
                &screen, "", Some(last), now,
                super::SCORES_AUTO_REFRESH_INTERVAL,
            ), "screen {screen:?} must not auto-refresh Scores");
        }
    }

    #[test]
    fn l0_scores_auto_refresh_paused_on_past_date() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(60);
        assert!(!super::should_auto_refresh(
            &Screen::Tonight, "2026-01-15", Some(last), now,
            super::SCORES_AUTO_REFRESH_INTERVAL,
        ), "non-empty scores_date (past or future) must not auto-refresh");
    }

    #[test]
    fn l0_scores_auto_refresh_paused_when_timer_unset() {
        // None means dormant — initial fetch happens via maybe_fetch_scores,
        // not via the polling tick.
        let now = std::time::Instant::now();
        assert!(!super::should_auto_refresh(
            &Screen::Tonight, "", None, now,
            super::SCORES_AUTO_REFRESH_INTERVAL,
        ));
    }

    #[test]
    fn l0_scores_auto_refresh_armed_on_t_jump_to_today() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.scores_date = "2026-01-15".to_owned();
        // Past date → timer dormant
        app.last_auto_refresh = None;
        app.handle(Action::Char('t'));
        assert!(app.scores_date.is_empty(), "t must clear the date");
        assert!(app.last_auto_refresh.is_some(), "t back to today must arm the timer");
    }

    #[test]
    fn l0_scores_auto_refresh_disarmed_on_left_right_to_past_date() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.last_auto_refresh = Some(std::time::Instant::now());
        // Move to a specific date — auto-refresh must disengage
        app.handle(Action::Left);
        assert!(!app.scores_date.is_empty(), "Left must set a specific date");
        assert!(app.last_auto_refresh.is_none(),
            "moving to a specific date must disarm the auto-refresh timer");
    }

    #[test]
    fn l0_scores_tick_no_op_when_timer_dormant() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        // Timer never armed — tick must not change anything
        app.tick_auto_refresh();
        assert!(app.last_auto_refresh.is_none(), "tick must leave dormant timer alone");
    }

    #[test]
    fn l0_admin_overlay_capital_f_key_toggles_off() {
        // Pressing F twice should leave the overlay closed.
        let mut app = App::new(false);
        app.handle(Action::Char('F'));
        assert!(app.show_admin);
        // While open, the F-handling branch is short-circuited; press Esc to close.
        app.handle(Action::Escape);
        assert!(!app.show_admin);
    }
}
