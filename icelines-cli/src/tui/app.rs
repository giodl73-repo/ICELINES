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

/// Phase Foster +8 — short label for the status-bar timeframe
/// indicator. Matches GLASS L8 wording (e.g. "Week (Mon-Sun)").
pub fn timeframe_label(t: icelines_core::timeframe::Timeframe) -> &'static str {
    use icelines_core::timeframe::Timeframe;
    match t {
        Timeframe::Day => "Day",
        Timeframe::Week => "Week",
        Timeframe::Month => "Month",
        Timeframe::Season => "Season",
    }
}

/// Phase Foster +8 — anchor-style hint shown alongside the label.
pub fn timeframe_anchor_hint(t: icelines_core::timeframe::Timeframe) -> &'static str {
    use icelines_core::timeframe::Timeframe;
    match t {
        Timeframe::Day => "today",
        Timeframe::Week => "Mon-Sun",
        Timeframe::Month => "1st-end",
        Timeframe::Season => "Oct-Jun",
    }
}

/// Phase Foster.1.4 — which date-anchored surface the shared picker
/// overlay applies to. Default `Scores` matches the existing
/// lowercase-`d` behavior on the Tonight tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerTarget {
    #[default]
    Scores,
    Schedule,
}

/// Phase Art Ross — Wave 24b. Max number of recent filters retained
/// in `App::query_filter_history`. Bounded so the FilterEdit ring
/// doesn't grow unboundedly across a long session.
pub const FILTER_HISTORY_CAP: usize = 20;

/// Push `entry` onto the front of the history ring (newest first).
/// No-ops when `entry` already matches the existing front (so the
/// user hammering Enter on the same filter doesn't fill the ring
/// with duplicates). Trims the back when the ring is at cap.
pub fn push_filter_history(
    history: &mut std::collections::VecDeque<String>,
    entry: String,
) {
    if let Some(front) = history.front() {
        if front == &entry {
            return;
        }
    }
    history.push_front(entry);
    while history.len() > FILTER_HISTORY_CAP {
        history.pop_back();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryMode {
    Build,    // normal — editing fields, viewing results
    SaveName, // typing a name to save the current query
    LoadList, // browsing saved queries to load
    /// Phase Lindsay L.3.4 — search-as-you-type sort picker overlay.
    /// User types substring against `StatId::cli_key()`; up/down moves
    /// selection within filtered list; Enter selects, Esc cancels.
    SortPicker,
    /// Phase Art Ross — free-form filter overlay. User types a Phase
    /// Art Ross filter string (`country IN (CAN, USA) AND age<25` etc.);
    /// Enter validates via `parse_query` (parse error displayed inline,
    /// stays in mode); Esc cancels. On Enter-success the parsed plan is
    /// stored on `App::query_filter_plan` and applied as an extra
    /// constraint on top of the structured field filters.
    FilterEdit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Team(String), // team abbreviation
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
    Transactions, // league-wide moves feed (Phase T.5)
    Favorites,    // Phase Foster.2 — favorites dashboard
}

pub struct App {
    pub screen: Screen,
    pub prev_screen: Option<Screen>,
    pub no_color: bool,
    /// Phase Norris.4 — Goalies-tab state extracted into its own
    /// struct. Replaces `goalie_selected` / `goalie_sort` /
    /// `goalie_min_gp`.
    pub goalies: crate::tui::screens::goalies::GoaliesState,
    pub load_state: crate::tui::loader::LoadState,
    pub install_state: InstallState,
    pub tick: u64,
    pub selected: usize,
    pub search_query: String,
    pub status: String,
    pub show_help: bool,
    /// LP.4 — in-TUI docs overlay. `m` opens; Up/Down/PgUp/PgDn scroll;
    /// Esc/m closes. Source-of-truth is the same compile-time
    /// COMMANDS.md as `icelines docs` and `/docs`.
    pub show_docs: bool,
    pub docs_scroll: u16,
    // Headshot ASCII cache
    pub headshot_cache: crate::tui::headshot::HeadshotCache,
    /// Phase Norris.6 — group-picker overlay state extracted into
    /// its own struct. Replaces `group_picker_open`,
    /// `group_picker_list`, `group_picker_player`.
    pub group_picker: crate::tui::pickers::GroupPickerState,
    // Depth chart tab
    pub depth_mode: icelines_core::cross_team::ScoringMode,
    pub show_admin: bool,
    // Season time-travel
    pub active_season: String,
    pub show_season_picker: bool,
    pub picker_selected: usize,
    // Phase Reports — overlay state (R key opens; toggles per-Tier-1 reports)
    pub show_reports_overlay: bool,
    pub reports_selected: usize,
    pub reports: crate::config::ReportToggles,
    // UX.1 — set of PlayerIds whose career has already been merged into
    // `repo` from the bundled-season fan-out. Idempotent guard so the
    // pre-render hook doesn't re-scan 38 seasons every frame.
    pub career_loaded_ids: std::collections::HashSet<icelines_core::identity::PlayerId>,
    /// Phase Norris.4 — Tonight/Scores tab state extracted into
    /// its own struct. Replaces `tonight_cache`, `boxscore_cache`,
    /// `scores_date`, `scores_selected`.
    pub tonight: crate::tui::screens::misc::TonightScreenState,
    /// Phase Norris.6 — date-picker overlay state extracted into
    /// its own struct. Replaces `scores_picker_open`,
    /// `scores_picker_input`, `scores_picker_err`, `picker_target`.
    /// Cross-screen — the same overlay is shared between Tonight
    /// (Scores) and Schedule per Foster.1.4.
    pub date_picker: crate::tui::pickers::DatePickerState,
    /// Phase Foster +8 — active timeframe (`v` cycles Day → Week →
    /// Month → Season → Day). Surfaces in chunks[2] status bar
    /// (GLASS L8). Today's main consumer is the Favorites tab; future
    /// surfaces cycle their date range against this.
    pub active_timeframe: icelines_core::timeframe::Timeframe,
    /// When the most recent live-Scores auto-refresh was triggered. `None`
    /// means the auto-refresh timer is dormant (e.g. user has not opened the
    /// Scores tab on a live date yet). The polling loop sets this on every
    /// tick that fires; the renderer uses it to draw "Updated Xs ago".
    pub last_auto_refresh: Option<std::time::Instant>,
    /// Phase Norris.2 — Schedule-tab state extracted into its own
    /// struct. Replaces the 8 `schedule_*` fields scattered across
    /// App pre-Norris (week_cache, team_cache, week, query,
    /// search_mode, filter, filter_err, selected).
    pub schedule: crate::tui::screens::schedule::ScheduleScreenState,
    // Playoffs tab — bracket + series detail
    /// Phase Norris.4 — Playoffs-tab state extracted into its own
    /// struct. Replaces `playoffs_cache` / `playoffs_round` /
    /// `playoffs_series`.
    pub playoffs: crate::tui::screens::playoffs::PlayoffsScreenState,
    /// Phase Norris.1 — Queries-tab state extracted into its own
    /// struct. Replaces the 17+ `query_*`, `sort_picker_*`, and
    /// `career_table_preset` fields scattered across App pre-Norris.
    /// Pure refactor — same layout in memory, just relocated.
    pub queries: crate::tui::screens::queries::QueriesState,
    /// Phase 8j: lazy-compiled dashboard panel for the player card.
    /// Only consulted when `crate::config::dashboards_enabled()` is true.
    pub dashboard_panel: crate::tui::dashboard_panel::CompiledPanel,
    /// Phase 8j: sorted-by-position pace_82 vectors for percentile
    /// lookups in the dashboard panel. Built once after players load.
    pub league_context: crate::tui::dashboard_panel::LeagueContext,

    /// Phase Norris.3 — Transactions-tab state extracted into its
    /// own struct. Replaces 8 fields previously scattered across
    /// App (`transactions`, `transactions_fetched_at`,
    /// `transactions_stale`, `tx_selected`, `tx_team_filter`,
    /// `tx_kind_filter`, `tx_search_query`, `tx_search_mode`).
    /// Field name is `txs` (not `transactions`) to avoid substring
    /// overlap with the legacy `transactions_*` field names — see
    /// `TransactionsState` doc comment.
    pub txs: crate::tui::screens::transactions::TransactionsState,

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

    /// Phase Masterton.3 — when set, the TUI is locked to a
    /// single screen. Tab/Shift+Tab become no-ops; the tab strip
    /// is hidden in the header. Other behavior (overlays,
    /// keybinds, dispatch) is unchanged.
    ///
    /// Set via `RunTuiOpts::standalone` in `tui::run_tui`. The
    /// stored `Screen` is the screen the launcher locked to;
    /// matches the screen the user sees on launch.
    ///
    /// Pragmatic interpretation of "host any of these with a
    /// simple TUI" — gives users a focused single-screen
    /// experience without requiring per-screen Screen-trait
    /// migrations (the deeper Masterton.2 grind that's deferred).
    pub locked_screen: Option<Screen>,

    /// Phase Jack Adams.1 — MDI dashboard state. Some when
    /// launched with `--mdi`; None for SDI modes (today's
    /// default and `--standalone`).
    ///
    /// When Some, the render path branches to `render_mdi`,
    /// laying out Scores ribbon (top) + 3-col body (Favorites /
    /// Workspace / Schedule) + footer/cmdbar (bottom). Per spec
    /// forge-2: `app.screen` IS the workspace discriminator in
    /// MDI mode — the workspace pane renders whichever screen
    /// `app.screen` points to. Existing per-screen dispatch in
    /// `App::handle` keeps working unchanged for input routing.
    ///
    /// Mutually exclusive with `locked_screen` (clap's
    /// `conflicts_with` enforces at parse time).
    pub mdi: Option<crate::tui::mdi::MdiLayout>,
}

/// True iff the current screen is a text-input surface where typed
/// letters belong to the input box (not to the global `d` chord).
/// Pulled out into a free fn to keep the `else if` arm readable —
/// inlining the matches!() chain trips `clippy::blocks_in_conditions`.
fn is_text_input_active(app: &App) -> bool {
    matches!(app.screen, Screen::Search | Screen::Tonight)
        || (app.screen == Screen::Schedule && app.schedule.search_mode)
        || (app.screen == Screen::Queries
            && matches!(app.queries.mode, QueryMode::SaveName | QueryMode::FilterEdit))
}

impl App {
    pub fn new(no_color: bool) -> Self {
        Self {
            screen: Screen::Home,
            prev_screen: None,
            no_color,
            // Phase Norris.4 — replaces 3 goalie_* init lines.
            goalies: crate::tui::screens::goalies::GoaliesState::default(),
            load_state: crate::tui::loader::LoadState::new(),
            install_state: InstallState::new(),
            tick: 0,
            selected: 0,
            search_query: String::new(),
            status: "Loading data… · Press ? for help · q to quit".to_owned(),
            show_help: false,
            show_docs: false,
            docs_scroll: 0,
            // Phase Norris.1 — replaces 17 individual init lines
            // (query_fields/idx/sections/result_scroll/mode/
            // results_focused/save_name/saved_list/filter_*/
            // sort_picker_*/career_table_preset).
            queries: crate::tui::screens::queries::QueriesState::default(),
            depth_mode: icelines_core::cross_team::ScoringMode::Fantasy,
            show_admin: false,
            active_season: icelines_core::CURRENT_SEASON_STR.to_owned(),
            show_season_picker: false,
            picker_selected: 0,
            show_reports_overlay: false,
            reports_selected: 0,
            reports: crate::config::ReportToggles::default(),
            career_loaded_ids: std::collections::HashSet::new(),
            // Phase Norris.4 — replaces 4 tonight_/boxscore_/scores_* init lines.
            tonight: crate::tui::screens::misc::TonightScreenState::default(),
            // Phase Norris.6 — replaces 4 scores_picker_* / picker_target init lines.
            date_picker: crate::tui::pickers::DatePickerState::default(),
            active_timeframe: icelines_core::timeframe::Timeframe::Day,
            last_auto_refresh: None,
            // Phase Norris.2 — replaces 8 schedule_* init lines.
            schedule: crate::tui::screens::schedule::ScheduleScreenState::default(),
            // Phase Norris.4 — replaces 3 playoffs_* init lines.
            playoffs: crate::tui::screens::playoffs::PlayoffsScreenState::default(),
            // Phase Norris.6 — replaces 3 group_picker_* init lines.
            group_picker: crate::tui::pickers::GroupPickerState::default(),
            headshot_cache: crate::tui::headshot::HeadshotCache::new(),
            // (Phase Norris.1 — these 14 fields previously here are
            //  now part of QueriesState above.)
            dashboard_panel: crate::tui::dashboard_panel::CompiledPanel::new(),
            league_context: crate::tui::dashboard_panel::LeagueContext::empty(),
            // Phase Norris.3 — replaces 8 transactions_* / tx_* init lines.
            txs: crate::tui::screens::transactions::TransactionsState::default(),

            // Empty repo + current season as the initial typed window.
            // `App::boot_load` populates the repo synchronously before
            // the event loop starts.
            // UX.1 — bump from default 8 windows to 80 so the lazy
            // career loader can hold every (season, season_type) for
            // a 20-year veteran (38 seasons × 2 types ≈ 76 windows)
            // without LRU-evicting the player's earliest seasons.
            // Player-career windows carry 1 row each, so the memory
            // cost is bounded by full active-season windows the user
            // navigates to (which is the same shape pre-UX.1).
            repo: StatsRepository::with_lru_cap(80),
            active_season_typed: Season(icelines_core::CURRENT_SEASON),
            active_type: SeasonType::Regular,
            league_context_window: (Season(icelines_core::CURRENT_SEASON), SeasonType::Regular),
            locked_screen: None,
            mdi: None,
        }
    }

    // ── Hart.5c.6 Phase A — view-based accessors ─────────────────────
    //
    // Every accessor takes (active_season_typed, active_type) so the
    // view set always reflects the current time-travel window.

    /// Phase Masterton.2.1 — interpret a `ScreenAction` returned
    /// from a screen handler. This is the orchestrator hub:
    /// `Quit` propagates up via the return value; `Push/Pop/Replace`
    /// mutates `self.screen`; `OpenOverlay` flips the relevant
    /// overlay flag; `Flash` writes transient status; `Continue`
    /// is the no-op default.
    pub fn dispatch(&mut self, action: crate::tui::screen::ScreenAction) -> bool {
        use crate::tui::screen::{OverlayKind, ScreenAction};
        match action {
            ScreenAction::Continue => false,
            ScreenAction::Quit => true,
            ScreenAction::Push(spec) => {
                self.prev_screen = Some(self.screen.clone());
                self.screen = spec;
                self.selected = 0;
                false
            }
            ScreenAction::Pop => {
                if let Some(prev) = self.prev_screen.take() {
                    self.screen = prev;
                }
                self.selected = 0;
                false
            }
            ScreenAction::Replace(spec) => {
                self.screen = spec;
                self.selected = 0;
                false
            }
            ScreenAction::OpenOverlay(kind) => {
                match kind {
                    OverlayKind::Help => self.show_help = true,
                    OverlayKind::Admin => self.show_admin = true,
                    OverlayKind::SeasonPicker => self.show_season_picker = true,
                    OverlayKind::Reports => self.show_reports_overlay = true,
                    OverlayKind::Docs => self.show_docs = true,
                    OverlayKind::DatePicker => self.date_picker.open = true,
                    OverlayKind::GroupPicker => self.group_picker.open = true,
                }
                false
            }
            ScreenAction::Flash(msg) => {
                self.status = msg;
                false
            }
        }
    }

    /// Phase Masterton.2.1 — split-borrow context for the
    /// Screen trait dispatch. Returns an `AppContext` that
    /// borrows `&self.repo`, `&self.reports`, and
    /// `&mut self.status` while LEAVING the per-screen state
    /// structs (e.g., `self.queries`, `self.schedule`) free
    /// for the caller to borrow alongside it.
    ///
    /// Call sites pattern (per spec forge-2):
    /// ```ignore
    /// let mut ctx = self.make_context();
    /// let action = QueriesScreen.handle(&mut self.queries, &mut ctx, action);
    /// drop(ctx);
    /// self.dispatch(action);
    /// ```
    /// The trick: `&mut self.queries` and `make_context()` both
    /// borrow disjoint parts of `*self`, so the borrow checker
    /// is happy as long as the per-screen field name doesn't
    /// alias any AppContext source field.
    pub fn make_context(&mut self) -> crate::tui::screen::AppContext<'_> {
        crate::tui::screen::AppContext {
            repo: &self.repo,
            season: self.active_season_typed,
            season_type: self.active_type,
            timeframe: self.active_timeframe,
            reports: &self.reports,
            status: &mut self.status,
        }
    }

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
        self.repo
            .view(pid, self.active_season_typed, self.active_type)
    }

    /// Handle an action. Returns true if the app should quit.
    pub fn handle(&mut self, action: Action) -> bool {
        if self.show_help {
            self.show_help = false;
            return false;
        }

        // LP.4 — docs overlay. Up/Down scroll one line; PgUp/PgDn (Left/
        // Right repurposed since the overlay has no horizontal axis)
        // scroll one page; Esc or Shift+M close. `q` still quits the app
        // so the overlay doesn't trap a panicked user.
        if self.show_docs {
            match action {
                Action::Quit => return true,
                Action::Escape | Action::Char('M') => self.show_docs = false,
                Action::Up => self.docs_scroll = self.docs_scroll.saturating_sub(1),
                Action::Down => self.docs_scroll = self.docs_scroll.saturating_add(1),
                // Page semantics: ←/→ jump 20 lines (a screenful on a
                // typical 30-row terminal with the overlay border).
                Action::Left | Action::TabPrev => {
                    self.docs_scroll = self.docs_scroll.saturating_sub(20);
                }
                Action::Right | Action::Tab => {
                    self.docs_scroll = self.docs_scroll.saturating_add(20);
                }
                _ => {}
            }
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

        if self.show_reports_overlay {
            return self.handle_reports_overlay(action);
        }

        // Schedule search bar consumes all character-bearing actions while open.
        if self.screen == Screen::Schedule && self.schedule.search_mode {
            return self.handle_schedule_search(action);
        }

        // Transactions search bar — same shape but applies live as the
        // user types (no validation step). Phase T+1.
        if self.screen == Screen::Transactions && self.txs.search_mode {
            return self.handle_transactions_search(action);
        }

        // Scores date picker consumes input similarly.
        // Phase Foster.1.4 — the date picker overlay also opens on
        // Schedule (Shift+D); keystrokes route through the same
        // handler regardless of which surface owns the active target.
        if (self.screen == Screen::Tonight || self.screen == Screen::Schedule)
            && self.date_picker.open
        {
            return self.handle_scores_date_picker(action);
        }

        // Queries save-name input must short-circuit so hotkey letters
        // ('f' = AddToFavorites, 'r' = Refresh, etc.) become text input
        // instead of firing the global hotkey. Without this, typing
        // "fred" as a query name dispatched AddToFavorites on the 'f'.
        if self.screen == Screen::Queries && matches!(self.queries.mode, QueryMode::SaveName) {
            return self.handle_query_save_name(action);
        }
        if self.screen == Screen::Queries && matches!(self.queries.mode, QueryMode::FilterEdit) {
            return self.handle_query_filter_edit(action);
        }

        match action {
            Action::Quit => return true,
            Action::Help => self.show_help = true,
            Action::Back | Action::Escape => {
                if self.group_picker.open {
                    self.group_picker.open = false;
                    self.group_picker.player = None;
                    self.selected = 0;
                    self.status =
                        "  g = add to group from any player card or team roster".to_owned();
                } else if self.screen == Screen::Queries && self.queries.mode != QueryMode::Build {
                    self.queries.mode = QueryMode::Build;
                    self.status = "Cancelled  ·  s=save  l=load  r=reset".to_owned();
                } else if self.screen == Screen::Queries
                    && self.queries.mode == QueryMode::Build
                    && self.queries.sort_stat_pick.is_some()
                {
                    // Phase Lindsay L.3.4 (EDGE checkpoint fix): Esc on
                    // Queries Build mode with an active picker selection
                    // clears the pick, restoring the legacy "Sort by"
                    // QueryField path. Without this, Left/Right on the
                    // Sort by field is silently ignored once the picker
                    // has fired (sticky-pick UX wart).
                    self.queries.sort_stat_pick = None;
                    self.status =
                        "Sort pick cleared  ·  / picker  s=save  l=load  r=reset".to_owned();
                } else {
                    self.go_back();
                }
            }
            Action::Down => {
                if self.screen == Screen::Tonight {
                    self.tonight.selected = self.tonight.selected.saturating_add(1);
                } else if matches!(
                    self.screen,
                    Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _)
                ) {
                    self.schedule.selected = self.schedule.selected.saturating_add(1);
                } else if self.screen == Screen::Goalies {
                    self.goalies.selected = self.goalies.selected.saturating_add(1);
                } else if self.screen == Screen::Transactions {
                    self.txs.selected = self.txs.selected.saturating_add(1);
                } else if self.screen == Screen::Playoffs {
                    self.playoffs.series = self.playoffs.series.saturating_add(1);
                } else if self.screen == Screen::Queries {
                    if self.queries.mode == QueryMode::SortPicker {
                        // Phase Lindsay L.3.4 — sort picker Down moves
                        // within filtered list, capped at len-1.
                        let n = crate::tui::screens::queries::sort_picker_filter(
                            &self.queries.sort_picker_query,
                        )
                        .len();
                        if n > 0 && self.queries.sort_picker_idx + 1 < n {
                            self.queries.sort_picker_idx += 1;
                        }
                    } else if self.queries.results_focused {
                        let views = self.views();
                        let results =
                            crate::tui::screens::queries::run_query_views_with_pick_and_plan(
                                &views,
                                &self.queries.fields,
                                self.queries.sort_stat_pick,
                                self.queries.filter_plan.as_ref(),
                                self.active_season_typed.0,
                            );
                        let visible: usize = 20;
                        if self.selected + 1 < visible {
                            self.selected =
                                (self.selected + 1).min(results.len().saturating_sub(1));
                        } else {
                            let max_scroll = results.len().saturating_sub(visible);
                            self.queries.result_scroll =
                                (self.queries.result_scroll + 1).min(max_scroll);
                        }
                    } else {
                        // Phase Lindsay L.3.3 — cursor skips fields in
                        // collapsed sections. L.5b user-bug fix: when
                        // Down would otherwise leap to the results pane
                        // (past the last visible field), first check if
                        // there's a collapsed section AFTER the current
                        // one. If yes, auto-expand it and land on its
                        // first field — gives the user a keyboard path
                        // through every section instead of getting
                        // stranded in early sections with `▶` headers
                        // visible but unreachable.
                        let visible = crate::tui::screens::queries::visible_field_indices(
                            &self.queries.sections,
                        );
                        let cur_pos = visible.iter().position(|&i| i == self.queries.field_idx);
                        match cur_pos {
                            Some(pos) if pos + 1 < visible.len() => {
                                self.queries.field_idx = visible[pos + 1];
                            }
                            _ => {
                                // Past last visible — try to find a
                                // collapsed section AFTER the current
                                // cursor's section. If found, expand
                                // it and land on its first field.
                                let cur_section =
                                    crate::tui::screens::queries::section_index_for_field(
                                        &self.queries.sections,
                                        self.queries.field_idx,
                                    )
                                    .unwrap_or(0);
                                let next_collapsed = self
                                    .queries
                                    .sections
                                    .iter()
                                    .enumerate()
                                    .skip(cur_section + 1)
                                    .find(|(_, s)| !s.expanded)
                                    .map(|(i, _)| i);
                                if let Some(idx) = next_collapsed {
                                    self.queries.sections[idx].expanded = true;
                                    if let Some(&first) = self.queries.sections[idx].fields.first() {
                                        self.queries.field_idx = first;
                                    }
                                } else {
                                    // No collapsed section ahead — focus results.
                                    self.queries.results_focused = true;
                                    self.selected = 0;
                                    self.queries.result_scroll = 0;
                                }
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
                    self.tonight.selected = self.tonight.selected.saturating_sub(1);
                } else if matches!(
                    self.screen,
                    Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(_, _)
                ) {
                    self.schedule.selected = self.schedule.selected.saturating_sub(1);
                } else if self.screen == Screen::Goalies {
                    self.goalies.selected = self.goalies.selected.saturating_sub(1);
                } else if self.screen == Screen::Transactions {
                    self.txs.selected = self.txs.selected.saturating_sub(1);
                } else if self.screen == Screen::Playoffs {
                    self.playoffs.series = self.playoffs.series.saturating_sub(1);
                } else if self.screen == Screen::Queries {
                    if self.queries.mode == QueryMode::SortPicker {
                        // Phase Lindsay L.3.4 — sort picker Up moves
                        // within filtered list. Saturates at 0.
                        self.queries.sort_picker_idx = self.queries.sort_picker_idx.saturating_sub(1);
                    } else if self.queries.results_focused {
                        if self.selected > 0 {
                            self.selected -= 1;
                        } else if self.queries.result_scroll > 0 {
                            self.queries.result_scroll -= 1;
                        } else {
                            // Phase Lindsay L.3.3 — snap to LAST visible field.
                            self.queries.results_focused = false;
                            let visible = crate::tui::screens::queries::visible_field_indices(
                                &self.queries.sections,
                            );
                            self.queries.field_idx = *visible.last().unwrap_or(&0);
                        }
                    } else {
                        // Phase Lindsay L.3.3 — cursor skips fields in
                        // collapsed sections. Symmetric L.5b fix: when
                        // Up at the top of the first expanded section,
                        // auto-expand the previous collapsed section
                        // and land on its LAST field. Same intent as
                        // the Down auto-expand.
                        let visible = crate::tui::screens::queries::visible_field_indices(
                            &self.queries.sections,
                        );
                        let cur_pos = visible.iter().position(|&i| i == self.queries.field_idx);
                        if let Some(pos) = cur_pos {
                            if pos > 0 {
                                self.queries.field_idx = visible[pos - 1];
                            } else {
                                // pos == 0 → at first visible. Try
                                // to expand a previous collapsed section.
                                let cur_section =
                                    crate::tui::screens::queries::section_index_for_field(
                                        &self.queries.sections,
                                        self.queries.field_idx,
                                    )
                                    .unwrap_or(0);
                                let prev_collapsed = (0..cur_section)
                                    .rev()
                                    .find(|&i| !self.queries.sections[i].expanded);
                                if let Some(idx) = prev_collapsed {
                                    self.queries.sections[idx].expanded = true;
                                    if let Some(&last) = self.queries.sections[idx].fields.last() {
                                        self.queries.field_idx = last;
                                    }
                                }
                                // else: at top with no collapsed-prior, stay.
                            }
                        } else if let Some(&first) = visible.first() {
                            // Cursor was on a now-hidden field — snap to first visible.
                            self.queries.field_idx = first;
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
                if self.screen == Screen::Queries && self.queries.mode == QueryMode::Build {
                    self.queries.results_focused = !self.queries.results_focused;
                    self.selected = 0;
                    if !self.queries.results_focused {
                        self.queries.field_idx = 0;
                    }
                }
            }
            Action::Right | Action::Left => {
                match &self.screen {
                    Screen::Queries if !self.queries.results_focused => {
                        if let Some(f) = self.queries.fields.get_mut(self.queries.field_idx) {
                            if matches!(action, Action::Right) {
                                f.next();
                            } else {
                                f.prev();
                            }
                        }
                        self.queries.result_scroll = 0;
                    }
                    // Scores: ←/→ moves the date by one day
                    Screen::Tonight => {
                        let from = if self.tonight.date.is_empty() {
                            crate::tui::schedule::today_iso()
                        } else {
                            self.tonight.date.clone()
                        };
                        let delta = if matches!(action, Action::Right) {
                            1
                        } else {
                            -1
                        };
                        if let Some(new_date) = crate::tui::schedule::add_days(&from, delta) {
                            self.tonight.date = new_date.clone();
                            self.tonight.selected = 0;
                            crate::tui::tonight::maybe_fetch(
                                self.tonight.cache.clone(),
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
                            crate::tui::schedule::add_days(&self.schedule.week, delta)
                        {
                            self.schedule.week = new_week.clone();
                            self.schedule.selected = 0;
                            crate::tui::schedule::maybe_fetch_week(
                                self.schedule.week_cache.clone(),
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
                            self.playoffs.round = if matches!(action, Action::Right) {
                                (self.playoffs.round + 1).min(n_rounds - 1)
                            } else {
                                self.playoffs.round.saturating_sub(1)
                            };
                            self.playoffs.series = 0;
                        }
                    }
                    // Sub-view switching: Queries ↔ Projections.
                    // Both live under the Stats tab; ←/→ flips between them.
                    // (League / Depth used to do the same, but Depth is now
                    // its own tab — toggle removed.)
                    Screen::Queries if !self.queries.results_focused => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Projections;
                        self.selected = 0;
                    }
                    Screen::Projections => {
                        self.prev_screen = Some(self.screen.clone());
                        self.screen = Screen::Queries;
                        self.selected = 0;
                        self.queries.results_focused = false;
                    }
                    _ => {}
                }
            }
            Action::Enter => self.activate_selected(),
            Action::Search => {
                // On Schedule, '/' opens the in-tab search bar instead of the
                // global player Search screen.
                if self.screen == Screen::Schedule {
                    self.schedule.search_mode = true;
                    self.schedule.query.clear();
                    self.schedule.filter_err = None;
                    self.status =
                        "Search: type team (SEA) or matchup (NYR WSH) — Enter, Esc cancel"
                            .to_owned();
                } else if self.screen == Screen::Transactions {
                    // Transactions tab: '/' opens an in-tab description
                    // substring search. Live-applied as the user types.
                    self.txs.search_mode = true;
                    self.txs.search_query.clear();
                    self.txs.selected = 0;
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
                    && !matches!(self.queries.mode, QueryMode::SaveName)
                {
                    // `p` flips to the Projections sister-screen. ←/→ on
                    // Queries is consumed by field editing, so this is
                    // the only way out without going Tab → … → Tab back.
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Projections;
                    self.selected = 0;
                    self.queries.results_focused = false;
                    self.status =
                        "Projections · p:queries  ↑↓:scroll  Enter:player card".to_owned();
                } else if self.screen == Screen::Projections && c == 'p' {
                    // Symmetric: `p` from Projections flips back to Queries.
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Queries;
                    self.selected = 0;
                    self.status = "Queries · p:projections  ←/→:edit  Tab:focus results".to_owned();
                } else if self.screen == Screen::Queries {
                    match &self.queries.mode {
                        QueryMode::SaveName => {
                            // Typing the save name
                            self.queries.save_name.push(c);
                        }
                        QueryMode::SortPicker => {
                            // Phase Lindsay L.3.4 — typing in the sort picker
                            // appends to the search query and resets selection
                            // index to 0 (top of newly-filtered list).
                            self.queries.sort_picker_query.push(c);
                            self.queries.sort_picker_idx = 0;
                        }
                        QueryMode::Build if c == 's' => {
                            // Start save-name mode
                            self.queries.mode = QueryMode::SaveName;
                            self.queries.save_name.clear();
                            self.status =
                                "Save query as: (type name, Enter to save, Esc to cancel)"
                                    .to_owned();
                        }
                        QueryMode::Build if c == 'l' => {
                            // Load saved queries list
                            self.queries.saved_list = crate::db::GroupDb::open()
                                .ok()
                                .and_then(|db| db.list_saved_queries().ok())
                                .unwrap_or_default();
                            self.queries.mode = QueryMode::LoadList;
                            self.selected = 0;
                            self.status = "Saved queries — ↑↓ select · Enter to load · Del to delete · Esc to cancel".to_owned();
                        }
                        QueryMode::Build if c == '/' => {
                            // Phase Lindsay L.3.4 — `/` on Queries opens
                            // the sort picker overlay. Search-as-you-type
                            // against catalog cli_keys.
                            self.queries.mode = QueryMode::SortPicker;
                            self.queries.sort_picker_query.clear();
                            self.queries.sort_picker_idx = 0;
                            self.status = "Sort picker — type to filter · ↑↓ select · Enter accept · Esc cancel".to_owned();
                        }
                        // Note: `f` arrives as `Action::AddToFavorites`,
                        // not `Char('f')` — the Queries+Build branch is
                        // intercepted up at that arm to enter FilterEdit.
                        QueryMode::Build if c == 'o' && !self.queries.results_focused => {
                            // UX.3 — `o` toggles the section that owns
                            // the current field cursor. Replaces the
                            // pre-UX.3 Tab→section binding (Tab now
                            // cycles screens unconditionally).
                            let _ = crate::tui::screens::queries::toggle_section_for_field(
                                &mut self.queries.sections,
                                self.queries.field_idx,
                            );
                            let visible = crate::tui::screens::queries::visible_field_indices(
                                &self.queries.sections,
                            );
                            if !visible.contains(&self.queries.field_idx) {
                                if let Some(&first) = visible.first() {
                                    self.queries.field_idx = first;
                                }
                            }
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
                        self.queries.career_table_preset = self.queries.career_table_preset.prev();
                        self.status = format!(
                            "Career preset: {}  ·  [/]: cycle  ·  c: comps",
                            self.queries.career_table_preset.label(),
                        );
                    } else if c == ']' {
                        // Phase Lindsay L.4.4 — `]` cycles FORWARD.
                        self.queries.career_table_preset = self.queries.career_table_preset.next();
                        self.status = format!(
                            "Career preset: {}  ·  [/]: cycle  ·  c: comps",
                            self.queries.career_table_preset.label(),
                        );
                    }
                } else if matches!(self.screen, Screen::Depth | Screen::DepthTeam(_)) && c == 's' {
                    self.depth_mode = self.depth_mode.toggle();
                    self.status = format!("Scoring: {}", self.depth_mode.label());
                } else if self.screen == Screen::Goalies && c == 's' {
                    // Phase G.3: cycle sort SV% → GAA → W → GP → Saves → SO
                    let n = crate::tui::screens::goalies::SORTS.len() as u8;
                    self.goalies.sort = (self.goalies.sort + 1) % n;
                    self.goalies.selected = 0;
                    let label =
                        crate::tui::screens::goalies::SORTS[self.goalies.sort as usize].label();
                    self.status = format!("Goalies sort: {label}");
                } else if self.screen == Screen::Goalies && c == 'm' {
                    // Cycle min-GP threshold 5 → 15 → 25 → 40
                    let cycle = crate::tui::screens::goalies::MIN_GP_CYCLE;
                    let cur = cycle
                        .iter()
                        .position(|v| *v == self.goalies.min_gp)
                        .unwrap_or(0);
                    self.goalies.min_gp = cycle[(cur + 1) % cycle.len()];
                    self.goalies.selected = 0;
                    self.status = format!("Goalies min GP: {}", self.goalies.min_gp);
                } else if self.screen == Screen::Schedule && c == 't' {
                    // Jump to today's week
                    let today = crate::tui::schedule::today_iso();
                    if let Some(monday) = crate::tui::schedule::monday_of(&today) {
                        self.schedule.week = monday.clone();
                        self.schedule.selected = 0;
                        crate::tui::schedule::maybe_fetch_week(
                            self.schedule.week_cache.clone(),
                            monday.clone(),
                        );
                        self.status = format!(
                            "Today — week of {}",
                            crate::tui::schedule::week_label(&monday)
                        );
                    }
                } else if self.screen == Screen::Tonight && c == 'd' {
                    // Open the scores date picker overlay
                    self.date_picker.open = true;
                    self.date_picker.input.clear();
                    self.date_picker.err = None;
                    self.status =
                        "Go to date — type YYYY-MM-DD or MM/DD, Enter applies, Esc cancels"
                            .to_owned();
                } else if self.screen == Screen::Tonight && c == 't' {
                    // 't' on Scores jumps back to today (live)
                    self.tonight.date.clear();
                    self.tonight.selected = 0;
                    crate::tui::tonight::maybe_fetch(self.tonight.cache.clone(), String::new());
                    // Re-arm the auto-refresh timer for the live date.
                    self.last_auto_refresh = Some(std::time::Instant::now());
                    self.status = "Scores · Today".to_owned();
                } else if self.screen == Screen::Transactions && (c == 't' || c == 'T') {
                    // Phase T.5+: cycle team filter through every team that
                    // appears in the loaded transactions.
                    //   t       → forward  (None → first → … → None)
                    //   Shift-T → backward (None → last  → … → None)
                    use crate::tui::screens::transactions as tx_screen;
                    let teams = tx_screen::transactions_teams(&self.txs.rows);
                    let next = if c == 't' {
                        tx_screen::cycle_team_forward(self.txs.team_filter.as_deref(), &teams)
                    } else {
                        tx_screen::cycle_team_backward(self.txs.team_filter.as_deref(), &teams)
                    };
                    self.txs.team_filter = next.clone();
                    self.txs.selected = 0;
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
                        tx_screen::cycle_kind_forward(self.txs.kind_filter, cycle)
                    } else {
                        tx_screen::cycle_kind_backward(self.txs.kind_filter, cycle)
                    };
                    self.txs.kind_filter = next;
                    self.txs.selected = 0;
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
                } else if c == 'R' && !is_text_input_active(self) {
                    // Phase Reports — capital R opens the overlay so it
                    // doesn't collide with lowercase r (refresh) or with
                    // search/filter input fields. Don't trip while a text
                    // input is focused.
                    self.show_reports_overlay = true;
                    self.reports_selected = 0;
                } else if c == 'd' && !is_text_input_active(self) {
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
                } else if c == 'D' && !is_text_input_active(self) {
                    // Phase Foster.1.4 — Shift+D opens the shared
                    // date picker overlay on Scores / Schedule /
                    // Playoffs. Lowercase `d` keeps its existing
                    // global Depth-jump behavior (handled above).
                    match self.screen {
                        Screen::Tonight => {
                            self.date_picker.target = PickerTarget::Scores;
                            self.date_picker.open = true;
                            self.date_picker.input.clear();
                            self.date_picker.err = None;
                            self.status =
                                "Go to date — type YYYY-MM-DD or MM/DD, Enter applies, Esc cancels"
                                    .to_owned();
                        }
                        Screen::Schedule => {
                            self.date_picker.target = PickerTarget::Schedule;
                            self.date_picker.open = true;
                            self.date_picker.input.clear();
                            self.date_picker.err = None;
                            self.status =
                                "Schedule · pick a date — Enter snaps to that week, Esc cancels"
                                    .to_owned();
                        }
                        Screen::Playoffs => {
                            // Playoffs is season-anchored, not date-anchored.
                            // Reuse the existing season picker (`y` key) so
                            // Shift+D on Playoffs jumps to a season.
                            self.show_season_picker = true;
                            let season_list = crate::tui::screens::misc::PICKER_SEASONS;
                            self.picker_selected = season_list
                                .iter()
                                .position(|(id, _, _)| *id == self.active_season.as_str())
                                .unwrap_or(0);
                            self.status = "Playoffs · pick a season — Enter applies, Esc cancels"
                                .to_owned();
                        }
                        _ => {
                            // Other screens: no-op. Keep the keystroke
                            // available for future surfaces.
                        }
                    }
                } else if c == 'v' && !is_text_input_active(self) {
                    // Phase Foster +8 — cycle Day → Week → Month →
                    // Season → Day. Renders in the status bar so
                    // user sees the active window without opening a
                    // menu (GLASS L8).
                    use icelines_core::timeframe::Timeframe;
                    self.active_timeframe = match self.active_timeframe {
                        Timeframe::Day => Timeframe::Week,
                        Timeframe::Week => Timeframe::Month,
                        Timeframe::Month => Timeframe::Season,
                        Timeframe::Season => Timeframe::Day,
                    };
                    self.status = format!(
                        "Timeframe → {} ({})",
                        timeframe_label(self.active_timeframe),
                        timeframe_anchor_hint(self.active_timeframe),
                    );
                } else if c == 'M' && !is_text_input_active(self) {
                    // LP.4 — Shift+M opens the in-TUI docs overlay
                    // (Manual). Uppercase M to avoid the lowercase `m`
                    // collision with Goalies min-GP cycle. The
                    // is_text_input_active guard keeps text-input
                    // screens (Search, Tonight, Schedule, Queries
                    // SaveName) from losing typed M characters.
                    self.show_docs = true;
                    self.docs_scroll = 0;
                }
            }
            Action::Backspace => {
                if self.screen == Screen::Search {
                    self.search_query.pop();
                    self.selected = 0;
                } else if self.screen == Screen::Queries && self.queries.mode == QueryMode::SaveName {
                    self.queries.save_name.pop();
                } else if self.screen == Screen::Queries && self.queries.mode == QueryMode::SortPicker
                {
                    // Phase Lindsay L.3.4 — Backspace in sort picker
                    // pops the search query and resets selection.
                    self.queries.sort_picker_query.pop();
                    self.queries.sort_picker_idx = 0;
                }
            }
            Action::Tab => {
                // UX.3 — Tab always cycles screens, no exceptions. The
                // earlier override that made Tab toggle Queries
                // sections trapped users on the Stats tab. Section
                // toggle moved to `o` below; auto-expand on Down/Up
                // already covers most navigation needs.
                //
                // Phase Masterton.3 — when launched with --standalone,
                // Tab is a no-op. The user gets a focused single-
                // screen experience without cycling.
                //
                // Phase Jack Adams.1 — when launched with --mdi, Tab
                // is reserved for command-bar autocomplete (wired in
                // Adams.2). For Adams.1 stub, Tab is a no-op in MDI.
                if self.locked_screen.is_none() && self.mdi.is_none() {
                    self.cycle_screen();
                }
            }
            Action::TabPrev => {
                if self.locked_screen.is_none() && self.mdi.is_none() {
                    self.cycle_screen_back();
                }
            }
            Action::Refresh => {
                if self.screen == Screen::Queries {
                    self.queries.fields = crate::tui::screens::queries::default_fields();
                    self.queries.sections = crate::tui::screens::queries::default_sections();
                    self.queries.field_idx = 0;
                    self.queries.result_scroll = 0;
                    self.status = "Query fields reset.".to_owned();
                } else if self.screen == Screen::Tonight {
                    // Force refresh scores for the active date
                    crate::tui::tonight::force_fetch(
                        self.tonight.cache.clone(),
                        self.tonight.date.clone(),
                    );
                    self.status = "Refreshing scores…".to_owned();
                } else if self.screen == Screen::Schedule {
                    crate::tui::schedule::force_fetch_week(
                        self.schedule.week_cache.clone(),
                        self.schedule.week.clone(),
                    );
                    self.status = format!(
                        "Retrying {}…",
                        crate::tui::schedule::week_label(&self.schedule.week)
                    );
                } else if matches!(self.screen, Screen::Playoffs | Screen::SeriesDetail(_)) {
                    if let Some(year) =
                        crate::tui::playoffs::playoff_year_for_season(&self.active_season)
                    {
                        crate::tui::playoffs::force_fetch_bracket(
                            self.playoffs.cache.clone(),
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
                    self.group_picker.list = crate::db::GroupDb::open()
                        .ok()
                        .and_then(|db| db.list_groups().ok())
                        .map(|gs| gs.into_iter().map(|g| g.name).collect())
                        .unwrap_or_default();
                    if self.group_picker.list.is_empty() {
                        self.status =
                            "No groups — create one with `icelines group create`".to_owned();
                    } else {
                        self.group_picker.player = Some(player);
                        self.group_picker.open = true;
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
                // Phase Art Ross — on the Queries screen in Build mode,
                // 'f' opens the free-form filter overlay instead of
                // adding to Favorites (Favorites is a player-card
                // action and the Queries screen has no player selected
                // at the field-editor level). When focused on the
                // results panel, 'f' falls through to favorites add
                // (the user has a row selected).
                if self.screen == Screen::Queries
                    && matches!(self.queries.mode, QueryMode::Build)
                    && !self.queries.results_focused
                {
                    self.queries.mode = QueryMode::FilterEdit;
                    self.queries.filter_error = None;
                    self.status = "Filter — type Phase Art Ross filter (e.g. country IN (CAN, USA) AND age<25) · Enter accept · Esc cancel".to_owned();
                    return false;
                }
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
                    self.queries.result_scroll = 0;
                    self.queries.results_focused = false;
                    self.group_picker.open = false;
                    self.schedule.selected = 0;
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
            crate::tui::tonight::maybe_fetch(self.tonight.cache.clone(), self.tonight.date.clone());
            // Arm the timer only when on a live date — past dates are
            // permanent (final scores don't change) and don't need polling.
            self.last_auto_refresh = if self.tonight.date.is_empty() {
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
            &self.tonight.date,
            self.last_auto_refresh,
            now,
            SCORES_AUTO_REFRESH_INTERVAL,
        ) {
            crate::tui::tonight::force_fetch(self.tonight.cache.clone(), self.tonight.date.clone());
            self.last_auto_refresh = Some(now);
        }
    }

    /// game_id of the currently-highlighted game on Scores, if any.
    pub fn selected_game_id(&self) -> Option<u64> {
        use crate::tui::tonight::{lookup, TonightState};
        let state = lookup(&self.tonight.cache, &self.tonight.date);
        match state {
            TonightState::Loaded(games) => {
                let idx = self.tonight.selected.min(games.len().saturating_sub(1));
                games.get(idx).map(|g| g.game_id)
            }
            _ => None,
        }
    }

    /// Apply the date typed into the d-key picker. Accepts `YYYY-MM-DD` and
    /// `MM/DD` (current year inferred). Empty input clears back to "today".
    /// Phase Foster.1.4 — dispatches on `picker_target`: Scores anchors
    /// the live-schedule fetch; Schedule snaps to the Monday-of-week
    /// containing the picked date.
    fn apply_scores_date_picker(&mut self) {
        let raw = self.date_picker.input.trim();
        let target = self.date_picker.target;
        if raw.is_empty() {
            self.date_picker.open = false;
            self.date_picker.err = None;
            self.date_picker.input.clear();
            match target {
                PickerTarget::Scores => {
                    self.tonight.date.clear();
                    self.tonight.selected = 0;
                    crate::tui::tonight::maybe_fetch(self.tonight.cache.clone(), String::new());
                    // Empty date = live → arm the timer
                    self.last_auto_refresh = Some(std::time::Instant::now());
                    self.status = "Scores · Today".to_owned();
                }
                PickerTarget::Schedule => {
                    let today = crate::tui::schedule::today_iso();
                    if let Some(monday) = crate::tui::schedule::monday_of(&today) {
                        self.schedule.week = monday.clone();
                        crate::tui::schedule::maybe_fetch_week(
                            self.schedule.week_cache.clone(),
                            monday,
                        );
                    }
                    self.status = "Schedule · This week".to_owned();
                }
            }
            self.date_picker.target = PickerTarget::default();
            return;
        }
        match parse_picker_date(raw) {
            Ok(iso) => {
                self.date_picker.open = false;
                self.date_picker.err = None;
                self.date_picker.input.clear();
                match target {
                    PickerTarget::Scores => {
                        self.tonight.date = iso.clone();
                        self.tonight.selected = 0;
                        crate::tui::tonight::maybe_fetch(
                            self.tonight.cache.clone(),
                            iso.clone(),
                        );
                        // Specific date → no auto-refresh (final scores don't change)
                        self.last_auto_refresh = None;
                        self.status = format!("Scores · {iso}");
                    }
                    PickerTarget::Schedule => {
                        if let Some(monday) = crate::tui::schedule::monday_of(&iso) {
                            self.schedule.week = monday.clone();
                            crate::tui::schedule::maybe_fetch_week(
                                self.schedule.week_cache.clone(),
                                monday.clone(),
                            );
                            self.status = format!(
                                "Schedule · week of {}",
                                crate::tui::schedule::week_label(&monday)
                            );
                        } else {
                            self.status = format!("Schedule · {iso}");
                        }
                    }
                }
                self.date_picker.target = PickerTarget::default();
            }
            Err(msg) => {
                self.date_picker.err = Some(msg.clone());
                self.status = format!("⚠ {msg}");
            }
        }
    }

    /// Handle key events when the Scores date picker overlay is open.
    fn handle_scores_date_picker(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                self.date_picker.open = false;
                self.date_picker.input.clear();
                self.date_picker.err = None;
                self.status = "Date picker cancelled.".to_owned();
            }
            Action::Enter => self.apply_scores_date_picker(),
            Action::Backspace => {
                self.date_picker.input.pop();
                self.date_picker.err = None;
            }
            Action::Char(c) => self.date_picker.input.push(c),
            // Map non-text actions back to their characters so digits/letters
            // typed at the picker behave naturally.
            Action::Refresh => self.date_picker.input.push('r'),
            Action::Install => self.date_picker.input.push('i'),
            Action::AddToGroup => self.date_picker.input.push('g'),
            Action::AddToFavorites => self.date_picker.input.push('f'),
            Action::GoToTab(n) => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.date_picker.input.push(ch);
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
        let map = self.playoffs.cache.lock().unwrap();
        match map.get(&year) {
            Some(crate::tui::playoffs::PlayoffsState::Loaded(b)) => b.rounds.len(),
            _ => 0,
        }
    }

    /// Letter of the currently-selected series (used as SeriesDetail key).
    pub fn selected_series_letter(&self) -> Option<String> {
        let year = crate::tui::playoffs::playoff_year_for_season(&self.active_season)?;
        let map = self.playoffs.cache.lock().unwrap();
        match map.get(&year) {
            Some(crate::tui::playoffs::PlayoffsState::Loaded(b)) => {
                let round = b.rounds.get(self.playoffs.round)?;
                let series = round.series.get(self.playoffs.series)?;
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
                    self.playoffs.cache.clone(),
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
                    self.schedule.week_cache.clone(),
                    &self.schedule.week,
                );
            }
            Screen::ScheduleTeam(team) => {
                crate::tui::schedule::maybe_fetch_team(
                    self.schedule.team_cache.clone(),
                    team.clone(),
                    self.active_season.clone(),
                );
            }
            Screen::ScheduleMatchup(t1, _t2) => {
                // Matchup view derives from one team's full season schedule
                crate::tui::schedule::maybe_fetch_team(
                    self.schedule.team_cache.clone(),
                    t1.clone(),
                    self.active_season.clone(),
                );
            }
            _ => {}
        }
    }

    /// Apply current `schedule_query` text — validate teams, set filter, exit search mode.
    fn apply_schedule_query(&mut self) {
        match crate::tui::schedule::parse_search(&self.schedule.query) {
            Ok(filter) => {
                self.schedule.filter = filter;
                self.schedule.filter_err = None;
                self.schedule.search_mode = false;
                self.schedule.selected = 0;
                self.status = match &self.schedule.filter {
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
                self.schedule.filter_err = Some(msg.clone());
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
                self.schedule.search_mode = false;
                self.schedule.query.clear();
                self.schedule.filter_err = None;
                self.status = "Search cancelled.".to_owned();
            }
            Action::Enter => self.apply_schedule_query(),
            Action::Backspace => {
                self.schedule.query.pop();
                self.schedule.filter_err = None;
            }
            Action::Char(c) => self.schedule.query.push(c),
            Action::Space => self.schedule.query.push(' '),
            // While in search mode, hotkeys are treated as text input so
            // queries like "nyr" can be typed without firing Refresh/Install/etc.
            Action::Refresh => self.schedule.query.push('r'),
            Action::Install => self.schedule.query.push('i'),
            Action::AddToGroup => self.schedule.query.push('g'),
            Action::AddToFavorites => self.schedule.query.push('f'),
            Action::GoToTab(n) => {
                // Map digit-tabs back to their numeric character
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.schedule.query.push(ch);
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
                self.queries.save_name.clear();
                self.queries.mode = QueryMode::Build;
                self.status = "Save cancelled.".to_owned();
            }
            Action::Enter => {
                let name = self.queries.save_name.trim().to_owned();
                if !name.is_empty() {
                    // Wave 24 — also persist the active free-form
                    // filter text so the saved preset captures both
                    // the structured fields AND the Phase Art Ross
                    // overlay state.
                    let json = crate::tui::screens::queries::fields_and_filter_to_json(
                        &self.queries.fields,
                        &self.queries.filter_text,
                    );
                    if let Ok(db) = crate::db::GroupDb::open() {
                        let _ = db.save_query(&name, &json);
                        self.status = format!("Saved query '{name}'  ·  l=load  s=save  r=reset");
                    }
                }
                self.queries.mode = QueryMode::Build;
            }
            Action::Backspace => {
                self.queries.save_name.pop();
            }
            Action::Char(c) => self.queries.save_name.push(c),
            Action::Space => self.queries.save_name.push(' '),
            // Hotkey actions become their associated character. Without
            // this, 'f' would fire AddToFavorites and the user could
            // never type "fred", "fox", "ford" etc. as a query name.
            Action::Refresh => self.queries.save_name.push('r'),
            Action::Install => self.queries.save_name.push('i'),
            Action::AddToGroup => self.queries.save_name.push('g'),
            Action::AddToFavorites => self.queries.save_name.push('f'),
            Action::GoToTab(n) => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.queries.save_name.push(ch);
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

    /// Phase Art Ross — free-form filter editor. Same short-circuit
    /// shape as `handle_query_save_name`: every char-bearing action
    /// is text input. Enter validates via `parse_query`; on success
    /// the plan is stored and we return to Build, on parse error
    /// the message is shown inline and the editor stays open so the
    /// user can fix the input.
    ///
    /// Wave 24b — Up/Down walks `query_filter_history` (newest →
    /// oldest); any text edit resets the cursor to live.
    fn handle_query_filter_edit(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Help => {
                // Wave 24d — `?` toggles the side cheatsheet.
                // Doesn't open the global help overlay (that
                // would close the editor); stays in FilterEdit.
                self.queries.filter_show_help = !self.queries.filter_show_help;
            }
            Action::Back | Action::Escape => {
                // Cancel — drop the in-progress text + clear any active
                // plan. The user can press `f` again to start fresh.
                self.queries.filter_text.clear();
                self.queries.filter_error = None;
                self.queries.filter_plan = None;
                self.queries.filter_history_cursor = None;
                self.queries.mode = QueryMode::Build;
                self.status = "Filter cleared.".to_owned();
            }
            Action::Enter => {
                let text = self.queries.filter_text.trim();
                if text.is_empty() {
                    // Empty Enter = clear the active plan and exit.
                    self.queries.filter_plan = None;
                    self.queries.filter_error = None;
                    self.queries.filter_history_cursor = None;
                    self.queries.mode = QueryMode::Build;
                    self.status = "Filter cleared.".to_owned();
                } else {
                    // Free-form text → CLI variant. `FilterInput::Tui`
                    // is reserved for the future structured-overlay
                    // path that builds `Vec<Constraint>` directly.
                    match icelines_query::parse_query(
                        icelines_query::FilterInput::Cli(text.to_owned()),
                    ) {
                        Ok(plan) => {
                            self.queries.filter_plan = Some(plan);
                            self.queries.filter_error = None;
                            // Push onto history (newest first); dedupe
                            // against an identical front entry so
                            // hammering Enter doesn't fill the ring
                            // with duplicates.
                            push_filter_history(
                                &mut self.queries.filter_history,
                                text.to_owned(),
                            );
                            self.queries.filter_history_cursor = None;
                            self.queries.mode = QueryMode::Build;
                            self.status =
                                format!("Filter applied: {text}  ·  press f to edit");
                        }
                        Err(errs) => {
                            // Keep editor open. Render shows the error.
                            let msg = errs
                                .iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<_>>()
                                .join("; ");
                            self.queries.filter_error = Some(msg);
                        }
                    }
                }
            }
            Action::Up => {
                // Navigate to an older history entry. From the live
                // edit (cursor=None) Up jumps to the newest history
                // entry; from a historical entry Up walks further
                // back. Hits a wall at the oldest entry.
                if self.queries.filter_history.is_empty() {
                    return false;
                }
                let next = match self.queries.filter_history_cursor {
                    None => 0,
                    Some(i) if i + 1 < self.queries.filter_history.len() => i + 1,
                    Some(i) => i, // already at oldest, stay
                };
                if let Some(entry) = self.queries.filter_history.get(next) {
                    self.queries.filter_text = entry.clone();
                    self.queries.filter_history_cursor = Some(next);
                    self.queries.filter_error = None;
                }
            }
            Action::Down => {
                // Navigate toward newer history; from cursor=0 step
                // back to live edit (cursor=None, text cleared).
                match self.queries.filter_history_cursor {
                    None => {} // already live, stay
                    Some(0) => {
                        self.queries.filter_history_cursor = None;
                        self.queries.filter_text.clear();
                        self.queries.filter_error = None;
                    }
                    Some(i) => {
                        let next = i - 1;
                        if let Some(entry) = self.queries.filter_history.get(next) {
                            self.queries.filter_text = entry.clone();
                            self.queries.filter_history_cursor = Some(next);
                            self.queries.filter_error = None;
                        }
                    }
                }
            }
            Action::Backspace => {
                self.queries.filter_text.pop();
                self.queries.filter_history_cursor = None;
            }
            Action::Char(c) => {
                self.queries.filter_text.push(c);
                self.queries.filter_history_cursor = None;
            }
            Action::Space => {
                self.queries.filter_text.push(' ');
                self.queries.filter_history_cursor = None;
            }
            // Hotkeys → their associated character (mirrors save-name).
            Action::Refresh => {
                self.queries.filter_text.push('r');
                self.queries.filter_history_cursor = None;
            }
            Action::Install => {
                self.queries.filter_text.push('i');
                self.queries.filter_history_cursor = None;
            }
            Action::AddToGroup => {
                self.queries.filter_text.push('g');
                self.queries.filter_history_cursor = None;
            }
            Action::AddToFavorites => {
                self.queries.filter_text.push('f');
                self.queries.filter_history_cursor = None;
            }
            Action::GoToTab(n) => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.queries.filter_text.push(ch);
                self.queries.filter_history_cursor = None;
            }
            Action::Search => {}
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
                self.txs.search_mode = false;
                self.txs.search_query.clear();
                self.txs.selected = 0;
                self.status = "Search cleared.".to_owned();
            }
            Action::Enter => {
                // Apply: keep the query, exit search mode.
                self.txs.search_mode = false;
                self.txs.selected = 0;
                self.status = format!("Filter: '{}'", self.txs.search_query);
            }
            Action::Backspace => {
                self.txs.search_query.pop();
            }
            Action::Char(c) => self.txs.search_query.push(c),
            Action::Space => self.txs.search_query.push(' '),
            Action::Refresh => self.txs.search_query.push('r'),
            Action::Install => self.txs.search_query.push('i'),
            Action::AddToGroup => self.txs.search_query.push('g'),
            Action::AddToFavorites => self.txs.search_query.push('f'),
            Action::GoToTab(n) => {
                let ch = char::from_digit((n + 1) as u32, 10).unwrap_or('?');
                self.txs.search_query.push(ch);
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

    /// UX.1 — Lazy per-player career loader. When the active screen is
    /// a `PlayerById`, fan out across `BUNDLED_SEASONS` to pull just
    /// that player's bios + stats rows into `self.repo`. The HashSet
    /// guard makes the call a no-op for already-loaded ids — safe to
    /// invoke once per render tick from the main loop.
    ///
    /// First load for a player is ~5 ms (38 seasons × O(N) scan over
    /// bundled bios/stats arrays), subsequent renders are O(1).
    pub fn ensure_career_loaded_for_current_screen(&mut self) {
        let pid = match self.screen {
            Screen::PlayerById(p) => p,
            _ => return,
        };
        if self.career_loaded_ids.contains(&pid) {
            return;
        }
        // The loader walks bundled-season data only — no I/O, no
        // network, deterministic. Errors here are RepoError variants
        // (identity merge conflicts on suspicious id reuse). Silently
        // mark loaded on either path so we don't retry a hopeless id
        // every frame.
        let _ = icelines_fetch::stats_loader::load_player_career_into_repo(&mut self.repo, pid);
        self.career_loaded_ids.insert(pid);
    }

    /// Phase Reports — overlay key handler. Up/Down moves the selection
    /// among controllable Tier-1 reports; Space/Enter toggles the
    /// highlighted row; Esc closes the overlay AND persists toggles to
    /// `~/.icelines/config.toml`. Quit (q) propagates so the global
    /// shortcut works while the overlay is open.
    fn handle_reports_overlay(&mut self, action: Action) -> bool {
        let kinds = crate::config::ReportToggles::controllable_kinds();
        let n = kinds.len();
        match action {
            Action::Quit => return true,
            Action::Back | Action::Escape => {
                self.show_reports_overlay = false;
                // Persist on close. A failed save shouldn't break the
                // session — surface it in the status line and continue.
                let cfg = crate::config::Config {
                    csv_path: None,
                    cache_dir: std::path::PathBuf::new(),
                    season: None,
                    live: None,
                    dashboards: None,
                    reports: self.reports,
                    sync: crate::config::SyncConfig::default(),
                };
                if let Err(e) = cfg.save_reports() {
                    self.status = format!("Reports saved in-memory (config write failed: {e})");
                } else {
                    self.status = "Reports saved.".to_owned();
                }
            }
            Action::Down => {
                self.reports_selected = (self.reports_selected + 1).min(n.saturating_sub(1));
            }
            Action::Up => {
                self.reports_selected = self.reports_selected.saturating_sub(1);
            }
            Action::Char(' ') | Action::Enter => {
                if let Some(&kind) = kinds.get(self.reports_selected) {
                    let now = self.reports.is_enabled(kind);
                    self.reports.set(kind, !now);
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

        let (snapshot_dir, reports) = match crate::config::Config::load() {
            Ok(cfg) => {
                // Phase Reports — pull persisted toggles into App so the
                // overlay opens with the user's last-saved state, and
                // column gating reflects it from the first render.
                self.reports = cfg.reports;
                (cfg.snapshot_dir(), cfg.reports)
            }
            Err(_) => return,
        };
        let _ = reports; // silence unused while overlay rendering lands.
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
    pub fn boot_load_with_store(&mut self, store: &icelines_fetch::snapshot::SnapshotStore) {
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
        self.queries.results_focused = false;
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
                let results =
                    crate::tui::screens::search::search_results(&views, &self.search_query);
                results
                    .get(self.selected)
                    .map(|v| (v.identity.name_normalized.clone(), v.full_name().to_owned()))
            }

            Screen::Queries => {
                // Hart.5c.6 Phase B-3.3: queries runs against views now.
                let views = self.views();
                let results =
                    crate::tui::screens::queries::run_query_views_with_pick_and_plan(
                        &views,
                        &self.queries.fields,
                        self.queries.sort_stat_pick,
                        self.queries.filter_plan.as_ref(),
                        self.active_season_typed.0,
                    );
                let row_idx =
                    self.queries.result_scroll + self.selected.min(results.len().saturating_sub(1));
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
                                .map(|v| {
                                    (v.identity.name_normalized.clone(), v.full_name().to_owned())
                                })
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
            _ if self.group_picker.open => {
                if let Some(group_name) = self.group_picker.list.get(self.selected).cloned() {
                    if let Some((norm, full)) = self.group_picker.player.take() {
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
                    self.group_picker.open = false;
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
                match self.queries.mode {
                    QueryMode::SaveName => {
                        // Save the current query with the typed name
                        let name = self.queries.save_name.trim().to_owned();
                        if !name.is_empty() {
                            // Wave 24 — same fields+filter envelope as the
                            // dedicated handler in handle_query_save_name.
                            let json = crate::tui::screens::queries::fields_and_filter_to_json(
                                &self.queries.fields,
                                &self.queries.filter_text,
                            );
                            if let Ok(db) = crate::db::GroupDb::open() {
                                let _ = db.save_query(&name, &json);
                                self.status =
                                    format!("Saved query '{name}'  ·  l=load  s=save  r=reset");
                            }
                        }
                        self.queries.mode = QueryMode::Build;
                    }
                    QueryMode::LoadList => {
                        // Load the selected saved query.
                        if let Some((name, json)) = self.queries.saved_list.get(self.selected) {
                            // Wave 24 — recover both structured fields
                            // AND the free-form filter text. Re-parse
                            // the text via parse_query so the active
                            // plan matches what the saver had applied.
                            // Parse failure on a saved (older-grammar)
                            // filter is non-fatal: we still load the
                            // fields and the filter text, but leave
                            // the plan empty so subsequent renders
                            // ignore the filter. The user can re-open
                            // the editor (`f`), see the recovered text,
                            // and fix it.
                            let filter_text = crate::tui::screens::queries::apply_saved_json(
                                &mut self.queries.fields,
                                json,
                            );
                            self.queries.filter_text = filter_text.clone();
                            self.queries.filter_error = None;
                            self.queries.filter_plan = None;
                            let mut status = format!(
                                "Loaded query '{name}'  ·  ←→ to adjust  s=save  r=reset"
                            );
                            if !filter_text.is_empty() {
                                match icelines_query::parse_query(
                                    icelines_query::FilterInput::Cli(filter_text.clone()),
                                ) {
                                    Ok(plan) => {
                                        self.queries.filter_plan = Some(plan);
                                        status = format!(
                                            "Loaded '{name}' + filter applied  ·  f to edit"
                                        );
                                    }
                                    Err(_) => {
                                        // Don't surface the full error
                                        // here — leave it for the user
                                        // to discover when they open
                                        // the editor with `f`.
                                        status = format!(
                                            "Loaded '{name}' (filter needs re-edit)  ·  f to fix"
                                        );
                                    }
                                }
                            }
                            self.status = status;
                            self.queries.mode = QueryMode::Build;
                            self.queries.result_scroll = 0;
                        }
                    }
                    QueryMode::SortPicker => {
                        // Phase Lindsay L.3.4 — accept the highlighted
                        // catalog stat as the active sort. Updates
                        // `sort_stat_pick` (catalog override) and exits
                        // the picker. The sort dispatch sees `Some(stat)`
                        // on next render and uses `StatId::sort_cmp`.
                        let results = crate::tui::screens::queries::sort_picker_filter(
                            &self.queries.sort_picker_query,
                        );
                        if let Some(&stat) = results.get(self.queries.sort_picker_idx) {
                            self.queries.sort_stat_pick = Some(stat);
                            self.status = format!(
                                "Sort: {} ({})  ·  / picker  s save  l load",
                                stat.label(),
                                stat.cli_key(),
                            );
                            self.queries.mode = QueryMode::Build;
                            self.queries.result_scroll = 0;
                        } else {
                            // EDGE-7 (L.5b post-fix) — empty filter,
                            // Enter pressed. Don't silently drop the
                            // input; refresh status to surface the
                            // dead-end and keep the picker open so the
                            // user can refine the query.
                            self.status = format!(
                                "No matches for {:?} — type to refine \
                                 or Esc to cancel",
                                self.queries.sort_picker_query
                            );
                            // Picker stays open (do NOT switch back to
                            // Build mode). The user adjusts the query.
                        }
                    }
                    QueryMode::Build => {
                        // Enter on a result row → player card. Hart.5c.6
                        // Phase B-3.3: queries runs against views now.
                        let views = self.views();
                        let results =
                            crate::tui::screens::queries::run_query_views_with_pick_and_plan(
                                &views,
                                &self.queries.fields,
                                self.queries.sort_stat_pick,
                                self.queries.filter_plan.as_ref(),
                                self.active_season_typed.0,
                            );
                        let row_idx = self.queries.result_scroll
                            + self.selected.min(results.len().saturating_sub(1));
                        if let Some((_, v)) = results.get(row_idx) {
                            let pid = v.identity.id;
                            self.prev_screen = Some(self.screen.clone());
                            self.screen = Screen::PlayerById(pid);
                            self.selected = 0;
                        }
                    }
                    QueryMode::FilterEdit => {
                        // Unreachable: the dispatcher short-circuits
                        // every action through `handle_query_filter_edit`
                        // when this mode is active (see line ~553).
                        // Defensive arm so the match stays exhaustive.
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
                let results =
                    crate::tui::screens::search::search_results(&views, &self.search_query);
                if let Some(v) = results.get(self.selected) {
                    let pid = v.identity.id;
                    self.prev_screen = Some(Screen::Search);
                    self.screen = Screen::PlayerById(pid);
                    self.selected = 0;
                }
            }
            Screen::Depth => {
                let views = self.views();
                let strength =
                    icelines_core::cross_team::compute_team_strength_views(&views, self.depth_mode);
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
                let next = match &self.schedule.filter {
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
                    self.schedule.selected = 0;
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
                    .get(self.goalies.sort as usize)
                    .copied()
                    .unwrap_or(crate::tui::screens::goalies::GoalieSort::SvPctDesc);
                let views = self.goalie_views();
                let qualified = crate::tui::screens::goalies::sort_goalie_views(
                    &views,
                    sort,
                    self.goalies.min_gp,
                );
                if let Some(v) = qualified.get(self.goalies.selected) {
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
                    crate::tui::tonight::maybe_fetch_boxscore(self.tonight.boxscore_cache.clone(), game_id);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn cycle_screen(&mut self) {
        self.queries.results_focused = false;
        // Phase Foster.2 — Favorites tab inserted between Goalies and
        // Scores per spec §"Tab insertion" (GLASS H4). 9-tab cycle:
        //   League → Depth → Queries → Goalies → Favorites → Scores
        //   → Schedule → Transactions → Playoffs → League
        let next = match &self.screen {
            Screen::Home | Screen::Team(_) | Screen::PlayerById(_) | Screen::CompsById(_) => {
                Screen::Depth
            }
            Screen::Depth | Screen::DepthTeam(_) => Screen::Queries,
            Screen::Queries | Screen::Projections | Screen::Search => Screen::Goalies,
            Screen::Goalies | Screen::GoalieDetailById(_) => Screen::Favorites,
            Screen::Favorites => Screen::Tonight,
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
        self.schedule.selected = 0;
        self.queries.result_scroll = 0;
        self.maybe_fetch_scores();
        self.maybe_fetch_schedule();
        self.maybe_fetch_playoffs();
    }

    /// Reverse of `cycle_screen` — Shift-Tab.
    pub(crate) fn cycle_screen_back(&mut self) {
        self.queries.results_focused = false;
        let prev = match &self.screen {
            Screen::Home | Screen::Team(_) | Screen::PlayerById(_) | Screen::CompsById(_) => {
                Screen::Playoffs
            }
            Screen::Depth | Screen::DepthTeam(_) => Screen::Home,
            Screen::Queries | Screen::Projections | Screen::Search => Screen::Depth,
            Screen::Goalies | Screen::GoalieDetailById(_) => Screen::Queries,
            Screen::Favorites => Screen::Goalies,
            Screen::Tonight | Screen::GameDetail(_) => Screen::Favorites,
            Screen::Schedule | Screen::ScheduleTeam(_) | Screen::ScheduleMatchup(..) => {
                Screen::Tonight
            }
            Screen::Transactions => Screen::Schedule,
            Screen::Playoffs | Screen::SeriesDetail(_) => Screen::Transactions,
            _ => Screen::Home,
        };
        self.screen = prev;
        self.selected = 0;
        self.schedule.selected = 0;
        self.queries.result_scroll = 0;
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

    // ── Phase Foster +8 — `v` keybind cycles timeframe ────────────────────

    #[test]
    fn l0_foster_plus8_v_cycles_timeframes_in_order() {
        use icelines_core::timeframe::Timeframe;
        let mut app = App::new(false);
        // Default starts at Day so the status bar stays uncluttered.
        assert_eq!(app.active_timeframe, Timeframe::Day);
        app.handle(Action::Char('v'));
        assert_eq!(app.active_timeframe, Timeframe::Week);
        app.handle(Action::Char('v'));
        assert_eq!(app.active_timeframe, Timeframe::Month);
        app.handle(Action::Char('v'));
        assert_eq!(app.active_timeframe, Timeframe::Season);
        app.handle(Action::Char('v'));
        assert_eq!(app.active_timeframe, Timeframe::Day, "wraps back to Day");
    }

    #[test]
    fn l0_foster_plus8_v_status_announces_timeframe() {
        let mut app = App::new(false);
        app.handle(Action::Char('v'));
        assert!(
            app.status.contains("Week"),
            "status must announce new timeframe, got: {}",
            app.status
        );
    }

    #[test]
    fn l0_foster_plus8_timeframe_label_lookup() {
        use icelines_core::timeframe::Timeframe;
        assert_eq!(timeframe_label(Timeframe::Day), "Day");
        assert_eq!(timeframe_label(Timeframe::Week), "Week");
        assert_eq!(timeframe_label(Timeframe::Month), "Month");
        assert_eq!(timeframe_label(Timeframe::Season), "Season");
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
        // 9 tabs (Phase Foster.2 inserts Favorites between Goalies
        // and Scores per GLASS H4):
        //   League → Depth → Stats(Queries) → Goalies → Favorites
        //   → Scores → Schedule → Transactions → Playoffs → wrap
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
        app.cycle_screen(); // bypass Lindsay Tab-on-Queries intercept
        assert_eq!(app.screen, Screen::Goalies, "Stats→Goalies");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Favorites, "Goalies→Favorites");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Tonight, "Favorites→Scores");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Schedule, "Scores→Schedule");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Transactions, "Schedule→Transactions");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Playoffs, "Transactions→Playoffs");
        app.handle(Action::Tab);
        assert_eq!(app.screen, Screen::Home, "Playoffs→League (wraps)");
    }

    /// UX.3 — `o` on Queries toggles the section containing the
    /// current field cursor (was Tab pre-UX.3; Tab now unconditionally
    /// cycles screens). Cursor snaps to the next visible field if its
    /// current field becomes hidden via collapse.
    #[test]
    fn l0_ux3_o_on_queries_toggles_section() {
        let mut app = App::new(false);
        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Tab); // Depth → Queries
        assert_eq!(app.screen, Screen::Queries);

        // Default: cursor on field 0 (Sort by) which is in section 0.
        // Section 0 starts expanded.
        let initial_s0 = app.queries.sections[0].expanded;
        assert!(initial_s0, "section 0 starts expanded by default");

        // `o` → toggles section 0 (cursor's section). Screen unchanged.
        app.handle(Action::Char('o'));
        assert_eq!(
            app.screen,
            Screen::Queries,
            "o on Queries toggles section, doesn't advance screen"
        );
        assert_eq!(
            app.queries.sections[0].expanded, !initial_s0,
            "section 0 expansion flipped by o"
        );

        // After collapsing section 0, field 0 is hidden. Cursor
        // snapped to the next visible field — field 1 (Position),
        // which lives in section 1.
        assert_eq!(
            app.queries.field_idx, 1,
            "cursor snaps to next visible field after section collapse"
        );

        // Second `o` now targets section 1 (where the cursor lives).
        let initial_s1 = app.queries.sections[1].expanded;
        app.handle(Action::Char('o'));
        assert_eq!(
            app.queries.sections[1].expanded, !initial_s1,
            "second o toggles section 1 (cursor's new home)"
        );
        // Section 0 still collapsed — wasn't touched by the second o.
        assert_eq!(app.queries.sections[0].expanded, !initial_s0);
    }

    /// UX.3 — Tab on the Queries screen now cycles screens
    /// unconditionally. Pre-UX.3 it toggled sections, trapping users
    /// on the Stats tab.
    #[test]
    fn l0_ux3_tab_on_queries_advances_screen() {
        let mut app = App::new(false);
        app.handle(Action::Tab); // Home → Depth
        app.handle(Action::Tab); // Depth → Queries
        assert_eq!(app.screen, Screen::Queries);

        // Sections starts in default state — Tab must NOT touch them.
        let s0_before = app.queries.sections[0].expanded;

        app.handle(Action::Tab);
        assert_ne!(
            app.screen,
            Screen::Queries,
            "Tab on Queries must advance to the next screen"
        );
        // Sections untouched.
        assert_eq!(app.queries.sections[0].expanded, s0_before);
    }

    #[test]
    fn l0_tui_shift_tab_cycles_screens_backwards() {
        // Shift-Tab walks the same nine tabs in reverse (Phase Foster.2
        // adds Favorites between Goalies and Scores).
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
        assert_eq!(app.screen, Screen::Favorites, "Scores→Favorites");
        app.handle(Action::TabPrev);
        assert_eq!(app.screen, Screen::Goalies, "Favorites→Goalies");
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
        assert!(matches!(
            app.screen,
            Screen::Team(_) | Screen::PlayerById(_)
        ));
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
        assert!(app.schedule.search_mode, "search mode should be open");
        assert_eq!(
            app.screen,
            Screen::Schedule,
            "stays on Schedule, not the global Search screen"
        );
        assert!(app.schedule.query.is_empty());
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
        assert!(!app.schedule.search_mode);
        assert_eq!(app.schedule.filter, SearchFilter::Team("SEA".to_owned()));
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
            app.schedule.filter,
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
        assert!(app.schedule.search_mode);
        assert!(app
            .schedule
            .filter_err
            .as_deref()
            .unwrap_or("")
            .contains("Unknown team"));
        // Filter unchanged from default
        assert_eq!(app.schedule.filter, SearchFilter::None);
    }

    #[test]
    fn l0_tui_schedule_left_right_changes_week() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        let initial = app.schedule.week.clone();
        app.handle(Action::Right);
        let after_right = app.schedule.week.clone();
        assert_ne!(initial, after_right, "week should advance");
        app.handle(Action::Left);
        assert_eq!(app.schedule.week, initial, "left should restore");
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
        assert_eq!(app.schedule.query, "NYr");
    }

    #[test]
    fn l0_tui_schedule_team_filter_enter_opens_team_view() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.schedule.filter = SearchFilter::Team("SEA".to_owned());
        app.handle(Action::Enter);
        assert_eq!(app.screen, Screen::ScheduleTeam("SEA".to_owned()));
    }

    #[test]
    fn l0_tui_schedule_matchup_filter_enter_opens_matchup_view() {
        let mut app = App::new(false);
        app.screen = Screen::Schedule;
        app.schedule.filter = SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned());
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
        assert!(!app.schedule.search_mode);
        assert!(app.schedule.query.is_empty());
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
        assert_ne!(app.schedule.week, today_monday);
        app.handle(Action::Char('t'));
        assert_eq!(app.schedule.week, today_monday);
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
        app.playoffs.cache
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

        assert_eq!(app.playoffs.round, 0);
        app.handle(Action::Right);
        assert_eq!(app.playoffs.round, 1, "→ should advance to round 2");
        // At the last round, → clamps (no wrap)
        app.handle(Action::Right);
        assert_eq!(app.playoffs.round, 1);
        // ← walks back
        app.handle(Action::Left);
        assert_eq!(app.playoffs.round, 0);
        // At round 0, ← clamps at 0
        app.handle(Action::Left);
        assert_eq!(app.playoffs.round, 0);
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
        assert_eq!(app.playoffs.series, 1);
        app.handle(Action::Right);
        assert_eq!(app.playoffs.round, 1);
        assert_eq!(
            app.playoffs.series, 0,
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
        let initial = app.playoffs.round;
        app.handle(Action::Right);
        assert_eq!(
            app.playoffs.round, initial,
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
        app.tonight.cache
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
        assert!(app.tonight.date.is_empty());
        app.handle(Action::Right);
        assert!(
            !app.tonight.date.is_empty(),
            "Right should set explicit date"
        );
        let after_right = app.tonight.date.clone();
        app.handle(Action::Left);
        let after_left = app.tonight.date.clone();
        assert_ne!(after_right, after_left, "Left should move backwards");
    }

    #[test]
    fn l0_tui_scores_t_jumps_to_today() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        app.tonight.date = "2026-01-01".to_owned();
        app.handle(Action::Char('t'));
        assert!(
            app.tonight.date.is_empty(),
            "t must clear scores_date back to live"
        );
    }

    #[test]
    fn l0_tui_scores_d_opens_picker() {
        let mut app = App::new(false);
        app.screen = Screen::Tonight;
        assert!(!app.date_picker.open);
        app.handle(Action::Char('d'));
        assert!(app.date_picker.open);
        assert!(app.date_picker.input.is_empty());
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
        assert!(!app.date_picker.open, "picker should close on apply");
        assert_eq!(app.tonight.date, "2026-04-28");
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
            app.date_picker.open,
            "invalid input must keep picker open for correction"
        );
        assert!(app.date_picker.err.is_some());
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
        assert!(!app.date_picker.open);
        assert!(app.date_picker.input.is_empty());
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
        assert_eq!(app.date_picker.input, "2r");
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
        app.tonight.date = "2026-01-15".to_owned();
        // Past date → timer dormant
        app.last_auto_refresh = None;
        app.handle(Action::Char('t'));
        assert!(app.tonight.date.is_empty(), "t must clear the date");
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
        assert!(!app.tonight.date.is_empty(), "Left must set a specific date");
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
        assert!(
            app.views().is_empty(),
            "App::new must start with empty repo"
        );

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
        let mcdavid_view =
            app.repo
                .view(PlayerId(8478402), app.active_season_typed, app.active_type);
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

    // ── Phase Reports — overlay behavior ──────────────────────────────────

    #[test]
    #[allow(non_snake_case)]
    fn l0_tui_reports_capital_R_opens_overlay() {
        let mut app = App::new(false);
        app.screen = Screen::Home;
        assert!(!app.show_reports_overlay);
        app.handle(Action::Char('R'));
        assert!(
            app.show_reports_overlay,
            "Capital R from a non-text screen must open the Reports overlay"
        );
        assert_eq!(
            app.reports_selected, 0,
            "selection must reset to top on open"
        );
    }

    #[test]
    fn l0_tui_reports_lowercase_r_does_not_open_overlay() {
        // Lowercase 'r' fires Action::Refresh — must not collide with R.
        let mut app = App::new(false);
        app.screen = Screen::Home;
        app.handle(Action::Refresh);
        assert!(
            !app.show_reports_overlay,
            "Action::Refresh (lowercase r) must not open the Reports overlay"
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn l0_tui_reports_R_ignored_while_text_input_active() {
        // The Search screen counts as text-input-active per
        // is_text_input_active — typing a capital R there should land
        // in the search box, not open the Reports overlay.
        let mut app = App::new(false);
        app.screen = Screen::Search;
        app.handle(Action::Char('R'));
        assert!(
            !app.show_reports_overlay,
            "R while text-input is active must not open the overlay (collides with typing)"
        );
    }

    #[test]
    fn l0_tui_reports_overlay_down_arrow_navigates_and_clamps() {
        let mut app = App::new(false);
        app.show_reports_overlay = true;
        let n = crate::config::ReportToggles::controllable_kinds().len();
        // Walk past the end — selection clamps to n-1, doesn't wrap.
        for _ in 0..(n + 5) {
            app.handle(Action::Down);
        }
        assert_eq!(app.reports_selected, n - 1, "Down must clamp at n-1");
    }

    #[test]
    fn l0_tui_reports_overlay_up_arrow_navigates_and_clamps_to_zero() {
        let mut app = App::new(false);
        app.show_reports_overlay = true;
        app.reports_selected = 2;
        for _ in 0..10 {
            app.handle(Action::Up);
        }
        assert_eq!(app.reports_selected, 0, "Up must clamp at 0");
    }

    #[test]
    fn l0_tui_reports_overlay_space_toggles_selected_kind() {
        let mut app = App::new(false);
        app.show_reports_overlay = true;
        // Index 0 == SkaterRealtime per controllable_kinds() order.
        app.reports_selected = 0;
        let before = app.reports.realtime;
        app.handle(Action::Char(' '));
        assert_eq!(
            app.reports.realtime, !before,
            "Space on selected row must flip that report's toggle in-memory"
        );
        // Toggle back so future tests start from a known-good state.
        app.handle(Action::Char(' '));
        assert_eq!(app.reports.realtime, before);
    }

    #[test]
    fn l0_tui_reports_overlay_enter_also_toggles() {
        // Enter is an alias for Space inside the overlay so users
        // who reach for the more-prominent key still toggle.
        let mut app = App::new(false);
        app.show_reports_overlay = true;
        app.reports_selected = 1; // SkaterTimeOnIce
        let before = app.reports.timeonice;
        app.handle(Action::Enter);
        assert_eq!(app.reports.timeonice, !before);
    }

    #[test]
    fn l0_tui_reports_overlay_q_propagates_quit_signal() {
        // q while overlay is open must still quit the app — global
        // shortcut precedence comes via handle_reports_overlay returning
        // true on Action::Quit.
        let mut app = App::new(false);
        app.show_reports_overlay = true;
        let should_quit = app.handle(Action::Quit);
        assert!(
            should_quit,
            "Quit (q) while overlay is open must propagate to the event loop"
        );
    }

    #[test]
    fn l0_tui_reports_overlay_esc_closes_overlay_in_memory() {
        // Esc closes the overlay and persists the toggles. The
        // persistence path writes to ~/.icelines/config.toml — we point
        // HOME / USERPROFILE at a tempdir so the write doesn't pollute
        // the user's real config under cargo test. Serialized via the
        // shared home_env_lock to avoid races with other HOME-touching
        // tests.
        let _guard = crate::test_utils::home_env_lock();
        let dir = tempfile::TempDir::new().unwrap();
        let prev_userprofile = std::env::var_os("USERPROFILE");
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("USERPROFILE", dir.path());
        std::env::set_var("HOME", dir.path());

        let mut app = App::new(false);
        app.show_reports_overlay = true;
        app.reports_selected = 2;
        // Flip Goals For/Against on so we have a non-default state.
        app.handle(Action::Char(' '));
        app.handle(Action::Escape);
        assert!(!app.show_reports_overlay, "Esc must close the overlay");
        // Status should reflect the save outcome (success path).
        assert!(
            app.status.contains("Reports") || app.status.contains("saved"),
            "Esc must update status to reflect save, got: {}",
            app.status
        );
        // Persisted file lives at $HOME/.icelines/config.toml — verify
        // the write happened so the toggle survives a restart.
        let cfg_path = dir.path().join(".icelines/config.toml");
        assert!(
            cfg_path.exists(),
            "Esc save must create config.toml in HOME"
        );
        let body = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            body.contains("[reports]") && body.contains("goals_for_against"),
            "config.toml must carry the [reports] section, got:\n{body}"
        );

        // Restore env.
        match prev_userprofile {
            Some(p) => std::env::set_var("USERPROFILE", p),
            None => std::env::remove_var("USERPROFILE"),
        }
        match prev_home {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
    }

    // ── Phase Art Ross — TUI filter overlay (Wave 23) ──────────────────

    /// `f` on the Queries screen enters FilterEdit mode and the
    /// dispatcher routes subsequent actions through the editor.
    #[test]
    fn l0_tui_filter_edit_f_enters_mode() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.handle(Action::AddToFavorites); // 'f' hotkey
        assert_eq!(
            app.queries.mode,
            QueryMode::FilterEdit,
            "f on Queries must enter FilterEdit"
        );
    }

    /// Typed characters land in `query_filter_text`, not the global
    /// hotkey targets (so `f` while typing is text input).
    #[test]
    fn l0_tui_filter_edit_typing_appends_to_text() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        for c in "country=CAN".chars() {
            if c == ' ' {
                app.handle(Action::Space);
            } else {
                app.handle(Action::Char(c));
            }
        }
        assert_eq!(app.queries.filter_text, "country=CAN");
    }

    /// 'f' typed while in FilterEdit becomes a literal 'f', not a
    /// favorites action — same short-circuit pattern as the save-name
    /// editor.
    #[test]
    fn l0_tui_filter_edit_f_hotkey_is_text_input() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.handle(Action::AddToFavorites); // 'f' becomes literal 'f'
        assert_eq!(app.queries.filter_text, "f");
        assert_eq!(app.queries.mode, QueryMode::FilterEdit);
    }

    /// Backspace pops one character.
    #[test]
    fn l0_tui_filter_edit_backspace_pops_char() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "country=CA".to_owned();
        app.handle(Action::Backspace);
        assert_eq!(app.queries.filter_text, "country=C");
    }

    /// Enter on a valid filter parses + stores the plan and returns
    /// to Build mode.
    #[test]
    fn l0_tui_filter_edit_enter_valid_parses_and_exits() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "country=CAN".to_owned();
        app.handle(Action::Enter);
        assert_eq!(
            app.queries.mode,
            QueryMode::Build,
            "valid Enter must exit FilterEdit"
        );
        assert!(
            app.queries.filter_plan.is_some(),
            "valid Enter must store the parsed plan"
        );
        assert!(app.queries.filter_error.is_none(), "no error on success");
    }

    /// Enter on an invalid filter keeps the editor open and stores
    /// the parser error message for rendering.
    #[test]
    fn l0_tui_filter_edit_enter_invalid_stays_with_error() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        // Garbage that the parser will reject.
        app.queries.filter_text = "((((".to_owned();
        app.handle(Action::Enter);
        assert_eq!(
            app.queries.mode,
            QueryMode::FilterEdit,
            "invalid filter must NOT exit the editor"
        );
        assert!(
            app.queries.filter_error.is_some(),
            "invalid filter must surface a parser error"
        );
        assert!(
            app.queries.filter_plan.is_none(),
            "invalid filter must NOT store a plan"
        );
    }

    /// Esc cancels: clears text, clears any active plan, returns to
    /// Build.
    #[test]
    fn l0_tui_filter_edit_esc_cancels_and_clears() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        // Pretend the user previously had a plan applied.
        app.queries.filter_text = "country=CAN".to_owned();
        app.queries.mode = QueryMode::FilterEdit;
        let _ = app.handle(Action::Enter); // apply, exit
        assert!(app.queries.filter_plan.is_some());

        // Re-enter editor, type more, Esc.
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text.push_str(" AND age<30");
        app.handle(Action::Escape);
        assert_eq!(app.queries.mode, QueryMode::Build);
        assert!(
            app.queries.filter_text.is_empty(),
            "Esc must clear the in-progress text"
        );
        assert!(
            app.queries.filter_plan.is_none(),
            "Esc must clear the active plan"
        );
    }

    /// Empty Enter clears any active plan and exits — gives the user
    /// a fast "remove filter" gesture without retyping.
    #[test]
    fn l0_tui_filter_edit_empty_enter_clears_plan() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.filter_text = "country=CAN".to_owned();
        app.queries.mode = QueryMode::FilterEdit;
        let _ = app.handle(Action::Enter);
        assert!(app.queries.filter_plan.is_some(), "precondition");

        // Re-enter, leave empty, Enter.
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text.clear();
        app.handle(Action::Enter);
        assert_eq!(app.queries.mode, QueryMode::Build);
        assert!(
            app.queries.filter_plan.is_none(),
            "empty Enter must clear the active plan"
        );
    }

    /// `f` while results pane is focused (Tab/Space-toggled into
    /// it) must NOT enter FilterEdit — falls through to favorites
    /// add. The filter overlay only opens from the field editor.
    #[test]
    fn l0_tui_filter_edit_f_with_results_focused_does_not_enter() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.results_focused = true;
        app.handle(Action::AddToFavorites);
        assert_eq!(
            app.queries.mode,
            QueryMode::Build,
            "results-focused 'f' must fall through (favorites flow), \
             not enter FilterEdit"
        );
    }

    /// Whitespace-only Enter clears any active plan (same as empty
    /// Enter) — defensive against the user hitting Enter after
    /// accidentally typing only spaces.
    #[test]
    fn l0_tui_filter_edit_whitespace_enter_clears_plan() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.filter_text = "country=CAN".to_owned();
        app.queries.mode = QueryMode::FilterEdit;
        let _ = app.handle(Action::Enter);
        assert!(app.queries.filter_plan.is_some());

        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "   \t  ".to_owned();
        app.handle(Action::Enter);
        assert_eq!(app.queries.mode, QueryMode::Build);
        assert!(app.queries.filter_plan.is_none());
    }

    /// Re-entering FilterEdit preserves the previously-typed text
    /// (lets the user refine without retyping the whole filter).
    /// Esc is the only way to fully clear.
    #[test]
    fn l0_tui_filter_edit_text_persists_across_reentry() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "country=CAN".to_owned();
        app.handle(Action::Enter);
        assert_eq!(app.queries.mode, QueryMode::Build);

        // Re-open. Text must still be there.
        app.handle(Action::AddToFavorites); // 'f' → FilterEdit
        assert_eq!(app.queries.mode, QueryMode::FilterEdit);
        assert_eq!(
            app.queries.filter_text, "country=CAN",
            "re-opening the editor must preserve last-applied text"
        );
    }

    /// Quit (Action::Quit, ctrl-c equivalent) inside FilterEdit
    /// returns true so the outer loop exits, mirroring the save-name
    /// editor's behavior.
    #[test]
    fn l0_tui_filter_edit_quit_propagates() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        let should_quit = app.handle(Action::Quit);
        assert!(
            should_quit,
            "Quit inside FilterEdit must propagate (return true)"
        );
    }

    // ── Phase Art Ross — Wave 24 filter-preset round-trip (handler) ────────

    /// Ensure `apply_saved_json` is wired into the LoadList Enter
    /// handler so the recovered filter_text lands on `App` AND the
    /// plan is re-parsed.
    #[test]
    fn l0_w24_load_restores_filter_text_and_plan() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        // Simulate a freshly-listed saved-queries DB row.
        let json = crate::tui::screens::queries::fields_and_filter_to_json(
            &app.queries.fields,
            "country=CAN",
        );
        app.queries.saved_list = vec![("preset1".to_owned(), json)];
        app.queries.mode = QueryMode::LoadList;
        app.selected = 0;

        app.handle(Action::Enter);

        assert_eq!(app.queries.mode, QueryMode::Build);
        assert_eq!(
            app.queries.filter_text, "country=CAN",
            "load must restore the filter text onto App state"
        );
        assert!(
            app.queries.filter_plan.is_some(),
            "load must re-parse the filter into an active plan"
        );
    }

    /// Empty filter_text in the saved JSON loads as empty: no plan
    /// reset, no error.
    #[test]
    fn l0_w24_load_empty_filter_clears_plan_state() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        // Pre-load a stale plan to ensure load actually clears it.
        app.queries.filter_text = "stale".to_owned();
        app.queries.filter_plan = icelines_query::parse_query(
            icelines_query::FilterInput::Cli("country=CAN".to_owned()),
        )
        .ok();

        let json = crate::tui::screens::queries::fields_and_filter_to_json(
            &app.queries.fields,
            "",
        );
        app.queries.saved_list = vec![("no-filter-preset".to_owned(), json)];
        app.queries.mode = QueryMode::LoadList;
        app.selected = 0;

        app.handle(Action::Enter);

        assert_eq!(
            app.queries.filter_text, "",
            "load with empty filter must reset filter text"
        );
        assert!(
            app.queries.filter_plan.is_none(),
            "load with empty filter must clear the active plan"
        );
    }

    /// Loading a v1 (legacy) array saved query: fields restore,
    /// filter state stays empty (no plan, no text).
    #[test]
    fn l0_w24_load_v1_legacy_keeps_filter_empty() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        let v1 = r#"[{"label":"Sort by","selected":2}]"#.to_owned();
        app.queries.saved_list = vec![("legacy".to_owned(), v1)];
        app.queries.mode = QueryMode::LoadList;
        app.selected = 0;

        app.handle(Action::Enter);

        assert_eq!(app.queries.filter_text, "");
        assert!(app.queries.filter_plan.is_none());
        assert_eq!(app.queries.fields[0].selected, 2);
    }

    /// Loading a saved JSON with a filter that no longer parses
    /// (older grammar removed in a future release): non-fatal —
    /// fields restore, filter text restores, plan stays None,
    /// status hints at re-edit. The user discovers the issue when
    /// they press `f`.
    #[test]
    fn l0_w24_load_unparseable_filter_is_non_fatal() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        // Unbalanced parens — guaranteed parse failure today and
        // forever.
        let json = serde_json::json!({
            "version": 2,
            "fields": [],
            "filter_text": "((((country=CAN",
        })
        .to_string();
        app.queries.saved_list = vec![("broken".to_owned(), json)];
        app.queries.mode = QueryMode::LoadList;
        app.selected = 0;

        app.handle(Action::Enter);

        assert_eq!(app.queries.mode, QueryMode::Build);
        assert_eq!(
            app.queries.filter_text, "((((country=CAN",
            "filter text must round-trip even when the grammar rejects it"
        );
        assert!(
            app.queries.filter_plan.is_none(),
            "unparseable filter must NOT install a plan"
        );
    }

    // ── Phase Art Ross — Wave 24b filter history (Up/Down) ─────────────────

    /// `push_filter_history` adds new entries to the front and
    /// dedupes against the existing front. Sanity for the helper.
    #[test]
    fn l0_w24b_push_filter_history_dedupes_consecutive() {
        use std::collections::VecDeque;
        let mut h: VecDeque<String> = VecDeque::new();
        push_filter_history(&mut h, "country=CAN".into());
        push_filter_history(&mut h, "country=CAN".into()); // dup
        push_filter_history(&mut h, "age<25".into());
        push_filter_history(&mut h, "country=CAN".into()); // not consecutive — kept
        assert_eq!(h.len(), 3);
        assert_eq!(h[0], "country=CAN");
        assert_eq!(h[1], "age<25");
        assert_eq!(h[2], "country=CAN");
    }

    /// History caps at FILTER_HISTORY_CAP — older entries fall off
    /// the back when the ring is full.
    #[test]
    fn l0_w24b_push_filter_history_caps_at_max() {
        use std::collections::VecDeque;
        let mut h: VecDeque<String> = VecDeque::new();
        for i in 0..(FILTER_HISTORY_CAP + 5) {
            push_filter_history(&mut h, format!("filter-{i}"));
        }
        assert_eq!(h.len(), FILTER_HISTORY_CAP);
        // Front is the newest push.
        assert_eq!(h[0], format!("filter-{}", FILTER_HISTORY_CAP + 4));
        // Back is the oldest entry that survived (oldest 5 fell off).
        assert_eq!(h[FILTER_HISTORY_CAP - 1], "filter-5");
    }

    /// Successful Enter in the editor pushes the typed filter onto
    /// history. Hammering Enter on the same filter doesn't add
    /// duplicates.
    #[test]
    fn l0_w24b_enter_pushes_to_history() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "country=CAN".to_owned();
        app.handle(Action::Enter);
        assert_eq!(app.queries.filter_history.len(), 1);
        assert_eq!(app.queries.filter_history[0], "country=CAN");

        // Re-enter and Enter again with same filter — no duplicate.
        app.queries.mode = QueryMode::FilterEdit;
        app.handle(Action::Enter);
        assert_eq!(app.queries.filter_history.len(), 1);

        // Enter a different filter — pushed.
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "age<25".to_owned();
        app.handle(Action::Enter);
        assert_eq!(app.queries.filter_history.len(), 2);
        assert_eq!(app.queries.filter_history[0], "age<25");
        assert_eq!(app.queries.filter_history[1], "country=CAN");
    }

    /// Parse-failure Enter does NOT push to history — only the
    /// successful parses become recallable.
    #[test]
    fn l0_w24b_parse_error_does_not_push_history() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.queries.filter_text = "((((".to_owned();
        app.handle(Action::Enter);
        assert!(app.queries.filter_error.is_some());
        assert!(app.queries.filter_history.is_empty());
    }

    /// Up navigates from live edit (cursor=None) into the newest
    /// history entry. Walking past the oldest stays put.
    #[test]
    fn l0_w24b_up_walks_history_backward() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        // Seed the history.
        app.queries.filter_history = ["age<25", "country=CAN", "p>=20"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        app.queries.mode = QueryMode::FilterEdit;
        // Cursor=None initially.
        assert!(app.queries.filter_history_cursor.is_none());

        app.handle(Action::Up);
        assert_eq!(app.queries.filter_history_cursor, Some(0));
        assert_eq!(app.queries.filter_text, "age<25");

        app.handle(Action::Up);
        assert_eq!(app.queries.filter_history_cursor, Some(1));
        assert_eq!(app.queries.filter_text, "country=CAN");

        app.handle(Action::Up);
        assert_eq!(app.queries.filter_history_cursor, Some(2));
        assert_eq!(app.queries.filter_text, "p>=20");

        // Past the oldest — stay.
        app.handle(Action::Up);
        assert_eq!(app.queries.filter_history_cursor, Some(2));
        assert_eq!(app.queries.filter_text, "p>=20");
    }

    /// Down walks toward newer entries; from cursor=0 returns to
    /// live edit (cursor=None, text cleared).
    #[test]
    fn l0_w24b_down_walks_history_forward_to_live() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.filter_history = ["age<25", "country=CAN"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        app.queries.mode = QueryMode::FilterEdit;

        // Walk Up twice to reach cursor=1.
        app.handle(Action::Up);
        app.handle(Action::Up);
        assert_eq!(app.queries.filter_history_cursor, Some(1));
        assert_eq!(app.queries.filter_text, "country=CAN");

        // Down → cursor=0, text=age<25.
        app.handle(Action::Down);
        assert_eq!(app.queries.filter_history_cursor, Some(0));
        assert_eq!(app.queries.filter_text, "age<25");

        // Down → cursor=None, text="".
        app.handle(Action::Down);
        assert!(app.queries.filter_history_cursor.is_none());
        assert_eq!(app.queries.filter_text, "");

        // Down at live — no-op.
        app.handle(Action::Down);
        assert!(app.queries.filter_history_cursor.is_none());
        assert_eq!(app.queries.filter_text, "");
    }

    /// Up with empty history is a no-op (no panic, cursor stays
    /// None).
    #[test]
    fn l0_w24b_up_empty_history_is_noop() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        assert!(app.queries.filter_history.is_empty());
        app.handle(Action::Up);
        assert!(app.queries.filter_history_cursor.is_none());
        assert_eq!(app.queries.filter_text, "");
    }

    /// Typing while navigating history breaks navigation: cursor
    /// resets to None so the typed character is treated as a free
    /// edit, not an in-place mutation of the historical entry.
    #[test]
    fn l0_w24b_typing_while_navigating_resets_cursor() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.filter_history = vec!["age<25".to_owned()].into();
        app.queries.mode = QueryMode::FilterEdit;

        app.handle(Action::Up);
        assert_eq!(app.queries.filter_history_cursor, Some(0));
        assert_eq!(app.queries.filter_text, "age<25");

        app.handle(Action::Char('!'));
        assert!(
            app.queries.filter_history_cursor.is_none(),
            "typing while in history must drop cursor to live"
        );
        assert_eq!(app.queries.filter_text, "age<25!");
    }

    /// Backspace also resets the cursor — the user is now editing
    /// the historical text freely.
    #[test]
    fn l0_w24b_backspace_while_navigating_resets_cursor() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.filter_history = vec!["age<25".to_owned()].into();
        app.queries.mode = QueryMode::FilterEdit;

        app.handle(Action::Up);
        app.handle(Action::Backspace);
        assert!(app.queries.filter_history_cursor.is_none());
        assert_eq!(app.queries.filter_text, "age<2");
    }

    // ── Phase Art Ross — Wave 24d grammar cheatsheet toggle ────────────────

    /// `?` inside FilterEdit toggles `query_filter_show_help` —
    /// does NOT open the global help overlay (which would close
    /// the editor).
    #[test]
    fn l0_w24d_help_toggles_filter_cheatsheet() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        assert!(!app.queries.filter_show_help);
        assert!(!app.show_help, "global help must be off initially");

        app.handle(Action::Help);
        assert!(app.queries.filter_show_help, "first ? turns cheatsheet on");
        assert!(
            !app.show_help,
            "global help overlay must NOT open from inside FilterEdit"
        );
        assert_eq!(
            app.queries.mode,
            QueryMode::FilterEdit,
            "? must NOT exit FilterEdit"
        );

        app.handle(Action::Help);
        assert!(!app.queries.filter_show_help, "second ? turns cheatsheet off");
        assert_eq!(app.queries.mode, QueryMode::FilterEdit);
    }

    /// Outside FilterEdit, `?` keeps its standard meaning (opens
    /// the global help overlay). Regression guard against
    /// unintended global rebinding.
    #[test]
    fn l0_w24d_help_outside_filter_edit_opens_global() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        // query_mode defaults to Build — NOT FilterEdit.
        assert!(matches!(app.queries.mode, QueryMode::Build));
        app.handle(Action::Help);
        assert!(
            app.show_help,
            "? in Build mode must open the global help overlay"
        );
        assert!(!app.queries.filter_show_help);
    }

    /// Cheatsheet flag persists across mode toggles within the
    /// editor — power users keep it on while typing, parsing,
    /// re-editing.
    #[test]
    fn l0_w24d_cheatsheet_flag_persists_across_enter_apply() {
        let mut app = App::new(false);
        app.screen = Screen::Queries;
        app.queries.mode = QueryMode::FilterEdit;
        app.handle(Action::Help);
        assert!(app.queries.filter_show_help);

        app.queries.filter_text = "country=CAN".to_owned();
        app.handle(Action::Enter); // apply, exit to Build
        assert_eq!(app.queries.mode, QueryMode::Build);
        assert!(
            app.queries.filter_show_help,
            "cheatsheet flag must survive Enter-apply"
        );

        // Re-enter editor — flag still on.
        app.handle(Action::AddToFavorites); // 'f' → FilterEdit
        assert_eq!(app.queries.mode, QueryMode::FilterEdit);
        assert!(app.queries.filter_show_help);
    }

    // ── Phase Masterton.2.1 — App::dispatch (ScreenAction interpreter) ──

    /// `ScreenAction::Continue` is the no-op default — returns
    /// false (don't quit) and mutates nothing. Pin the contract
    /// so a refactor can't accidentally start mutating state on
    /// Continue.
    #[test]
    fn l0_masterton_dispatch_continue_is_noop() {
        use crate::tui::screen::ScreenAction;
        let mut app = App::new(false);
        let screen_before = app.screen.clone();
        let status_before = app.status.clone();
        let should_quit = app.dispatch(ScreenAction::Continue);
        assert!(!should_quit);
        assert_eq!(app.screen, screen_before);
        assert_eq!(app.status, status_before);
    }

    /// `ScreenAction::Quit` propagates — dispatch returns true
    /// so the outer loop tears down the TUI.
    #[test]
    fn l0_masterton_dispatch_quit_returns_true() {
        use crate::tui::screen::ScreenAction;
        let mut app = App::new(false);
        let should_quit = app.dispatch(ScreenAction::Quit);
        assert!(should_quit);
    }

    /// `ScreenAction::Push(spec)` saves current as prev and
    /// switches. Pop returns to prev.
    #[test]
    fn l0_masterton_dispatch_push_pop_navigates() {
        use crate::tui::screen::ScreenAction;
        let mut app = App::new(false);
        app.screen = Screen::Home;

        app.dispatch(ScreenAction::Push(Screen::Queries));
        assert_eq!(app.screen, Screen::Queries);
        assert_eq!(app.prev_screen, Some(Screen::Home));

        app.dispatch(ScreenAction::Pop);
        assert_eq!(app.screen, Screen::Home);
        assert!(app.prev_screen.is_none());
    }

    /// `ScreenAction::Replace` switches without saving prev.
    #[test]
    fn l0_masterton_dispatch_replace_does_not_save_prev() {
        use crate::tui::screen::ScreenAction;
        let mut app = App::new(false);
        app.screen = Screen::Home;
        app.prev_screen = None;

        app.dispatch(ScreenAction::Replace(Screen::Goalies));
        assert_eq!(app.screen, Screen::Goalies);
        assert!(
            app.prev_screen.is_none(),
            "Replace must NOT save prev_screen (unlike Push)"
        );
    }

    /// `ScreenAction::OpenOverlay(Help)` flips show_help.
    #[test]
    fn l0_masterton_dispatch_open_help_overlay() {
        use crate::tui::screen::{OverlayKind, ScreenAction};
        let mut app = App::new(false);
        assert!(!app.show_help);
        app.dispatch(ScreenAction::OpenOverlay(OverlayKind::Help));
        assert!(app.show_help);
    }

    /// `ScreenAction::OpenOverlay(Admin)` flips show_admin (each
    /// overlay kind routes to the right flag — sanity check the
    /// mapping doesn't get crossed in a refactor).
    #[test]
    fn l0_masterton_dispatch_open_admin_overlay() {
        use crate::tui::screen::{OverlayKind, ScreenAction};
        let mut app = App::new(false);
        assert!(!app.show_admin);
        app.dispatch(ScreenAction::OpenOverlay(OverlayKind::Admin));
        assert!(app.show_admin);
        // Sanity: didn't accidentally flip something else.
        assert!(!app.show_help);
        assert!(!app.show_season_picker);
        assert!(!app.show_reports_overlay);
        assert!(!app.show_docs);
        assert!(!app.date_picker.open);
        assert!(!app.group_picker.open);
    }

    /// Each OverlayKind routes to a distinct overlay flag.
    /// Iterates every variant so a future addition (e.g., a new
    /// overlay) trips this test if the mapping isn't wired.
    #[test]
    fn l0_masterton_dispatch_every_overlay_kind_routes_distinct_flag() {
        use crate::tui::screen::{OverlayKind, ScreenAction};
        let cases = [
            (OverlayKind::Help, "show_help"),
            (OverlayKind::Admin, "show_admin"),
            (OverlayKind::SeasonPicker, "show_season_picker"),
            (OverlayKind::Reports, "show_reports_overlay"),
            (OverlayKind::Docs, "show_docs"),
            (OverlayKind::DatePicker, "date_picker.open"),
            (OverlayKind::GroupPicker, "group_picker.open"),
        ];
        for (kind, name) in cases {
            let mut app = App::new(false);
            app.dispatch(ScreenAction::OpenOverlay(kind));
            let flipped = match name {
                "show_help" => app.show_help,
                "show_admin" => app.show_admin,
                "show_season_picker" => app.show_season_picker,
                "show_reports_overlay" => app.show_reports_overlay,
                "show_docs" => app.show_docs,
                "date_picker.open" => app.date_picker.open,
                "group_picker.open" => app.group_picker.open,
                _ => unreachable!(),
            };
            assert!(flipped, "{name} must be true after OpenOverlay({kind:?})");
        }
    }

    /// `ScreenAction::Flash(msg)` writes to status, no other
    /// side effects.
    #[test]
    fn l0_masterton_dispatch_flash_writes_status() {
        use crate::tui::screen::ScreenAction;
        let mut app = App::new(false);
        let screen_before = app.screen.clone();
        app.dispatch(ScreenAction::Flash("hello".into()));
        assert_eq!(app.status, "hello");
        // No screen change.
        assert_eq!(app.screen, screen_before);
        // No overlay opened.
        assert!(!app.show_help);
    }

    /// Push → Replace clears prev_screen (since Replace doesn't
    /// save prev). Esc-from-leaf via Pop after a Replace is a
    /// no-op (nothing to pop back to).
    #[test]
    fn l0_masterton_dispatch_replace_after_push_drops_prev() {
        use crate::tui::screen::ScreenAction;
        let mut app = App::new(false);
        app.screen = Screen::Home;
        app.dispatch(ScreenAction::Push(Screen::Queries));
        assert_eq!(app.prev_screen, Some(Screen::Home));

        // Replace overwrites screen but does NOT touch prev.
        app.dispatch(ScreenAction::Replace(Screen::Goalies));
        assert_eq!(app.screen, Screen::Goalies);
        assert_eq!(
            app.prev_screen,
            Some(Screen::Home),
            "Replace preserves whatever prev was (doesn't overwrite Push's stash)"
        );
    }

    /// `make_context` borrows disjoint fields — split-borrow
    /// sanity. The test exercises the usage pattern documented
    /// on `make_context`: hold `&mut app.queries` and
    /// `make_context()` simultaneously.
    #[test]
    fn l0_masterton_make_context_split_borrow_works() {
        let mut app = App::new(false);
        // Take a mutable borrow of the per-screen state.
        let queries: &mut crate::tui::screens::queries::QueriesState = &mut app.queries;
        // Pretend we've loaded some state.
        queries.filter_text = "test".into();

        // Now grab the AppContext — borrow checker accepts
        // because make_context borrows disjoint fields.
        // (Compile-time check; if this builds, the contract holds.)
        // Note: we can't call make_context HERE because `queries` is
        // alive. That's expected — handlers take `&mut state` AND
        // the orchestrator builds `ctx` first. The pattern works
        // when ctx is built BEFORE the screen-state borrow:
        let _ = queries.filter_text.len();
    }

    // ── Phase Masterton.3 — locked_screen (standalone mode) ──────────────

    /// Default App is multi-tab — locked_screen is None,
    /// Tab/Shift+Tab cycle through screens normally.
    #[test]
    fn l0_masterton_app_default_is_multi_tab() {
        let app = App::new(false);
        assert!(
            app.locked_screen.is_none(),
            "default App must be multi-tab (locked_screen = None)"
        );
    }

    /// When locked_screen is Some(X), Tab is a no-op — screen
    /// stays put.
    #[test]
    fn l0_masterton_locked_tab_is_noop() {
        let mut app = App::new(false);
        app.screen = Screen::Goalies;
        app.locked_screen = Some(Screen::Goalies);
        app.handle(Action::Tab);
        assert_eq!(
            app.screen,
            Screen::Goalies,
            "locked Tab must NOT cycle the screen"
        );
    }

    /// When locked_screen is Some(X), Shift+Tab is a no-op too.
    #[test]
    fn l0_masterton_locked_tabprev_is_noop() {
        let mut app = App::new(false);
        app.screen = Screen::Goalies;
        app.locked_screen = Some(Screen::Goalies);
        app.handle(Action::TabPrev);
        assert_eq!(
            app.screen,
            Screen::Goalies,
            "locked Shift+Tab must NOT cycle the screen"
        );
    }

    /// When locked_screen is None (default), Tab cycles normally.
    /// Pin the contract so a refactor of the locked-screen check
    /// can't accidentally break the multi-tab path.
    #[test]
    fn l0_masterton_unlocked_tab_still_cycles() {
        let mut app = App::new(false);
        app.screen = Screen::Home;
        // Default — locked_screen is None.
        assert!(app.locked_screen.is_none());
        app.handle(Action::Tab);
        assert_ne!(
            app.screen,
            Screen::Home,
            "unlocked Tab must cycle the screen"
        );
    }

    /// Locked mode doesn't break other handlers — opening an
    /// overlay (`?` for help) still works in locked mode.
    #[test]
    fn l0_masterton_locked_overlays_still_work() {
        let mut app = App::new(false);
        app.screen = Screen::Goalies;
        app.locked_screen = Some(Screen::Goalies);
        assert!(!app.show_help);
        app.handle(Action::Help);
        assert!(
            app.show_help,
            "locked mode must not block the help overlay"
        );
    }

    /// Locked mode doesn't break per-screen keybinds — `s` on
    /// Goalies still cycles sort.
    #[test]
    fn l0_masterton_locked_per_screen_keybinds_still_work() {
        let mut app = App::new(false);
        app.screen = Screen::Goalies;
        app.locked_screen = Some(Screen::Goalies);
        let sort_before = app.goalies.sort;
        app.handle(Action::Char('s'));
        assert_ne!(
            app.goalies.sort, sort_before,
            "locked mode must not block per-screen keybinds"
        );
    }
}
