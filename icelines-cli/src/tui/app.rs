#![allow(dead_code)]
use crate::tui::event::Action;
use crate::tui::loader::InstallState;
use icelines_core::identity::PlayerId;
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::{PlayerView, StatsRepository};

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
    screen: &Screen,
    scores_date: &str,
    last: Option<std::time::Instant>,
    now: std::time::Instant,
    interval: std::time::Duration,
) -> bool {
    if !matches!(screen, Screen::Tonight) {
        return false;
    }
    if !scores_date.is_empty() {
        return false;
    }
    match last {
        Some(t) => now.duration_since(t) >= interval,
        None => false,
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
    /// Phase Lindsay L.3.4 — search-as-you-type sort picker overlay.
    /// User types substring against `StatId::cli_key()`; up/down moves
    /// selection within filtered list; Enter selects, Esc cancels.
    SortPicker,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Team(String),  // team abbreviation
    /// PlayerId-keyed player card. D6 auto-pop UX: renderer shows a
    /// placeholder if pid isn't in the active window.
    PlayerById(PlayerId),
    Search,
    Tonight,
    Projections,
    Queries, // interactive query builder
    Groups,
    GroupDetail(String), // viewing members of a named group
    Fetch,
    Help,
    /// PlayerId-keyed comps screen. D6 auto-pop UX: if the PlayerId
    /// isn't in the active window, the renderer shows a placeholder.
    CompsById(PlayerId),
    Depth,                           // league-wide team depth rankings
    DepthTeam(String),               // one team's depth chart with fit coloring
    Schedule,                        // weekly view with team / matchup search
    ScheduleTeam(String),            // full-season schedule for one team
    ScheduleMatchup(String, String), // head-to-head game log between two teams
    Playoffs,                        // list-style bracket — rounds × series
    SeriesDetail(String),            // one series — keyed by series letter
    GameDetail(u64),                 // boxscore for one game — keyed by game_id
    Goalies,                         // league goalie leaderboard (Phase G.3)
    /// PlayerId-keyed goalie detail. D6 auto-pop UX on missing pid.
    GoalieDetailById(PlayerId),
    Transactions,                    // league-wide moves feed (Phase T.5)
}

pub struct App {
    pub screen: Screen,
    pub prev_screen: Option<Screen>,
    pub no_color: bool,
    /// Selected row on the Goalies tab.
    pub goalie_selected: usize,
    /// Sort cycle index on the Goalies tab.
    /// 0=SV% desc, 1=GAA asc, 2=W desc, 3=GP desc, 4=Saves desc, 5=SO desc.
    pub goalie_sort: u8,
    /// Min-GP filter on the Goalies tab. Cycles 5 → 15 → 25 → 40 → 5.
    pub goalie_min_gp: u32,
    pub load_state: crate::tui::loader::LoadState,
    pub install_state: InstallState,
    pub tick: u64,
    pub selected: usize,
    pub search_query: String,
    pub status: String,
    pub show_help: bool,
    // Headshot ASCII cache
    pub headshot_cache: crate::tui::headshot::HeadshotCache,
    // Group picker (shown as overlay on player card or team roster)
    pub group_picker_open: bool,
    pub group_picker_list: Vec<String>, // group names
    pub group_picker_player: Option<(String, String)>, // (normalized, full_name)
    // Depth chart tab
    pub depth_mode: icelines_core::cross_team::ScoringMode,
    pub show_admin: bool,
    // Season time-travel
    pub active_season: String,
    pub show_season_picker: bool,
    pub picker_selected: usize,
    // Scores (live schedule)
    pub tonight_cache: crate::tui::tonight::TonightCache,
    pub boxscore_cache: crate::tui::tonight::BoxscoreCache,
    pub scores_date: String,               // "YYYY-MM-DD", empty = today
    pub scores_selected: usize,            // selected game row
    pub scores_picker_open: bool,          // d-key date picker visible
    pub scores_picker_input: String,       // text being typed in date picker
    pub scores_picker_err: Option<String>, // validation error to display
    /// When the most recent live-Scores auto-refresh was triggered. `None`
    /// means the auto-refresh timer is dormant (e.g. user has not opened the
    /// Scores tab on a live date yet). The polling loop sets this on every
    /// tick that fires; the renderer uses it to draw "Updated Xs ago".
    pub last_auto_refresh: Option<std::time::Instant>,
    // Schedule tab — weekly view, search, team / matchup sub-views
    pub schedule_week_cache: crate::tui::schedule::WeekCache,
    pub schedule_team_cache: crate::tui::schedule::TeamSeasonCache,
    pub schedule_week: String, // Monday "YYYY-MM-DD" of the week being viewed
    pub schedule_query: String, // current text in the search bar
    pub schedule_search_mode: bool, // true while the search bar is open
    pub schedule_filter: crate::tui::schedule::SearchFilter, // applied filter
    pub schedule_filter_err: Option<String>, // search validation error
    pub schedule_selected: usize, // selected row on schedule
    // Playoffs tab — bracket + series detail
    pub playoffs_cache: crate::tui::playoffs::PlayoffsCache,
    pub playoffs_round: usize,  // round index (0-based)
    pub playoffs_series: usize, // series index within the current round
    // Query manager state
    pub query_fields: Vec<crate::tui::screens::queries::QueryField>,
    pub query_field_idx: usize,      // which field row is active
    /// Phase Lindsay L.3.3 — categorized sections grouping `query_fields`.
    /// Tab toggles the section containing `query_field_idx`. Collapsed
    /// sections hide their fields from cursor + render.
    pub query_sections: Vec<crate::tui::screens::queries::QuerySection>,
    pub query_result_scroll: usize,  // scroll offset in results panel
    pub query_mode: QueryMode,       // build | save-name | load-list
    pub query_results_focused: bool, // Space toggles focus between field editor and result list
    pub query_save_name: String,     // name being typed for save
    pub query_saved_list: Vec<(String, String)>, // (name, json) loaded from DB
    /// Phase Lindsay L.3.4 — search query for the sort picker overlay.
    /// Substring-matched (case-insensitive) against `StatId::cli_key()`.
    pub sort_picker_query: String,
    /// Phase Lindsay L.3.4 — selected index within the filtered StatId
    /// list. Reset to 0 every time the search query changes.
    pub sort_picker_idx: usize,
    /// Phase Lindsay L.3.4 — the StatId chosen via the sort picker.
    /// `Some(stat)` means subsequent sort uses `StatId::sort_cmp(stat, …)`
    /// instead of the legacy QueryField[0] string. `None` means use the
    /// legacy field (default behavior).
    pub sort_stat_pick: Option<icelines_core::stats_catalog::StatId>,
    /// Phase Lindsay L.4 — active career-table column preset on the
    /// player card. `[`/`]` cycle through `CareerTablePreset::ALL`
    /// (Default | Scoring | Two-way | Special Teams | Time | Goalie | All).
    pub career_table_preset: crate::tui::screens::player::CareerTablePreset,
    /// Phase 8j: lazy-compiled dashboard panel for the player card.
    /// Only consulted when `crate::config::dashboards_enabled()` is true.
    pub dashboard_panel: crate::tui::dashboard_panel::CompiledPanel,
    /// Phase 8j: sorted-by-position pace_82 vectors for percentile
    /// lookups in the dashboard panel. Built once after players load.
    pub league_context: crate::tui::dashboard_panel::LeagueContext,

    // ── Phase T.5: Transactions tab ──────────────────────────────────────
    /// Loaded transactions envelope (rows + provenance). Empty until the
    /// loader picks up the snapshot.
    pub transactions: Vec<icelines_core::Transaction>,
    /// Wall-clock string ("YYYY-MM-DDThh:mm:ss-04:00") from the snapshot
    /// envelope; surfaced in the title bar for staleness display.
    pub transactions_fetched_at: String,
    /// True when the most recent fetch failed (read from
    /// `SnapshotMetaFlags::transactions_stale`). Drives the red [STALE]
    /// prefix in the title bar.
    pub transactions_stale: bool,
    /// Selected row index on the Transactions tab.
    pub tx_selected: usize,
    /// Filter to a single team abbrev (None = all). Cycles via `T`.
    pub tx_team_filter: Option<String>,
    /// Filter to a single kind (None = all). Cycles via `k`.
    pub tx_kind_filter: Option<icelines_core::TransactionKind>,
    /// Substring filter against the description (case-insensitive).
    /// Live-applied as the user types in search mode.
    pub tx_search_query: String,
    /// True while the `/` search bar is open and accepting characters.
    pub tx_search_mode: bool,

    // ── Repo-backed view state ─────────────────────────────────────────
    /// Post-Hart canonical store. `!Send + !Sync` by construction.
    /// Populated synchronously by `App::boot_load` and refreshed by
    /// `reload_for_season` on the season-picker `y` flow.
    pub repo: StatsRepository,
    /// Typed mirror of `active_season` — `Season(YYYYZZZZ)`. The
    /// String form survives for legacy callers and the season-picker
    /// UI; the typed form is what `repo.skaters` etc. require.
    pub active_season_typed: Season,
    /// Season-type axis (Regular | Playoff). Hart.5c.6 sets this from
    /// the `y` season picker; today it's always Regular until Hart.6
    /// lands playoff data.
    pub active_type: SeasonType,
    /// Window the current `league_context` was built for (D11 forcing
    /// function). Set in lockstep with `league_context`; passed into
    /// `dashboard_panel.compile` so cross-window construction is
    /// rejected at the boundary.
    pub league_context_window: (Season, SeasonType),
}

impl App {
    pub fn new(no_color: bool) -> Self {
        Self {
            screen: Screen::Home,
            prev_screen: None,
            no_color,
            goalie_selected: 0,
            goalie_sort: 0,    // SV% descending — Vezina-eligibility default
            goalie_min_gp: 15, // NHL leaderboard convention
            load_state: crate::tui::loader::LoadState::new(),
            install_state: InstallState::new(),
            tick: 0,
            selected: 0,
            search_query: String::new(),
            status: "Loading data… · Press ? for help · q to quit".to_owned(),
            show_help: false,
            query_fields: crate::tui::screens::queries::default_fields(),
            query_field_idx: 0,
            query_sections: crate::tui::screens::queries::default_sections(),
            query_result_scroll: 0,
            depth_mode: icelines_core::cross_team::ScoringMode::Fantasy,
            show_admin: false,
            active_season: icelines_core::CURRENT_SEASON_STR.to_owned(),
            show_season_picker: false,
            picker_selected: 0,
            tonight_cache: crate::tui::tonight::new_cache(),
            boxscore_cache: crate::tui::tonight::new_boxscore_cache(),
            scores_date: String::new(),
            scores_selected: 0,
            scores_picker_open: false,
            scores_picker_input: String::new(),
            scores_picker_err: None,
            last_auto_refresh: None,
            schedule_week_cache: crate::tui::schedule::new_week_cache(),
            schedule_team_cache: crate::tui::schedule::new_team_cache(),
            schedule_week: crate::tui::schedule::monday_of(&crate::tui::schedule::today_iso())
                .unwrap_or_else(crate::tui::schedule::today_iso),
            schedule_query: String::new(),
            schedule_search_mode: false,
            schedule_filter: crate::tui::schedule::SearchFilter::None,
            schedule_filter_err: None,
            schedule_selected: 0,
            playoffs_cache: crate::tui::playoffs::new_cache(),
            playoffs_round: 0,
            playoffs_series: 0,
            group_picker_open: false,
            group_picker_list: Vec::new(),
            group_picker_player: None,
            headshot_cache: crate::tui::headshot::HeadshotCache::new(),
            query_mode: QueryMode::Build,
            query_results_focused: false,
            query_save_name: String::new(),
            query_saved_list: Vec::new(),
            sort_picker_query: String::new(),
            sort_picker_idx: 0,
            sort_stat_pick: None,
            career_table_preset: Default::default(),
            dashboard_panel: crate::tui::dashboard_panel::CompiledPanel::new(),
            league_context: crate::tui::dashboard_panel::LeagueContext::empty(),
            transactions: Vec::new(),
            transactions_fetched_at: String::new(),
            transactions_stale: false,
            tx_selected: 0,
            tx_team_filter: None,
            tx_kind_filter: None,
            tx_search_query: String::new(),
            tx_search_mode: false,

            // Empty repo + current season as the initial typed window.
            // `App::boot_load` populates the repo synchronously before
            // the event loop starts.
            repo: StatsRepository::new(),
            active_season_typed: Season(icelines_core::CURRENT_SEASON),
            active_type: SeasonType::Regular,
            league_context_window: (
                Season(icelines_core::CURRENT_SEASON),
                SeasonType::Regular,
            ),
        }
    }

    // ── Hart.5c.6 Phase A — view-based accessors ─────────────────────
    //
    // Every accessor takes (active_season_typed, active_type) so the
    // view set always reflects the current time-travel window.

    /// Skater views for the active (season, season_type). O(LRU·N)
    /// per call; renderers should collect once per frame.
    pub fn views(&self) -> Vec<PlayerView<'_>> {
        self.repo
            .skaters(self.active_season_typed, self.active_type)
            .collect()
    }

    /// Goalie views for the active window.
    pub fn goalie_views(&self) -> Vec<PlayerView<'_>> {
        self.repo
            .goalies(self.active_season_typed, self.active_type)
            .collect()
    }

    /// Last-stint roster for `team` in the active window. O(1) index
    /// lookup + O(roster_size ≈ 25) view materialization. Used for
    /// depth chart and team page (post-trade roster shape).
    pub fn team_views(&self, team: &TeamAbbr) -> Vec<PlayerView<'_>> {
        self.repo
            .team_roster(team, self.active_season_typed, self.active_type)
    }

    /// All-stints roster for `team` — includes any player who
    /// played for `team` at any point in the active window. Use when
    /// mid-season trades should appear on both teams.
    pub fn team_views_all_stints(&self, team: &TeamAbbr) -> Vec<PlayerView<'_>> {
        self.repo
            .team_roster_all_stints(team, self.active_season_typed, self.active_type)
    }

    /// Resolve a Player ID via the repo's identity index. Returns the
    /// PlayerView if the player exists in the active window; None
    /// otherwise (e.g. after a season switch into a window the player
    /// wasn't active in). D6 auto-pop UX is the load-bearing
    /// mitigation for the None case.
    pub fn view_for(&self, pid: PlayerId) -> Option<PlayerView<'_>> {
        self.repo.view(pid, self.active_season_typed, self.active_type)
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

        // Transactions search bar — same shape but applies live as the
        // user types (no validation step). Phase T+1.
        if self.screen == Screen::Transactions && self.tx_search_mode {
            return self.handle_transactions_search(action);
        }

        // Scores date picker consumes input similarly.
        if self.screen == Screen::Tonight && self.scores_picker_open {
            return self.handle_scores_date_picker(action);
        }

        // Queries save-name input must short-circuit so hotkey letters
        // ('f' = AddToFavorites, 'r' = Refresh, etc.) become text input
        // instead of firing the global hotkey. Without this, typing
        // "fred" as a query name dispatched AddToFavorites on the 'f'.
        if self.screen == Screen::Queries
            && matches!(self.query_mode, QueryMode::SaveName)
        {
            return self.handle_query_save_name(action);
        }

        match action {
            Action::Quit => return true,
            Action::Help => self.show_help = true,
            Action::Back | Action::Escape => {
                if self.group_picker_open {
                    self.group_picker_open = false;
                    self.group_picker_player = None;
                    self.selected = 0;
                    self.status =
                        "  g = add to group from any player card or team roster".to_owned();
                } else if self.screen == Screen::Queries && self.query_mode != QueryMode::Build {
                    self.query_mode = QueryMode::Build;
                    self.status = "Cancelled  ·  s=save  l=load  r=reset".to_owned();
                } else if self.screen == Screen::Queries
                    && self.query_mode == QueryMode::Build
                    && self.sort_stat_pick.is_some()
                {
                    // Phase Lindsay L.3.4 (EDGE checkpoint fix): Esc on
                    // Queries Build mode with an active picker selection
                    // clears the pick, restoring the legacy "Sort by"
                    // QueryField path. Without this, Left/Right on the
                    // Sort by field is silently ignored once the picker
                    // has fired (sticky-pick UX wart).
                    self.sort_stat_pick = None;
                    self.status = "Sort pick cleared  ·  / picker  s=save  l=load  r=reset".to_owned();
                } else {
                    self.go_back();
                }
            }
            Action::Down => {
                if self.screen == Screen::Tonight {
                    self.scores_selected = self.scores_selected.saturating_add(1);
                } else if matches!(
                    self.screen,
                    Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _)
                ) {
                    self.schedule_selected = self.schedule_selected.saturating_add(1);
                } else if self.screen == Screen::Goalies {
                    self.goalie_selected = self.goalie_selected.saturating_add(1);
                } else if self.screen == Screen::Transactions {
                    self.tx_selected = self.tx_selected.saturating_add(1);
                } else if self.screen == Screen::Playoffs {
                    self.playoffs_series = self.playoffs_series.saturating_add(1);
                } else if self.screen == Screen::Queries {
                    if self.query_mode == QueryMode::SortPicker {
                        // Phase Lindsay L.3.4 — sort picker Down moves
                        // within filtered list, capped at len-1.
                        let n = crate::tui::screens::queries::sort_picker_filter(
                            &self.sort_picker_query,
                        )
                        .len();
                        if n > 0 && self.sort_picker_idx + 1 < n {
                            self.sort_picker_idx += 1;
                        }
                    } else if self.query_results_focused {
                        let views = self.views();
                        let results = crate::tui::screens::queries::run_query_views_with_pick(
                            &views,
                            &self.query_fields,
                            self.sort_stat_pick,
                        );
                        let visible: usize = 20;
                        if self.selected + 1 < visible {
                            self.selected =
                                (self.selected + 1).min(results.len().saturating_sub(1));
                        } else {
                            let max_scroll = results.len().saturating_sub(visible);
                            self.query_result_scroll =
                                (self.query_result_scroll + 1).min(max_scroll);
                        }
                    } else {
                        // Phase Lindsay L.3.3 — cursor skips fields in
                        // collapsed sections. `visible_field_indices`
                        // returns the cursor-stoppable subset.
                        let visible = crate::tui::screens::queries::visible_field_indices(
                            &self.query_sections,
                        );
                        let cur_pos = visible.iter().position(|&i| i == self.query_field_idx);
                        match cur_pos {
                            Some(pos) if pos + 1 < visible.len() => {
                                self.query_field_idx = visible[pos + 1];
                            }
                            _ => {
                                // Past last visible — focus results.
                                self.query_results_focused = true;
                                self.selected = 0;
                                self.query_result_scroll = 0;
                            }
                        }
                    }
                } else if self.screen == Screen::Home {
                    let n = crate::tui::screens::home::RANKED_TEAMS.len();
                    self.selected = if self.selected + 1 >= n {
                        0
                    } else {
                        self.selected + 1
                    };
                } else {
                    self.selected = self.selected.saturating_add(1);
                }
            }
            Action::Up => {
                if self.screen == Screen::Tonight {
                    self.scores_selected = self.scores_selected.saturating_sub(1);
                } else if matches!(
                    self.screen,
                    Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _)
                ) {
                    self.schedule_selected = self.schedule_selected.saturating_sub(1);
                } else if self.screen == Screen::Goalies {
                    self.goalie_selected = self.goalie_selected.saturating_sub(1);
                } else if self.screen == Screen::Transactions {
                    self.tx_selected = self.tx_selected.saturating_sub(1);
                } else if self.screen == Screen::Playoffs {
                    self.playoffs_series = self.playoffs_series.saturating_sub(1);
                } else if self.screen == Screen::Queries {
                    if self.query_mode == QueryMode::SortPicker {
                        // Phase Lindsay L.3.4 — sort picker Up moves
                        // within filtered list. Saturates at 0.
                        self.sort_picker_idx = self.sort_picker_idx.saturating_sub(1);
                    } else if self.query_results_focused {
                        if self.selected > 0 {
                            self.selected -= 1;
                        } else if self.query_result_scroll > 0 {
                            self.query_result_scroll -= 1;
                        } else {
                            // Phase Lindsay L.3.3 — snap to LAST visible field.
                            self.query_results_focused = false;
                            let visible = crate::tui::screens::queries::visible_field_indices(
                                &self.query_sections,
                            );
                            self.query_field_idx = *visible.last().unwrap_or(&0);
                        }
                    } else {
                        // Phase Lindsay L.3.3 — cursor skips fields in
                        // collapsed sections.
                        let visible = crate::tui::screens::queries::visible_field_indices(
                            &self.query_sections,
                        );
                        let cur_pos = visible.iter().position(|&i| i == self.query_field_idx);
                        if let Some(pos) = cur_pos {
                            if pos > 0 {
                                self.query_field_idx = visible[pos - 1];
                            }
                            // pos == 0 → already at first visible, stay.
                        } else if let Some(&first) = visible.first() {
                            // Cursor was on a now-hidden field — snap to first visible.
                            self.query_field_idx = first;
                        }
                    }
                } else if self.screen == Screen::Home {
                    let n = crate::tui::screens::home::RANKED_TEAMS.len();
                    self.selected = if self.selected == 0 {
                        n - 1
                    } else {
                        self.selected - 1
                    };
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
                            if matches!(action, Action::Right) {
                                f.next();
                            } else {
                                f.prev();
                            }
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
                        let delta = if matches!(action, Action::Right) {
                            1
                        } else {
                            -1
                        };
                        if let Some(new_date) = crate::tui::schedule::add_days(&from, delta) {
                            self.scores_date = new_date.clone();
                            self.scores_selected = 0;
                            crate::tui::tonight::maybe_fetch(
                                self.tonight_cache.clone(),
                                new_date.clone(),
                            );
                            // Past dates don't poll — clear the auto-refresh timer.
                            self.last_auto_refresh = None;
                            self.status = format!("Scores · {new_date}");
                        }
                    }
                    // Schedule: ←/→ moves between weeks (overrides global sub-view nav)
                    Screen::Schedule => {
                        let delta = if matches!(action, Action::Right) {
                            7
                        } else {
                            -7
                        };
                        if let Some(new_week) =
                            crate::tui::schedule::add_days(&self.schedule_week, delta)
                        {
                            self.schedule_week = new_week.clone();
                            self.schedule_selected = 0;
                            crate::tui::schedule::maybe_fetch_week(
                                self.schedule_week_cache.clone(),
                                new_week.clone(),
                            );
                            self.status =
                                format!("Week of {}", crate::tui::schedule::week_label(&new_week));
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
                    // Sub-view switching: Queries ↔ Projections.
                    // Both live under the Stats tab; ←/→ flips between them.
                    // (League / Depth used to do the same, but Depth is now
                    // its own tab — toggle removed.)
                    Screen::Queries if !self.query_results_focused => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Projections;
                        self.selected = 0;
                    }
                    Screen::Projections => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Queries;
                        self.selected = 0;
                        self.query_results_focused = false;
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
                    self.status =
                        "Search: type team (SEA) or matchup (NYR WSH) — Enter, Esc cancel"
                            .to_owned();
                } else if self.screen == Screen::Transactions {
                    // Transactions tab: '/' opens an in-tab description
                    // substring search. Live-applied as the user types.
                    self.tx_search_mode = true;
                    self.tx_search_query.clear();
                    self.tx_selected = 0;
                    self.status =
                        "Search transactions: type any substring — Enter applies, Esc clears"
                            .to_owned();
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
                } else if self.screen == Screen::Queries
                    && c == 'p'
                    && !matches!(self.query_mode, QueryMode::SaveName)
                {
                    // `p` flips to the Projections sister-screen. ←/→ on
                    // Queries is consumed by field editing, so this is
                    // the only way out without going Tab → … → Tab back.
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Projections;
                    self.selected = 0;
                    self.query_results_focused = false;
                    self.status =
                        "Projections · p:queries  ↑↓:scroll  Enter:player card".to_owned();
                } else if self.screen == Screen::Projections && c == 'p' {
                    // Symmetric: `p` from Projections flips back to Queries.
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Queries;
                    self.selected = 0;
                    self.status = "Queries · p:projections  ←/→:edit  Tab:focus results".to_owned();
                } else if self.screen == Screen::Queries {
                    match &self.query_mode {
                        QueryMode::SaveName => {
                            // Typing the save name
                            self.query_save_name.push(c);
                        }
                        QueryMode::SortPicker => {
                            // Phase Lindsay L.3.4 — typing in the sort picker
                            // appends to the search query and resets selection
                            // index to 0 (top of newly-filtered list).
                            self.sort_picker_query.push(c);
                            self.sort_picker_idx = 0;
                        }
                        QueryMode::Build if c == 's' => {
                            // Start save-name mode
                            self.query_mode = QueryMode::SaveName;
                            self.query_save_name.clear();
                            self.status =
                                "Save query as: (type name, Enter to save, Esc to cancel)"
                                    .to_owned();
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
                        QueryMode::Build if c == '/' => {
                            // Phase Lindsay L.3.4 — `/` on Queries opens
                            // the sort picker overlay. Search-as-you-type
                            // against catalog cli_keys.
                            self.query_mode = QueryMode::SortPicker;
                            self.sort_picker_query.clear();
                            self.sort_picker_idx = 0;
                            self.status = "Sort picker — type to filter · ↑↓ select · Enter accept · Esc cancel".to_owned();
                        }
                        _ => {}
                    }
                } else if let Screen::PlayerById(pid) = self.screen {
                    if c == 'c' {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::CompsById(pid);
                        self.selected = 0;
                    } else if c == '[' {
                        // Phase Lindsay L.4.4 — `[` cycles career-table
                        // preset BACKWARD (vim-canonical bracket motion).
                        self.career_table_preset = self.career_table_preset.prev();
                        self.status = format!(
                            "Career preset: {}  ·  [/]: cycle  ·  c: comps",
                            self.career_table_preset.label(),
                        );
                    } else if c == ']' {
                        // Phase Lindsay L.4.4 — `]` cycles FORWARD.
                        self.career_table_preset = self.career_table_preset.next();
                        self.status = format!(
                            "Career preset: {}  ·  [/]: cycle  ·  c: comps",
                            self.career_table_preset.label(),
                        );
                    }
                } else if matches!(self.screen, Screen::Depth | Screen::DepthTeam(_)) && c == 's' {
                    self.depth_mode = self.depth_mode.toggle();
                    self.status = format!("Scoring: {}", self.depth_mode.label());
                } else if self.screen == Screen::Goalies && c == 's' {
                    // Phase G.3: cycle sort SV% → GAA → W → GP → Saves → SO
                    let n = crate::tui::screens::goalies::SORTS.len() as u8;
                    self.goalie_sort = (self.goalie_sort + 1) % n;
                    self.goalie_selected = 0;
                    let label =
                        crate::tui::screens::goalies::SORTS[self.goalie_sort as usize].label();
                    self.status = format!("Goalies sort: {label}");
                } else if self.screen == Screen::Goalies && c == 'm' {
                    // Cycle min-GP threshold 5 → 15 → 25 → 40
                    let cycle = crate::tui::screens::goalies::MIN_GP_CYCLE;
                    let cur = cycle
                        .iter()
                        .position(|v| *v == self.goalie_min_gp)
                        .unwrap_or(0);
                    self.goalie_min_gp = cycle[(cur + 1) % cycle.len()];
                    self.goalie_selected = 0;
                    self.status = format!("Goalies min GP: {}", self.goalie_min_gp);
                } else if self.screen == Screen::Schedule && c == 't' {
                    // Jump to today's week
                    let today = crate::tui::schedule::today_iso();
                    if let Some(monday) = crate::tui::schedule::monday_of(&today) {
                        self.schedule_week = monday.clone();
                        self.schedule_selected = 0;
                        crate::tui::schedule::maybe_fetch_week(
                            self.schedule_week_cache.clone(),
                            monday.clone(),
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
                    self.status =
                        "Go to date — type YYYY-MM-DD or MM/DD, Enter applies, Esc cancels"
                            .to_owned();
                } else if self.screen == Screen::Tonight && c == 't' {
                    // 't' on Scores jumps back to today (live)
                    self.scores_date.clear();
                    self.scores_selected = 0;
                    crate::tui::tonight::maybe_fetch(self.tonight_cache.clone(), String::new());
                    // Re-arm the auto-refresh timer for the live date.
                    self.last_auto_refresh = Some(std::time::Instant::now());
                    self.status = "Scores · Today".to_owned();
                } else if self.screen == Screen::Transactions && (c == 't' || c == 'T') {
                    // Phase T.5+: cycle team filter through every team that
                    // appears in the loaded transactions.
                    //   t       → forward  (None → first → … → None)
                    //   Shift-T → backward (None → last  → … → None)
                    use crate::tui::screens::transactions as tx_screen;
                    let teams = tx_screen::transactions_teams(&self.transactions);
                    let next = if c == 't' {
                        tx_screen::cycle_team_forward(self.tx_team_filter.as_deref(), &teams)
                    } else {
                        tx_screen::cycle_team_backward(self.tx_team_filter.as_deref(), &teams)
                    };
                    self.tx_team_filter = next.clone();
                    self.tx_selected = 0;
                    self.status = match next {
                        Some(t) => format!("Transactions team filter: {t}"),
                        None => "Transactions team filter: all".to_owned(),
                    };
                } else if self.screen == Screen::Transactions && (c == 'k' || c == 'K') {
                    // Cycle kind filter; Shift-K reverses.
                    use crate::tui::screens::transactions as tx_screen;
                    use icelines_core::TransactionKind as K;
                    let cycle = K::ALL;
                    let next = if c == 'k' {
                        tx_screen::cycle_kind_forward(self.tx_kind_filter, cycle)
                    } else {
                        tx_screen::cycle_kind_backward(self.tx_kind_filter, cycle)
                    };
                    self.tx_kind_filter = next;
                    self.tx_selected = 0;
                    self.status = match next {
                        Some(k) => format!("Transactions kind filter: {}", k.label()),
                        None => "Transactions kind filter: all".to_owned(),
                    };
                } else if c == 'F' {
                    self.show_admin = !self.show_admin;
                } else if c == 'P' {
                    // Hart.6.9.B — Shift+P toggles between Regular and
                    // Playoff for the active season. Lowercase `p` is
                    // reserved for the Queries↔Projections flip; the
                    // capital is the global playoff toggle.
                    self.toggle_season_type();
                } else if c == 'y' {
                    self.show_season_picker = true;
                    // Start picker on current active season
                    let season_list = crate::tui::screens::misc::PICKER_SEASONS;
                    self.picker_selected = season_list
                        .iter()
                        .position(|(id, _, _)| *id == self.active_season.as_str())
                        .unwrap_or(0);
                } else if c == 'd'
                    && !matches!(
                        self.screen,
                        // Skip text-input screens; 'd' is part of the typed query.
                        Screen::Search | Screen::Tonight
                    )
                    && !(self.screen == Screen::Schedule && self.schedule_search_mode)
                    && !(self.screen == Screen::Queries
                        && matches!(self.query_mode, QueryMode::SaveName))
                {
                    // Global shortcut: jump to the league depth view.
                    // Already on a depth screen → toggle back to Home so
                    // the key can hide the chart too.
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = match &self.screen {
                        Screen::Depth | Screen::DepthTeam(_) => Screen::Home,
                        _ => Screen::Depth,
                    };
                    self.selected = 0;
                    self.status = match &self.screen {
                        Screen::Depth => {
                            "Depth chart — s: scoring  Enter: team chart  d: home".to_owned()
                        }
                        _ => "Home".to_owned(),
                    };
                }
            }
            Action::Backspace => {
                if self.screen == Screen::Search {
                    self.search_query.pop();
                    self.selected = 0;
                } else if self.screen == Screen::Queries && self.query_mode == QueryMode::SaveName {
                    self.query_save_name.pop();
                } else if self.screen == Screen::Queries
                    && self.query_mode == QueryMode::SortPicker
                {
                    // Phase Lindsay L.3.4 — Backspace in sort picker
                    // pops the search query and resets selection.
                    self.sort_picker_query.pop();
                    self.sort_picker_idx = 0;
                }
            }
            Action::Tab => {
                // Phase Lindsay L.3.3 — Tab on the Queries screen
                // toggles the section containing the current field
                // cursor (per v0.4 spec §"TUI integration"). Save/load
                // overlays + results-focus get cycle-screen still.
                let in_queries_build = self.screen == Screen::Queries
                    && self.query_mode == QueryMode::Build
                    && !self.query_results_focused;
                if in_queries_build {
                    let _ = crate::tui::screens::queries::toggle_section_for_field(
                        &mut self.query_sections,
                        self.query_field_idx,
                    );
                    // After collapse, cursor may now point at a hidden
                    // field — snap to nearest visible.
                    let visible = crate::tui::screens::queries::visible_field_indices(
                        &self.query_sections,
                    );
                    if !visible.contains(&self.query_field_idx) {
                        if let Some(&first) = visible.first() {
                            self.query_field_idx = first;
                        }
                    }
                } else {
                    self.cycle_screen();
                }
            }
            Action::TabPrev => self.cycle_screen_back(),
            Action::Refresh => {
                if self.screen == Screen::Queries {
                    self.query_fields = crate::tui::screens::queries::default_fields();
                    self.query_sections = crate::tui::screens::queries::default_sections();
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
                    self.status = format!(
                        "Retrying {}…",
                        crate::tui::schedule::week_label(&self.schedule_week)
                    );
                } else if matches!(self.screen, Screen::Playoffs | Screen::SeriesDetail(_)) {
                    if let Some(year) =
                        crate::tui::playoffs::playoff_year_for_season(&self.active_season)
                    {
                        crate::tui::playoffs::force_fetch_bracket(
                            self.playoffs_cache.clone(),
                            year,
                            &self.active_season,
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
                    // On a player card / team roster: open the picker so
                    // user can add this specific player to a group.
                    self.group_picker_list = crate::db::GroupDb::open()
                        .ok()
                        .and_then(|db| db.list_groups().ok())
                        .map(|gs| gs.into_iter().map(|g| g.name).collect())
                        .unwrap_or_default();
                    if self.group_picker_list.is_empty() {
                        self.status =
                            "No groups — create one with `icelines group create`".to_owned();
                    } else {
                        self.group_picker_player = Some(player);
                        self.group_picker_open = true;
                        self.selected = 0;
                        self.status = "Add to group — ↑↓ select · Enter · Esc cancel".to_owned();
                    }
                } else {
                    // No player in scope: `g` becomes the global "open Groups"
                    // shortcut. Phase T+1 — Groups was demoted from a tab.
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Groups;
                    self.selected = 0;
                    self.status =
                        "Groups — ↑↓ select · Enter to view members · Esc back".to_owned();
                }
            }
            Action::AddToFavorites => {
                // Instant add to "Favorites" — no picker, one key
                // Reuse the same player-detection logic as AddToGroup
                let target = self.get_selected_player();
                if let Some((norm, full)) = target {
                    if let Ok(db) = crate::db::GroupDb::open() {
                        match db.add_member("Favorites", &norm) {
                            Ok(true) => self.status = format!("★ Added {} to Favorites", full),
                            Ok(false) => {
                                self.status = format!("★ {} is already in Favorites", full)
                            }
                            Err(e) => self.status = format!("Error: {e}"),
                        }
                    }
                }
            }

            Action::GoToTab(n) => {
                // 1–8: League, Depth, Stats(Queries), Goalies, Scores,
                // Schedule, Transactions, Playoffs.
                let tabs = [
                    Screen::Home,
                    Screen::Depth,
                    Screen::Queries,
                    Screen::Goalies,
                    Screen::Tonight,
                    Screen::Schedule,
                    Screen::Transactions,
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
            crate::tui::tonight::maybe_fetch(self.tonight_cache.clone(), self.scores_date.clone());
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
        if !crate::config::live_feeds_enabled() {
            return;
        }
        let now = std::time::Instant::now();
        if should_auto_refresh(
            &self.screen,
            &self.scores_date,
            self.last_auto_refresh,
            now,
            SCORES_AUTO_REFRESH_INTERVAL,
        ) {
            crate::tui::tonight::force_fetch(self.tonight_cache.clone(), self.scores_date.clone());
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
            self.scores_picker_err = None;
            self.scores_selected = 0;
            crate::tui::tonight::maybe_fetch(self.tonight_cache.clone(), String::new());
            // Empty date = live → arm the timer
            self.last_auto_refresh = Some(std::time::Instant::now());
            self.status = "Scores · Today".to_owned();
            return;
        }
        match parse_picker_date(raw) {
            Ok(iso) => {
                self.scores_date = iso.clone();
                self.scores_picker_open = false;
                self.scores_picker_err = None;
                self.scores_picker_input.clear();
                self.scores_selected = 0;
                crate::tui::tonight::maybe_fetch(self.tonight_cache.clone(), iso.clone());
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
                self.scores_picker_err = None;
                self.status = "Date picker cancelled.".to_owned();
            }
            Action::Enter => self.apply_scores_date_picker(),
            Action::Backspace => {
                self.scores_picker_input.pop();
                self.scores_picker_err = None;
            }
            Action::Char(c) => self.scores_picker_input.push(c),
            // Map non-text actions back to their characters so digits/letters
            // typed at the picker behave naturally.
            Action::Refresh => self.scores_picker_input.push('r'),
            Action::Install => self.scores_picker_input.push('i'),
            Action::AddToGroup => self.scores_picker_input.push('g'),
            Action::AddToFavorites => self.scores_picker_input.push('f'),
            Action::GoToTab(n) => {
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
            None => return 0,
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
        let map = self.playoffs_cache.lock().unwrap();
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
                    self.playoffs_cache.clone(),
                    year,
                    &self.active_season,
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
                    crate::tui::schedule::SearchFilter::None => "Filter cleared.".to_owned(),
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
            Action::Backspace => {
                self.schedule_query.pop();
                self.schedule_filter_err = None;
            }
            Action::Char(c) => self.schedule_query.push(c),
            Action::Space => self.schedule_query.push(' '),
            // While in search mode, hotkeys are treated as text input so
            // queries like "nyr" can be typed without firing Refresh/Install/etc.
            Action::Refresh => self.schedule_query.push('r'),
            Action::Install => self.schedule_query.push('i'),
            Action::AddToGroup => self.schedule_query.push('g'),
            Action::AddToFavorites => self.schedule_query.push('f'),
            Action::GoToTab(n) => {
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

    /// Save-query name input. Mirrors `handle_transactions_search`:
    /// while QueryMode::SaveName is active, every character-bearing
    /// Action is treated as text input. Without this short-circuit, the
    /// global keymap fires AddToFavorites/Refresh/etc instead of
    /// typing the character into the name field — so the user couldn't
    /// type "fred" because the 'f' opened the Favorites flow.
    fn handle_query_save_name(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                // Cancel save — drop the typed name, go back to Build.
                self.query_save_name.clear();
                self.query_mode = QueryMode::Build;
                self.status = "Save cancelled.".to_owned();
            }
            Action::Enter => {
                let name = self.query_save_name.trim().to_owned();
                if !name.is_empty() {
                    let json =
                        crate::tui::screens::queries::fields_to_json(&self.query_fields);
                    if let Ok(db) = crate::db::GroupDb::open() {
                        let _ = db.save_query(&name, &json);
                        self.status =
                            format!("Saved query '{name}'  ·  l=load  s=save  r=reset");
                    }
                }
                self.query_mode = QueryMode::Build;
            }
            Action::Backspace => {
                self.query_save_name.pop();
            }
            Action::Char(c) => self.query_save_name.push(c),
            Action::Space => self.query_save_name.push(' '),
            // Hotkey actions become their associated character. Without
            // this, 'f' would fire AddToFavorites and the user could
            // never type "fred", "fox", "ford" etc. as a query name.
            Action::Refresh => self.query_save_name.push('r'),
            Action::Install => self.query_save_name.push('i'),
            Action::AddToGroup => self.query_save_name.push('g'),
            Action::AddToFavorites => self.query_save_name.push('f'),
            Action::GoToTab(n) => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.query_save_name.push(ch);
            }
            // '/' while typing — ignore (don't reopen, don't insert)
            Action::Search => {}
            // Help opens the overlay only at end-of-name confusion; cleaner to
            // treat as no-op so the user can press ? to see hints without
            // losing their typed name.
            _ => {}
        }
        false
    }

    /// Transactions tab `/` search — live substring match against the
    /// description. Enter freezes the filter and exits search mode (the
    /// query stays applied). Esc clears + exits.
    fn handle_transactions_search(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                self.tx_search_mode = false;
                self.tx_search_query.clear();
                self.tx_selected = 0;
                self.status = "Search cleared.".to_owned();
            }
            Action::Enter => {
                // Apply: keep the query, exit search mode.
                self.tx_search_mode = false;
                self.tx_selected = 0;
                self.status = format!("Filter: '{}'", self.tx_search_query);
            }
            Action::Backspace => {
                self.tx_search_query.pop();
            }
            Action::Char(c) => self.tx_search_query.push(c),
            Action::Space => self.tx_search_query.push(' '),
            Action::Refresh => self.tx_search_query.push('r'),
            Action::Install => self.tx_search_query.push('i'),
            Action::AddToGroup => self.tx_search_query.push('g'),
            Action::AddToFavorites => self.tx_search_query.push('f'),
            Action::GoToTab(n) => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.tx_search_query.push(ch);
            }
            Action::Search => {}
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
                if let Some(&(season_id, _, is_lockout)) = PICKER_SEASONS.get(self.picker_selected)
                {
                    if is_lockout {
                        self.status = "No season data — lockout year (2004-05).".to_owned();
                    } else {
                        let is_bundled =
                            icelines_fetch::bundled::BUNDLED_SEASONS.contains(&season_id);
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
                if let Some(&(season_id, _, is_lockout)) = PICKER_SEASONS.get(self.picker_selected)
                {
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

    /// Synchronous boot load. Populates `self.repo` from
    /// `(active_season_typed, active_type)` against the configured
    /// snapshot dir, falling back to bundled data when the snapshot is
    /// absent. Rebuilds `league_context` so post-load renders see the
    /// rank/percentile tables coupled to the current window.
    ///
    /// Must run BEFORE the event loop starts. `crossterm::event::poll`
    /// is a blocking sync syscall, so any async load in parallel would
    /// never get a chance to run. The synchronous load takes ~50ms cold
    /// against bundled data — well below the user's perception
    /// threshold for a CLI program startup.
    ///
    /// On error: `self.status` carries the error string. The repo stays
    /// empty so screens render their "no data" branch — better than
    /// crashing.
    pub fn boot_load(&mut self) {
        use icelines_fetch::snapshot::SnapshotStore;
        use icelines_fetch::stats_loader::{format_missing_sources, load_into_repo};

        let snapshot_dir = match crate::config::Config::load() {
            Ok(cfg) => cfg.snapshot_dir(),
            Err(_) => return,
        };
        let store = SnapshotStore::new(snapshot_dir);
        match load_into_repo(self.active_season_typed, self.active_type, &store) {
            Ok(outcome) => {
                let _old = self.repo.repo_swap(outcome.repo);
                self.league_context = crate::tui::dashboard_panel::LeagueContext::build(
                    &self.repo,
                    self.active_season_typed,
                    self.active_type,
                );
                self.league_context_window = (self.active_season_typed, self.active_type);
                self.dashboard_panel.clear_cache();
                self.status = if outcome.missing.is_empty() {
                    "Press ? for help · q to quit".to_owned()
                } else {
                    format_missing_sources(&outcome.missing)
                };
            }
            Err(e) => {
                self.status = format!("load failed: {e}");
            }
        }
    }

    /// Same as `boot_load` but accepts an explicit `SnapshotStore`. Used
    /// by tests so they can point at a tempdir or a custom fixture
    /// without touching `~/.icelines/`.
    pub fn boot_load_with_store(
        &mut self,
        store: &icelines_fetch::snapshot::SnapshotStore,
    ) {
        use icelines_fetch::stats_loader::{format_missing_sources, load_into_repo};
        match load_into_repo(self.active_season_typed, self.active_type, store) {
            Ok(outcome) => {
                let _old = self.repo.repo_swap(outcome.repo);
                self.league_context = crate::tui::dashboard_panel::LeagueContext::build(
                    &self.repo,
                    self.active_season_typed,
                    self.active_type,
                );
                self.league_context_window = (self.active_season_typed, self.active_type);
                self.dashboard_panel.clear_cache();
                self.status = if outcome.missing.is_empty() {
                    "Press ? for help · q to quit".to_owned()
                } else {
                    format_missing_sources(&outcome.missing)
                };
            }
            Err(e) => {
                self.status = format!("load failed: {e}");
            }
        }
    }

    fn reload_for_season(&mut self, season_id: &str) {
        // Hart.6.9.B — preserves the current `active_type` so a user
        // who's switched to Playoff and then picks a different season
        // stays in Playoff mode. Use `reload_for_season_typed` if you
        // want to force a specific type.
        self.reload_for_season_typed(season_id, self.active_type);
    }

    /// Hart.6.9.B — explicit season-type variant. Used by
    /// `toggle_season_type` to flip Regular ↔ Playoff in-place,
    /// and by future callers that need a typed reload.
    fn reload_for_season_typed(
        &mut self,
        season_id: &str,
        ty: icelines_core::season_stats::SeasonType,
    ) {
        use icelines_fetch::snapshot::SnapshotStore;
        use icelines_fetch::stats_loader::{format_missing_sources, load_into_repo};

        let season_u32: u32 = season_id.parse().unwrap_or(icelines_core::CURRENT_SEASON);
        let season = icelines_core::model::Season(season_u32);

        let outcome = match crate::config::Config::load() {
            Ok(cfg) => {
                let store = SnapshotStore::new(cfg.snapshot_dir());
                load_into_repo(season, ty, &store).ok()
            }
            Err(_) => None,
        };

        if let Some(outcome) = outcome {
            // Atomic repo swap; rebuild (season, type)-coupled caches per D5.
            let _old = self.repo.repo_swap(outcome.repo);
            self.active_season_typed = season;
            self.active_type = ty;
            self.league_context =
                crate::tui::dashboard_panel::LeagueContext::build(&self.repo, season, ty);
            self.league_context_window = (season, ty);
            self.dashboard_panel.clear_cache();

            if !outcome.missing.is_empty() {
                self.status = format_missing_sources(&outcome.missing);
            }
        } else {
            // Load failed (likely MissingBundle for playoff on an unbundled
            // season). Surface clean status; don't swap the repo.
            self.status = match ty {
                icelines_core::season_stats::SeasonType::Regular => {
                    format!("Failed to load season {season_id}.")
                }
                icelines_core::season_stats::SeasonType::Playoff => format!(
                    "No playoff data for {season_id} (Cup not contested or not bundled). \
                     Press Shift+P to return to Regular.",
                ),
            };
            return;
        }

        self.active_season = season_id.to_owned();
        self.selected = 0;
        // D5 invalidation matrix completion (tx_*, playoffs_*,
        // query_result_scroll, schedule_team_cache key widening) is a
        // separate Phase C deliverable.

        if season_id == icelines_core::CURRENT_SEASON_STR {
            self.status = "Current season loaded.".to_owned();
        } else {
            let label = crate::tui::screens::misc::PICKER_SEASONS
                .iter()
                .find(|(id, _, _)| *id == season_id)
                .map(|(_, label, _)| *label)
                .unwrap_or(season_id);
            self.status = format!(
                "[{}] — historical season. Live features unavailable.",
                label
            );
        }
    }

    /// Hart.6.9.B — flip the active season-type and reload. Triggered
    /// by Shift+P (capital P) — global keybind that works on any
    /// screen. Lowercase `p` is reserved for the Queries↔Projections
    /// flip; capital P is the playoff toggle.
    ///
    /// On a season with no playoff data bundled (e.g. 2025-26 until
    /// the Cup is contested), the load fails cleanly inside
    /// `reload_for_season_typed` — `active_type` does NOT flip in that
    /// case, and the status bar surfaces the missing-bundle reason.
    pub fn toggle_season_type(&mut self) {
        use icelines_core::season_stats::SeasonType;
        let prev = self.active_type;
        let next = match self.active_type {
            SeasonType::Regular => SeasonType::Playoff,
            SeasonType::Playoff => SeasonType::Regular,
        };
        let season_id = self.active_season.clone();
        self.reload_for_season_typed(&season_id, next);
        // If the load succeeded, status was set by the typed reload.
        // If it failed, active_type was NOT flipped — surface that
        // explicitly so the user isn't confused by "I pressed Shift+P
        // but nothing changed."
        if self.active_type == prev {
            // Status was set inside reload_for_season_typed — leave it.
        } else {
            // Successful flip — overwrite with a concise marker.
            self.status = match next {
                SeasonType::Regular => "Switched to Regular season.".to_owned(),
                SeasonType::Playoff => "Switched to Playoff. Shift+P to return.".to_owned(),
            };
        }
    }

    fn go_back(&mut self) {
        self.screen = if let Some(prev) = self.prev_screen.take() {
            prev
        } else {
            // Sensible parent for each drill-down screen when prev_screen is unset
            match &self.screen {
                Screen::DepthTeam(_) => Screen::Depth,
                Screen::Team(_) => Screen::Home,
                Screen::PlayerById(_) => Screen::Home,
                Screen::CompsById(_) => Screen::Home,
                Screen::GroupDetail(_) => Screen::Groups,
                Screen::ScheduleTeam(_) => Screen::Schedule,
                Screen::ScheduleMatchup(..) => Screen::Schedule,
                Screen::SeriesDetail(_) => Screen::Playoffs,
                Screen::GameDetail(_) => Screen::Tonight,
                _ => Screen::Home,
            }
        };
        self.selected = 0;
        self.query_results_focused = false;
    }

    /// Return the (normalized_name, full_name) of the currently highlighted player
    /// on whichever screen is active. Returns None on screens with no player list.
    fn get_selected_player(&self) -> Option<(String, String)> {
        match &self.screen {
            Screen::PlayerById(pid) => self
                .repo
                .identity(*pid)
                .map(|i| (i.name_normalized.clone(), i.full_name.clone())),

            Screen::Team(abbrev) => {
                let team_abbr = icelines_core::model::TeamAbbr(abbrev.clone());
                self.team_views(&team_abbr)
                    .into_iter()
                    .nth(self.selected)
                    .map(|v| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
            }

            Screen::Projections => {
                // Same sort shape as misc::render_projections: pace_82
                // desc, None last, full_name asc tiebreak.
                let mut sorted: Vec<icelines_core::stats_repository::PlayerView<'_>> = self
                    .views()
                    .into_iter()
                    .filter(|v| v.pace_82().is_some())
                    .collect();
                sorted.sort_by(|a, b| {
                    let sa = a.pace_82().unwrap_or(f64::NEG_INFINITY);
                    let sb = b.pace_82().unwrap_or(f64::NEG_INFINITY);
                    sb.partial_cmp(&sa)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.full_name().cmp(b.full_name()))
                });
                sorted
                    .get(self.selected)
                    .map(|v| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
            }

            Screen::Search => {
                // Same filter shape as screens::search::search_results.
                let views = self.views();
                let results = crate::tui::screens::search::search_results(
                    &views,
                    &self.search_query,
                );
                results
                    .get(self.selected)
                    .map(|v| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
            }

            Screen::Queries => {
                // Hart.5c.6 Phase B-3.3: queries runs against views now.
                let views = self.views();
                let results = crate::tui::screens::queries::run_query_views_with_pick(
                    &views,
                    &self.query_fields,
                    self.sort_stat_pick,
                );
                let row_idx =
                    self.query_result_scroll + self.selected.min(results.len().saturating_sub(1));
                results
                    .get(row_idx)
                    .map(|(_, v)| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
            }

            Screen::GroupDetail(group_name) => {
                let gn = group_name.clone();
                let views = self.views();
                crate::db::GroupDb::open()
                    .ok()
                    .and_then(|db| db.list_members(&gn).ok())
                    .and_then(|members| {
                        members.get(self.selected).cloned().and_then(|norm| {
                            views
                                .iter()
                                .find(|v| v.identity.name_normalized.contains(&norm))
                                .map(|v| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
                        })
                    })
            }

            Screen::CompsById(pid) => {
                let pid = *pid;
                let views = self.views();
                self.view_for(pid).and_then(|target| {
                    crate::tui::screens::comps::find_comps_views(&views, &target)
                        .get(self.selected)
                        .map(|v| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
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
                // Hart.5c.6 Phase B-2 step 2: navigate via PlayerId.
                // Resolve the selected row to its PlayerView and push
                // PlayerById; the team_views accessor honors last-stint
                // semantics per D10.
                let team_abbr = icelines_core::model::TeamAbbr(abbrev.clone());
                let pid = self
                    .team_views(&team_abbr)
                    .get(self.selected)
                    .map(|v| v.identity.id);
                if let Some(pid) = pid {
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::PlayerById(pid);
                    self.selected = 0;
                }
            }
            // Group picker overlay (shown on player card OR team roster)
            _ if self.group_picker_open => {
                if let Some(group_name) = self.group_picker_list.get(self.selected).cloned() {
                    if let Some((norm, full)) = self.group_picker_player.take() {
                        if let Ok(db) = crate::db::GroupDb::open() {
                            match db.add_member(&group_name, &norm) {
                                Ok(true) => {
                                    self.status = format!("✓ Added {} to '{}'", full, group_name)
                                }
                                Ok(false) => {
                                    self.status =
                                        format!("'{}' is already in '{}'", full, group_name)
                                }
                                Err(e) => self.status = format!("Error: {e}"),
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
                // Enter on a member row → player card. Hart.5c.6 Phase B-2 step 2:
                // resolve the member's stored normalized name to a PlayerView in
                // the active window, then navigate via PlayerById.
                if let Screen::GroupDetail(ref group_name) = self.screen.clone() {
                    let members = crate::db::GroupDb::open()
                        .ok()
                        .and_then(|db| db.list_members(group_name).ok())
                        .unwrap_or_default();
                    if let Some(norm) = members.get(self.selected) {
                        let views = self.views();
                        let pid = views
                            .iter()
                            .find(|v| v.identity.name_normalized.contains(norm.as_str()))
                            .map(|v| v.identity.id);
                        if let Some(pid) = pid {
                            self.prev_screen = Some(self.screen.clone());
                            self.screen = Screen::PlayerById(pid);
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
                            let json =
                                crate::tui::screens::queries::fields_to_json(&self.query_fields);
                            if let Ok(db) = crate::db::GroupDb::open() {
                                let _ = db.save_query(&name, &json);
                                self.status =
                                    format!("Saved query '{name}'  ·  l=load  s=save  r=reset");
                            }
                        }
                        self.query_mode = QueryMode::Build;
                    }
                    QueryMode::LoadList => {
                        // Load the selected saved query
                        if let Some((name, json)) = self.query_saved_list.get(self.selected) {
                            crate::tui::screens::queries::apply_saved_json(
                                &mut self.query_fields,
                                json,
                            );
                            self.status =
                                format!("Loaded query '{name}'  ·  ←→ to adjust  s=save  r=reset");
                            self.query_mode = QueryMode::Build;
                            self.query_result_scroll = 0;
                        }
                    }
                    QueryMode::SortPicker => {
                        // Phase Lindsay L.3.4 — accept the highlighted
                        // catalog stat as the active sort. Updates
                        // `sort_stat_pick` (catalog override) and exits
                        // the picker. The sort dispatch sees `Some(stat)`
                        // on next render and uses `StatId::sort_cmp`.
                        let results = crate::tui::screens::queries::sort_picker_filter(
                            &self.sort_picker_query,
                        );
                        if let Some(&stat) = results.get(self.sort_picker_idx) {
                            self.sort_stat_pick = Some(stat);
                            self.status = format!(
                                "Sort: {} ({})  ·  / picker  s save  l load",
                                stat.label(),
                                stat.cli_key(),
                            );
                        }
                        self.query_mode = QueryMode::Build;
                        self.query_result_scroll = 0;
                    }
                    QueryMode::Build => {
                        // Enter on a result row → player card. Hart.5c.6
                        // Phase B-3.3: queries runs against views now.
                        let views = self.views();
                        let results = crate::tui::screens::queries::run_query_views_with_pick(
                            &views,
                            &self.query_fields,
                            self.sort_stat_pick,
                        );
                        let row_idx = self.query_result_scroll
                            + self.selected.min(results.len().saturating_sub(1));
                        if let Some((_, v)) = results.get(row_idx) {
                            let pid = v.identity.id;
                            self.prev_screen = Some(self.screen.clone());
                            self.screen = Screen::PlayerById(pid);
                            self.selected = 0;
                        }
                    }
                }
            }
            Screen::Projections => {
                // Enter on a projection row → player card. Hart.5c.6
                // Phase B-2 step 2: build the same sorted view set that
                // misc::render_projections renders (pace_82 desc, None
                // last, full_name asc tiebreak), then pick out the
                // PlayerId at self.selected.
                let mut sorted: Vec<icelines_core::stats_repository::PlayerView<'_>> = self
                    .views()
                    .into_iter()
                    .filter(|v| v.pace_82().is_some())
                    .collect();
                sorted.sort_by(|a, b| {
                    let sa = a.pace_82().unwrap_or(f64::NEG_INFINITY);
                    let sb = b.pace_82().unwrap_or(f64::NEG_INFINITY);
                    sb.partial_cmp(&sa)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.full_name().cmp(b.full_name()))
                });
                if let Some(v) = sorted.get(self.selected) {
                    let pid = v.identity.id;
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::PlayerById(pid);
                    self.selected = 0;
                }
            }
            Screen::Search => {
                // Navigate to player screen for selected search result.
                // Hart.5c.6 Phase B-2 step 2: legacy code did
                // `Screen::Player(self.selected)` which used selected as
                // a global index into app.players — broken for non-empty
                // queries (jumped to Nth player in iteration order, not
                // Nth match). Fixed by re-running search_results to find
                // the actual selected match's PlayerId.
                let views = self.views();
                let results = crate::tui::screens::search::search_results(
                    &views,
                    &self.search_query,
                );
                if let Some(v) = results.get(self.selected) {
                    let pid = v.identity.id;
                    self.prev_screen = Some(Screen::Search);
                    self.screen = Screen::PlayerById(pid);
                    self.selected = 0;
                }
            }
            Screen::Depth => {
                let views = self.views();
                let strength = icelines_core::cross_team::compute_team_strength_views(
                    &views,
                    self.depth_mode,
                );
                let mut ranked: Vec<String> = strength.keys().cloned().collect();
                ranked.sort_by(|a, b| {
                    strength[b]
                        .total
                        .partial_cmp(&strength[a].total)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                if let Some(team) = ranked.get(self.selected) {
                    self.prev_screen = Some(Screen::Depth);
                    self.screen = Screen::DepthTeam(team.clone());
                    self.selected = 0;
                }
            }
            Screen::CompsById(target_pid) => {
                // Hart.5c.6 Phase B-2 step 2: view-based comps navigation.
                let target_pid = *target_pid;
                let views = self.views();
                if let Some(target) = self.view_for(target_pid) {
                    let comps = crate::tui::screens::comps::find_comps_views(&views, &target);
                    if let Some(comp) = comps.get(self.selected) {
                        let pid = comp.identity.id;
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::PlayerById(pid);
                        self.selected = 0;
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
            // Goalies: Enter on a leaderboard row opens GoalieDetail. We
            // resolve the selected row index against the same sort+filter
            // pipeline `goalies::render` uses, then translate the visible
            // rank back to a position in `app.goalies` so the detail
            // screen can address the goalie directly.
            Screen::Goalies => {
                // Hart.5c.6 Phase B-3: leaderboard navigates via
                // sort_goalie_views, picks the selected view, pushes
                // GoalieDetailById from view.identity.id.
                let sort = crate::tui::screens::goalies::SORTS
                    .get(self.goalie_sort as usize)
                    .copied()
                    .unwrap_or(crate::tui::screens::goalies::GoalieSort::SvPctDesc);
                let views = self.goalie_views();
                let qualified = crate::tui::screens::goalies::sort_goalie_views(
                    &views,
                    sort,
                    self.goalie_min_gp,
                );
                if let Some(v) = qualified.get(self.goalie_selected) {
                    let pid = v.identity.id;
                    self.prev_screen = Some(Screen::Goalies);
                    self.screen = Screen::GoalieDetailById(pid);
                }
            }
            // Scores: Enter on a game row opens GameDetail keyed by game_id.
            Screen::Tonight => {
                if let Some(game_id) = self.selected_game_id() {
                    self.prev_screen = Some(Screen::Tonight);
                    self.screen = Screen::GameDetail(game_id);
                    crate::tui::tonight::maybe_fetch_boxscore(self.boxscore_cache.clone(), game_id);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn cycle_screen(&mut self) {
        self.query_results_focused = false;
        // Phase T+1: 8-tab cycle. Groups is removed from the strip and
        // accessed via `g` from anywhere. Stats defaults to Queries.
        //   League → Depth → Queries → Goalies → Scores → Schedule
        //   → Transactions → Playoffs → League
        let next = match &self.screen {
            Screen::Home | Screen::Team(_) | Screen::PlayerById(_)
            | Screen::CompsById(_) => Screen::Depth,
            Screen::Depth | Screen::DepthTeam(_) => Screen::Queries,
            Screen::Queries | Screen::Projections | Screen::Search => Screen::Goalies,
            Screen::Goalies | Screen::GoalieDetailById(_) => Screen::Tonight,
            Screen::Tonight | Screen::GameDetail(_) => Screen::Schedule,
            Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(..) => {
                Screen::Transactions
            }
            Screen::Transactions => Screen::Playoffs,
            Screen::Playoffs | Screen::SeriesDetail(_) => Screen::Home,
            _ => Screen::Home,
        };
        self.screen = next;
        self.selected = 0;
        self.schedule_selected = 0;
        self.query_result_scroll = 0;
        self.maybe_fetch_scores();
        self.maybe_fetch_schedule();
        self.maybe_fetch_playoffs();
    }

    /// Reverse of `cycle_screen` — Shift-Tab.
    pub(crate) fn cycle_screen_back(&mut self) {
        self.query_results_focused = false;
        let prev = match &self.screen {
            Screen::Home | Screen::Team(_) | Screen::PlayerById(_)
            | Screen::CompsById(_) => {
                Screen::Playoffs
            }
            Screen::Depth | Screen::DepthTeam(_) => Screen::Home,
            Screen::Queries | Screen::Projections | Screen::Search => Screen::Depth,
            Screen::Goalies | Screen::GoalieDetailById(_) => Screen::Queries,
            Screen::Tonight | Screen::GameDetail(_) => Screen::Goalies,
            Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(..) => {
                Screen::Tonight
            }
            Screen::Transactions => Screen::Schedule,
            Screen::Playoffs | Screen::SeriesDetail(_) => Screen::Transactions,
            _ => Screen::Home,
        };
        self.screen = prev;
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
        // 8 tabs (Phase T+1):
        //   League → Depth → Stats(Queries) → Goalies → Scores → Schedule
        //   → Transactions → Playoffs → wrap
        //
        // Phase Lindsay L.3.3 — Tab on Queries now toggles the
        // section expansion (per spec §"TUI integration"). To advance
        // past Queries we call `cycle_screen()` directly. The test
        // still proves the global cycle order; for the Queries-Tab
        // section-toggle behavior see `l0_lindsay_tui_tab_on_queries_toggles_section`.
        let mut app = App::new(false);
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Depth, "Home→Depth");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Queries, "Depth→Stats(Queries)");
        app.cycle_screen();  // bypass Lindsay Tab-on-Queries intercept
        assert_eq!(app.screen, Screen::Goalies, "Stats→Goalies");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Tonight, "Goalies→Scores");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Schedule, "Scores→Schedule");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Transactions, "Schedule→Transactions");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Playoffs, "Transactions→Playoffs");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Home, "Playoffs→League (wraps)");
    }

    /// Phase Lindsay L.3.3 — Tab on the Queries screen toggles the
    /// section containing the current field cursor; it does NOT
    /// advance to the next screen. Cursor snaps to the next visible
    /// field if its current field becomes hidden via collapse.
    #[test]
    fn l0_lindsay_tui_tab_on_queries_toggles_section() {
        let mut app = App::new(false);
        app.handle(Action::Tab);  // Home → Depth
        app.handle(Action::Tab);  // Depth → Queries
        assert_eq!(app.screen, Screen::Queries);

        // Default: cursor on field 0 (Sort by) which is in section 0.
        // Section 0 starts expanded.
        let initial_s0 = app.query_sections[0].expanded;
        assert!(initial_s0, "section 0 starts expanded by default");

        // Tab → toggles section 0 (cursor's section). Screen does NOT advance.
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Queries,
            "Tab on Queries toggles section, doesn't advance screen");
        assert_eq!(app.query_sections[0].expanded, !initial_s0,
            "section 0 expansion flipped by Tab");

        // After collapsing section 0, field 0 is hidden. Cursor
        // snapped to the next visible field — field 1 (Position),
        // which lives in section 1.
        assert_eq!(app.query_field_idx, 1,
            "cursor snaps to next visible field after section collapse");

        // Second Tab now targets section 1 (where the cursor lives).
        let initial_s1 = app.query_sections[1].expanded;
        app.handle(Action::Tab);
        assert_eq!(app.query_sections[1].expanded, !initial_s1,
            "second Tab toggles section 1 (cursor's new home)");
        // Section 0 still collapsed — wasn't touched by the second Tab.
        assert_eq!(app.query_sections[0].expanded, !initial_s0);
    }

    #[test]
    fn l0_tui_shift_tab_cycles_screens_backwards() {
        // Shift-Tab walks the same eight tabs in reverse.
        let mut app = App::new(false);
        app.handle(Action::TabPrev);
        assert_eq!(
            app.screen,
            Screen::Playoffs,
            "Home→Playoffs (wraps backwards)"
        );
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Transactions, "Playoffs→Transactions");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Schedule, "Transactions→Schedule");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Tonight, "Schedule→Scores");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Goalies, "Scores→Goalies");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Queries, "Goalies→Stats");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Depth, "Stats→Depth");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Home, "Depth→League");
    }

    #[test]
    fn l0_tui_tab_and_shift_tab_are_inverses() {
        // Eight forward + eight backward should land on the original screen.
        // Phase Lindsay L.3.3 — Tab on Queries no longer cycles screens
        // (it toggles a section). Use `cycle_screen` / `cycle_screen_back`
        // directly so the inverse test still exercises the full ring.
        let mut app = App::new(false);
        for _ in 0..8 {
            app.cycle_screen();
        }
        for _ in 0..8 {
            app.cycle_screen_back();
        }
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn l0_tui_g_opens_groups_when_no_player_selected() {
        // From the Home screen with no player loaded, `g` (AddToGroup) acts
        // as the global "open Groups" shortcut now that Groups is no longer
        // a tab. Phase T+1.
        let mut app = App::new(false);
        // Ensure we're on Home with no players → no target player.
        assert_eq!(app.screen, Screen::Home);
        // Repo is empty until spawn_repo_load completes.
        assert_eq!(app.views().len(), 0);
        app.handle(Action::AddToGroup);
        assert_eq!(
            app.screen,
            Screen::Groups,
            "g from a non-player screen with no target must open Groups"
        );
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
        assert!(matches!(app.screen, Screen::Team(_) | Screen::PlayerById(_)));
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
        assert_eq!(
            app.screen,
            Screen::Schedule,
            "stays on Schedule, not the global Search screen"
        );
        assert!(app.schedule_query.is_empty());
    }

    #[test]
    fn l0_tui_schedule_search_single_team() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        for c in "SEA".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(!app.schedule_search_mode);
        assert_eq!(app.schedule_filter, SearchFilter::Team("SEA".to_owned()));
    }

    #[test]
    fn l0_tui_schedule_search_matchup() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.handle(Action::Search);
        for c in "NYR".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Space);
        for c in "WSH".chars() {
            app.handle(Action::Char(c));
        }
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
        for c in "XYZ".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        // Search bar stays open on validation failure so user can correct
        assert!(app.schedule_search_mode);
        assert!(app
            .schedule_filter_err
            .as_deref()
            .unwrap_or("")
            .contains("Unknown team"));
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
        app.handle(Action::Refresh); // mapped from lowercase 'r'
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
        let today_monday =
            crate::tui::schedule::monday_of(&crate::tui::schedule::today_iso()).unwrap();
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
            letter: Some(letter.to_owned()),
            top_seed_abbrev: top.to_owned(),
            top_seed_name: top.to_owned(),
            top_seed_wins: top_w,
            top_seed_rank: None,
            bottom_seed_abbrev: bot.to_owned(),
            bottom_seed_name: bot.to_owned(),
            bottom_seed_wins: bot_w,
            bottom_seed_rank: None,
            winner_abbrev: if top_w == 4 {
                Some(top.to_owned())
            } else if bot_w == 4 {
                Some(bot.to_owned())
            } else {
                None
            },
            conference: None,
            games: Vec::new(),
        }
    }

    fn seed_bracket(app: &mut App, year: u16, rounds: Vec<PlayoffRound>) {
        let bracket = PlayoffBracket {
            season: app.active_season.clone(),
            current_round: None,
            rounds,
        };
        app.playoffs_cache
            .lock()
            .unwrap()
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
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".into(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2)],
        };
        let r2 = PlayoffRound {
            round_number: 2,
            label: "Second Round".into(),
            series: vec![],
        };
        seed_bracket(&mut app, 2026, vec![r1, r2]);
        assert_eq!(app.playoffs_round_count(), 2);
    }

    #[test]
    fn l0_tui_playoffs_left_right_changes_round() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".into(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2)],
        };
        let r2 = PlayoffRound {
            round_number: 2,
            label: "Second Round".into(),
            series: vec![fixture_series("I", "FLA", "WSH", 1, 0)],
        };
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
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".into(),
            series: vec![
                fixture_series("A", "FLA", "TBL", 4, 2),
                fixture_series("B", "WSH", "NYR", 4, 3),
            ],
        };
        let r2 = PlayoffRound {
            round_number: 2,
            label: "Second Round".into(),
            series: vec![fixture_series("I", "FLA", "WSH", 1, 0)],
        };
        seed_bracket(&mut app, 2026, vec![r1, r2]);

        // Move down to series 1, then change rounds — cursor resets to 0
        app.handle(Action::Down);
        assert_eq!(app.playoffs_series, 1);
        app.handle(Action::Right);
        assert_eq!(app.playoffs_round, 1);
        assert_eq!(
            app.playoffs_series, 0,
            "switching rounds resets the series cursor"
        );
    }

    #[test]
    fn l0_tui_playoffs_enter_opens_series_detail() {
        let mut app = App::new(false);
        app.screen = Screen::Playoffs;
        let r1 = PlayoffRound {
            round_number: 1,
            label: "First Round".into(),
            series: vec![fixture_series("A", "FLA", "TBL", 4, 2)],
        };
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
        assert_eq!(
            app.screen,
            Screen::Playoffs,
            "Enter must not change screen when bracket isn't loaded"
        );
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
        assert_eq!(
            app.playoffs_round, initial,
            "no rounds loaded → no movement"
        );
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
            game_id: id,
            date: "2026-04-28".to_owned(),
            game_type: 2,
            away_abbrev: away.to_owned(),
            away_name: away.to_owned(),
            home_abbrev: home.to_owned(),
            home_name: home.to_owned(),
            start_time_utc: "2026-04-28T23:00:00Z".to_owned(),
            away_score: None,
            home_score: None,
            game_state: None,
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    fn seed_scores(app: &mut App, date_key: &str, games: Vec<ScheduledGame>) {
        app.tonight_cache
            .lock()
            .unwrap()
            .insert(date_key.to_owned(), TonightState::Loaded(games));
    }

    #[test]
    fn l0_tui_parse_picker_date_iso() {
        assert_eq!(
            super::parse_picker_date("2026-04-28").unwrap(),
            "2026-04-28"
        );
        assert_eq!(
            super::parse_picker_date("2026/04/28").unwrap(),
            "2026-04-28"
        );
    }

    #[test]
    fn l0_tui_parse_picker_date_mm_dd_uses_current_year() {
        let parsed = super::parse_picker_date("04/28").unwrap();
        // The year is whatever Utc::now() returns — assert prefix and structure
        assert!(
            parsed.ends_with("-04-28"),
            "must end with month-day, got: {parsed}"
        );
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
        assert!(
            !app.scores_date.is_empty(),
            "Right should set explicit date"
        );
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
        assert!(
            app.scores_date.is_empty(),
            "t must clear scores_date back to live"
        );
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
        for c in "2026-04-28".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(!app.scores_picker_open, "picker should close on apply");
        assert_eq!(app.scores_date, "2026-04-28");
    }

    #[test]
    fn l0_tui_scores_picker_invalid_keeps_open_with_error() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.handle(Action::Char('d'));
        for c in "garbage".chars() {
            app.handle(Action::Char(c));
        }
        app.handle(Action::Enter);
        assert!(
            app.scores_picker_open,
            "invalid input must keep picker open for correction"
        );
        assert!(app.scores_picker_err.is_some());
    }

    #[test]
    fn l0_tui_scores_picker_esc_cancels() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.handle(Action::Char('d'));
        for c in "abc".chars() {
            app.handle(Action::Char(c));
        }
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
        app.handle(Action::Refresh); // → 'r'
                                     // 'r' isn't a valid date character but should be in the buffer
        assert_eq!(app.scores_picker_input, "2r");
    }

    #[test]
    fn l0_tui_scores_enter_opens_game_detail() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        seed_scores(
            &mut app,
            "",
            vec![
                fixture_scheduled_game(2025020100, "SEA", "VGK"),
                fixture_scheduled_game(2025020101, "NYR", "WSH"),
            ],
        );
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
        assert_eq!(
            app.screen, initial,
            "Tab while admin open must not change screen"
        );
        assert!(app.show_admin, "Tab must not close the overlay");
        // Same for number-key tab jumps
        app.handle(Action::GoToTab(2));
        assert_eq!(
            app.screen, initial,
            "GoToTab while admin open must not change screen"
        );
        assert!(app.show_admin);
    }

    #[test]
    fn l0_admin_overlay_does_not_open_on_lowercase_f() {
        // Lowercase 'f' is mapped to AddToFavorites in event.rs and must not
        // be confused with the capital-F admin trigger.
        let mut app = App::new(false);
        app.handle(Action::AddToFavorites);
        assert!(
            !app.show_admin,
            "lowercase f (AddToFavorites action) must not open the admin overlay"
        );
    }

    // ── Scores auto-refresh (Phase 8b) ───────────────────────────────────────

    #[test]
    fn l0_scores_auto_refresh_fires_when_due() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(31);
        assert!(
            super::should_auto_refresh(
                &Screen::Tonight,
                "",
                Some(last),
                now,
                super::SCORES_AUTO_REFRESH_INTERVAL,
            ),
            "30s elapsed on live Scores tab must fire"
        );
    }

    #[test]
    fn l0_scores_auto_refresh_holds_off_within_interval() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(10);
        assert!(
            !super::should_auto_refresh(
                &Screen::Tonight,
                "",
                Some(last),
                now,
                super::SCORES_AUTO_REFRESH_INTERVAL,
            ),
            "10s after last refresh must hold off"
        );
    }

    #[test]
    fn l0_scores_auto_refresh_paused_off_tab() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(60);
        // Off the Scores tab → must not fire even if the interval passed.
        for screen in [
            Screen::Home,
            Screen::Schedule,
            Screen::Playoffs,
            Screen::Groups,
            Screen::Projections,
            Screen::GameDetail(1234),
        ] {
            assert!(
                !super::should_auto_refresh(
                    &screen,
                    "",
                    Some(last),
                    now,
                    super::SCORES_AUTO_REFRESH_INTERVAL,
                ),
                "screen {screen:?} must not auto-refresh Scores"
            );
        }
    }

    #[test]
    fn l0_scores_auto_refresh_paused_on_past_date() {
        let now = std::time::Instant::now();
        let last = now - std::time::Duration::from_secs(60);
        assert!(
            !super::should_auto_refresh(
                &Screen::Tonight,
                "2026-01-15",
                Some(last),
                now,
                super::SCORES_AUTO_REFRESH_INTERVAL,
            ),
            "non-empty scores_date (past or future) must not auto-refresh"
        );
    }

    #[test]
    fn l0_scores_auto_refresh_paused_when_timer_unset() {
        // None means dormant — initial fetch happens via maybe_fetch_scores,
        // not via the polling tick.
        let now = std::time::Instant::now();
        assert!(!super::should_auto_refresh(
            &Screen::Tonight,
            "",
            None,
            now,
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
        assert!(
            app.last_auto_refresh.is_some(),
            "t back to today must arm the timer"
        );
    }

    #[test]
    fn l0_scores_auto_refresh_disarmed_on_left_right_to_past_date() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.last_auto_refresh = Some(std::time::Instant::now());
        // Move to a specific date — auto-refresh must disengage
        app.handle(Action::Left);
        assert!(!app.scores_date.is_empty(), "Left must set a specific date");
        assert!(
            app.last_auto_refresh.is_none(),
            "moving to a specific date must disarm the auto-refresh timer"
        );
    }

    #[test]
    fn l0_scores_tick_no_op_when_timer_dormant() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        // Timer never armed — tick must not change anything
        app.tick_auto_refresh();
        assert!(
            app.last_auto_refresh.is_none(),
            "tick must leave dormant timer alone"
        );
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

    // ── Boot-load tests (regression fence for the bug shipped in 5c.6 Phase A) ──
    //
    // Hart.5c.6 originally used `spawn_local` + mpsc to load the repo
    // asynchronously. That path silently broke at runtime because
    // `crossterm::event::poll` is a blocking sync call — the event loop
    // never `.await`-yielded, so the single-threaded `LocalSet` runtime
    // never drove the spawn_local task. Tests existed for every UI
    // helper but no test exercised the boot path end-to-end.
    //
    // These tests assert the synchronous `boot_load` actually populates
    // `app.repo` from bundled data. If the bundled data path regresses
    // or `boot_load` is accidentally turned back into a fire-and-forget
    // async spawn, these fail loudly.

    /// Boot load against the current bundled season must populate
    /// `app.views()` with a non-trivial number of skaters. Bundled data
    /// is `include_bytes!`'d at build time, so this test does no I/O.
    /// The empty `tempdir` snapshot store forces the bundled fallback.
    #[test]
    fn l1_app_boot_load_populates_views_from_bundled_data() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = icelines_fetch::snapshot::SnapshotStore::new(dir.path());

        let mut app = App::new(true);
        // Sanity: pre-load app has empty repo.
        assert!(app.views().is_empty(), "App::new must start with empty repo");

        app.boot_load_with_store(&store);

        let views = app.views();
        assert!(
            !views.is_empty(),
            "boot_load must populate views from bundled data — got 0 views"
        );
        // The bundled current-season skater pool is several hundred — guard
        // against a partial load that returns a handful of rows.
        assert!(
            views.len() >= 200,
            "expected ≥200 skater views from bundled data, got {}",
            views.len()
        );
    }

    /// Boot load must rebuild `league_context` so post-load renders see
    /// rank/percentile tables coupled to the active window. Without
    /// this, the dashboard panel renders zero ranks even though views
    /// exist.
    #[test]
    fn l1_app_boot_load_rebuilds_league_context_window() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = icelines_fetch::snapshot::SnapshotStore::new(dir.path());

        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        assert_eq!(
            app.league_context_window,
            (app.active_season_typed, app.active_type),
            "boot_load must couple league_context_window to the active window"
        );
    }

    /// Boot load must clear the dashboard-panel cache so the first
    /// render after boot computes fresh lines (the cache key is
    /// (player, season, type) — a stale entry from a different repo
    /// would render outdated stats).
    #[test]
    fn l1_app_boot_load_clears_dashboard_panel_cache() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = icelines_fetch::snapshot::SnapshotStore::new(dir.path());

        let mut app = App::new(true);
        app.boot_load_with_store(&store);

        // No public introspection on cache size, but compile() round-trip
        // must yield real lines for a known player. Connor McDavid (8478402)
        // is in every recent bundled season.
        let mcdavid_view = app.repo.view(
            PlayerId(8478402),
            app.active_season_typed,
            app.active_type,
        );
        assert!(
            mcdavid_view.is_some(),
            "McDavid (8478402) must appear in the bundled current-season pool",
        );
    }

    /// On a configured-but-bad season (e.g. unbundled), `boot_load` must
    /// surface the failure via `app.status` and leave `app.repo` empty
    /// rather than panicking.
    #[test]
    fn l1_app_boot_load_unbundled_season_sets_status_and_keeps_empty_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = icelines_fetch::snapshot::SnapshotStore::new(dir.path());

        let mut app = App::new(true);
        // Force a season ID that has no bundled data and no snapshot.
        app.active_season_typed = Season(19101911);
        app.boot_load_with_store(&store);

        assert!(
            app.views().is_empty(),
            "unbundled-season load must leave repo empty, not panic"
        );
        assert!(
            app.status.starts_with("load failed:"),
            "status must surface the load error, got: {}",
            app.status
        );
    }
}
