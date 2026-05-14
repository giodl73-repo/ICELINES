//! Askama template structs.
//!
//! Phase King Clancy King.1.4 — `base.html` + `home.html` ship today.
//! Each future sub-phase adds its own template alongside the matching
//! handler:
//!
//! - King.2 — `leaders.html`, `leaders_partial.html` (HTMX fragment)
//! - King.3 — `player.html`, `player_partial.html`, `compare.html`, `comps.html`
//! - King.4 — `team.html`, `depth.html`, `class.html`, `trade.html`
//! - King.5 — `goalies.html`, `goalie.html`
//! - King.6 — `reports.html`, `seasons.html`
//! - King.7 — `scores.html`, `schedule.html`, `playoffs.html`, `series.html`, `game.html`
//! - King.8 — `transactions.html`, `search.html`, `docs.html`, `groups.html`, `games.html`
//! - King.9 — `fantasy/*.html`
//!
//! ## Inheritance
//!
//! Every page extends `base.html`. The base owns:
//! - `<head>`: charset, viewport, title block, stylesheet link, favicon
//! - skip-to-content link
//! - active-(season, season_type) sticky header
//! - `<main id="main">` landmark
//!
//! Pages override `{% block title %}` and `{% block content %}`. Every
//! page must therefore receive `active_label: String` so the base
//! header renders. The `l1_html_each_route_has_active_season_header`
//! fence (King.1.4 closeout) walks the router and asserts the label
//! appears in every HTML response.
//!
//! ## Compile-time templates
//!
//! `askama` parses templates at *build* time. A typo in a template
//! (`{% if foo` instead of `{% if foo %}`) fails `cargo build`, not
//! a runtime request. This is one of the spec's "vendored
//! interactivity" wins — no runtime template engine, no runtime
//! template fetch.

use askama::Template;

/// `home.html` — the `/` page.
///
/// Carries `active_label` (e.g. `"25-26 · Regular"`) into the base
/// template so the season header renders. Also carries top-3 preview
/// slices so the dashboard's first impression is real data, not a
/// templating skeleton.
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub active_label: String,
    /// Top-3 skaters by points in the active (season, season_type).
    /// Empty if the repo has no skater rows for that window.
    pub top_skaters: Vec<LeaderRow>,
    /// Top-3 qualified goalies by save percentage.
    /// Empty if no goalie meets the qualified-GP threshold.
    pub top_goalies: Vec<GoalieRow>,
}

/// `dashboard.html` — Jack Adams Web shell. Server-rendered first
/// so the dashboard is useful without JavaScript; later slices
/// progressively enhance workspace swaps and the command palette.
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub active_label: String,
    pub workspace_url: String,
    pub workspace_label: String,
    pub workspace_summary: Vec<DashboardSummaryRow>,
    pub scores_summary: String,
    pub favorites: Vec<DashboardEntityRow>,
    pub watchlist: Vec<DashboardEntityRow>,
    pub schedule_links: Vec<DashboardLinkRow>,
    pub workspace_links: Vec<DashboardLinkRow>,
}

#[derive(Template)]
#[template(path = "dashboard_workspace.html")]
pub struct DashboardWorkspaceTemplate {
    pub workspace_url: String,
    pub workspace_label: String,
    pub workspace_summary: Vec<DashboardSummaryRow>,
    pub workspace_links: Vec<DashboardLinkRow>,
}

#[derive(Debug, Clone)]
pub struct DashboardSummaryRow {
    pub label: String,
    pub value: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct DashboardEntityRow {
    pub label: String,
    pub href: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct DashboardLinkRow {
    pub label: String,
    pub href: String,
    pub detail: String,
}

/// `not_found.html` — Sasq.7. Friendly 404 with a player-search
/// input. Replaces axum's default bare 404 for a typo'd URL like
/// `/playr/8478402` so users land somewhere useful.
#[derive(Template)]
#[template(path = "not_found.html")]
pub struct NotFoundTemplate {
    pub active_label: String,
    pub requested_path: String,
    pub compare_suggestions: Vec<(String, u32)>,
}

/// `coming_soon.html` — placeholder for routes that have a slot in
/// the spec but no handler shipped yet. Each section nav link on the
/// home page mounts one of these so a click lands on a real page
/// (with the active-season header + back-to-dashboard link) rather
/// than axum's bare 404. As real handlers ship in King.2+, the
/// coming-soon mounts get replaced.
#[derive(Template)]
#[template(path = "coming_soon.html")]
pub struct ComingSoonTemplate {
    pub title: String,
    pub king_phase: String,
    pub description: String,
    pub active_label: String,
}

/// `leaders.html` — King.2.x leaderboards. King.2.1 shipped the
/// minimum viable real-data path; King.2.2 adds sort + position +
/// top-N query params; King.2.3 adds the boolean filter form;
/// King.2.4 adds the JSON twin at `/api/v1/leaders`.
#[derive(Template)]
#[template(path = "leaders.html")]
pub struct LeadersTemplate {
    pub active_label: String,
    pub rows: Vec<LeaderRow>,
    pub total: usize,
    /// Human label for the active sort, e.g. "Points/Game".
    pub active_sort_label: String,
    /// Active sort URL token (`points`/`goals`/`assists`/`gp`/`ppg`).
    pub active_sort: String,
    /// Active position filter URL token (`C`/`LW`/`F`/... or `""`).
    pub active_pos: String,
    /// Top-N rows requested (default 20).
    pub active_top: usize,
    /// Position-filter strip — pre-computed (label, url-value, is_active)
    /// triples so the template doesn't need to compare String against
    /// &str (askama doesn't auto-coerce).
    pub pos_chips: Vec<PosChip>,
    /// Column headers — same pre-computation pattern.
    pub col_headers: Vec<ColHeader>,
    /// Active `?filter=` strings, in URL order. Each renders as one
    /// row in the filter form. Empty Vec → form starts blank.
    pub active_filters: Vec<String>,
    /// Sasq.9 — bio-filter values. Each is the empty string when
    /// the corresponding query param wasn't set, so the template
    /// can put them in `value="…"` attributes without conditionals.
    /// `_str` because integers may be 0 legitimately so we can't
    /// use 0-as-sentinel.
    pub bio_age_min_str: String,
    pub bio_age_max_str: String,
    pub bio_draft_min_str: String,
    pub bio_draft_max_str: String,
    pub bio_height_min_str: String,
    pub bio_height_max_str: String,
    pub bio_weight_min_str: String,
    pub bio_weight_max_str: String,
    pub bio_country: String,
    pub bio_shoots: String,
    /// True when ANY bio filter is active, used to expand the
    /// filter accordion on page load.
    pub bio_active: bool,
    /// URL fragment like "&age-min=22&age-max=28&country=CAN" that
    /// every chip / column-header / pagination link appends so
    /// changing position or sort doesn't drop the bio narrowing.
    /// Empty when no bio filters set. Pre-encoded.
    pub bio_query_suffix: String,
}

/// One chip in the position-filter strip.
#[derive(Debug, Clone)]
pub struct PosChip {
    /// What the user sees ("All", "C", "LW", ...).
    pub label: String,
    /// `?pos=` value (empty for "All").
    pub value: String,
    /// True when this chip matches the active filter.
    pub is_active: bool,
}

/// One sortable column header.
#[derive(Debug, Clone)]
pub struct ColHeader {
    /// `?sort=` URL token.
    pub url_token: String,
    /// Visible header label ("GP", "G", "A", "P", "P/GP").
    pub label: String,
    /// True when this column is the active sort.
    pub is_active: bool,
}

/// One row in the leaderboard. Pre-projected from the
/// `PlayerView` so the template doesn't reach into core types
/// (askama doesn't allow `as f64` casts inline; rate stats and
/// Option<>'s are formatted to strings here).
///
/// UX.C (2026-05-04) added the realtime + special-teams columns:
/// `plus_minus_str`, `pim`, `shots`, `shooting_pct_str`,
/// `hits_str`, `blocks_str`, `faceoff_pct_str`, `pp_points`. The
/// extra fields are pre-formatted strings so the template doesn't
/// need to branch on Option<>.
#[derive(Debug, Clone)]
pub struct LeaderRow {
    /// NHL player id (e.g. 8478402 = Connor McDavid). Used to build
    /// the link to `/player/:id`.
    pub nhl_id: u32,
    pub name: String,
    pub position: String,
    pub team: String,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    /// Points per game, formatted to 2 decimals. Empty string if gp=0.
    pub ppg_str: String,
    /// Pre-built NHL CDN headshot URL for this row. UX.G2 — switched
    /// from the `mugs/nhl/default/{id}.png` pattern (which serves
    /// silhouettes for many players) to the seasonal team-keyed
    /// `mugs/nhl/{season}/{team}/{id}.png` which carries real
    /// photos for current rosters.
    pub headshot_url: String,
    /// Sasq.6 — fallback URL when `headshot_url` 404s (e.g. a
    /// player not yet on the seasonal roster). The template wires
    /// this into an `onerror` handler so the silhouette only shows
    /// after the real attempt fails.
    pub headshot_fallback_url: String,
    /// Pre-formatted "+12" / "-3" — leading sign always present.
    pub plus_minus_str: String,
    pub pim: u32,
    pub shots: u32,
    /// "10.5%" or "—".
    pub shooting_pct_str: String,
    /// "—" when realtime data is missing for this row.
    pub hits_str: String,
    pub blocks_str: String,
    /// "55.2%" or "—". Forwards only have meaningful values.
    pub faceoff_pct_str: String,
    pub pp_points: u32,
    // Numeric backing for sort comparators. We could re-derive from
    // strings in the sort closure but keeping the raw values here
    // means O(1) compare without re-parsing.
    pub plus_minus: i32,
    pub shooting_pct: Option<f32>,
    pub hits: Option<u32>,
    pub blocks: Option<u32>,
    pub faceoff_pct: Option<f32>,
    // Sasq.5 — per-60 rate stats. Computed as
    // `stat * 3600 / (toi_per_game * gp)`. None when toi data is
    // missing or gp == 0.
    pub points_per_60_str: String,
    pub goals_per_60_str: String,
    pub assists_per_60_str: String,
    pub hits_per_60_str: String,
    pub blocks_per_60_str: String,
    pub points_per_60: Option<f64>,
    pub goals_per_60: Option<f64>,
    pub assists_per_60: Option<f64>,
    pub hits_per_60: Option<f64>,
    pub blocks_per_60: Option<f64>,
    /// Sasq.4 — point delta vs prior season same-type. Used for the
    /// breakout / decline leaderboards. None when no prior season
    /// row exists (rookies, retired). Pre-formatted to "+12" / "-3".
    pub points_delta: Option<i32>,
    pub points_delta_str: String,
    pub points_delta_class: String,
}

/// `scores.html` — King.7.1. Live NHL schedule for a given date
/// (default: today). Uses `NhlApiClient::fetch_today_schedule` /
/// `fetch_schedule_for_date`.
#[derive(Template)]
#[template(path = "scores.html")]
pub struct ScoresTemplate {
    pub active_label: String,
    /// Active calendar date (YYYY-MM-DD). The fetch returns the full
    /// game-week starting from this date, so the template groups rows
    /// by date.
    pub active_date: String,
    /// Pre-calculated previous/next-week date strings for the picker
    /// arrows. ISO YYYY-MM-DD; the template just emits links.
    pub prev_date: String,
    pub next_date: String,
    pub today_date: String,
    /// Pre-grouped by date, sorted ascending. Each `(date_label, rows)`
    /// pair becomes one section in the template.
    pub days: Vec<ScoresDay>,
    pub total_games: usize,
    /// Set when the live fetch failed entirely. Renders an inline
    /// error block instead of the games list.
    pub fetch_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoresDay {
    pub date: String,
    /// Pretty label like "Mon, May 4". Computed in the handler.
    pub date_pretty: String,
    pub rows: Vec<ScoreRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoreRow {
    pub away_abbrev: String,
    pub away_name: String,
    pub home_abbrev: String,
    pub home_name: String,
    /// Empty string when score not yet available (FUT/PRE).
    pub away_score_str: String,
    pub home_score_str: String,
    /// "FINAL", "FINAL/OT", "LIVE", "PRE", "FUT", etc. Pre-formatted.
    pub state_label: String,
    /// "live" / "final" / "future" — used as a CSS class hook.
    pub state_class: String,
    /// Pretty start time like "19:00 UTC" or empty when unknown.
    pub start_time_label: String,
    /// True for playoff games — template adds a series-context line.
    pub is_playoff: bool,
    /// "Game 4 · FLA leads 2-1" style line (only when is_playoff).
    pub series_context: String,
}

/// `playoffs.html` — King.7.2. Bracket view (round-by-round).
#[derive(Template)]
#[template(path = "playoffs.html")]
pub struct PlayoffsTemplate {
    pub active_label: String,
    pub season_pretty: String,
    /// "bundled" or "live" — surfaced in the page so the user knows
    /// whether they're looking at a static historical bundle or a
    /// live-API snapshot.
    pub source_label: String,
    pub rounds: Vec<PlayoffsRoundView>,
    /// True when the bracket has zero series (off-season for live
    /// API, missing bundle for older seasons).
    pub empty: bool,
    pub fetch_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlayoffsRoundView {
    pub round_number: u8,
    pub label: String,
    pub series: Vec<PlayoffsSeriesView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlayoffsSeriesView {
    pub top_abbrev: String,
    pub top_name: String,
    pub top_wins: u8,
    pub bottom_abbrev: String,
    pub bottom_name: String,
    pub bottom_wins: u8,
    /// Pre-formatted summary line ("FLA 4-2 TBL · FLA wins" or
    /// "tied 2-2", "FLA leads 3-1"). Source: `PlayoffSeries::summary`.
    pub summary: String,
    /// True when one side has 4 wins.
    pub is_complete: bool,
    pub conference: String,
}

/// `schedule.html` — King.7.3. Team-season schedule view.
#[derive(Template)]
#[template(path = "schedule.html")]
pub struct ScheduleTemplate {
    pub active_label: String,
    pub season_pretty: String,
    /// Active team abbrev (uppercase) or empty when no team selected.
    pub active_team: String,
    pub team_chips: Vec<TeamChip>,
    pub rows: Vec<ScheduleRow>,
    pub total: usize,
    pub fetch_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TeamChip {
    pub abbrev: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleRow {
    pub date: String,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub away_score_str: String,
    pub home_score_str: String,
    pub state_label: String,
    /// "home" / "away" — perspective of the active team.
    pub home_or_away: String,
    /// Opponent abbrev — convenience for the template.
    pub opponent_abbrev: String,
    pub is_playoff: bool,
}

/// `transactions.html` — King.8.2. League moves feed for the active
/// season. Source: ESPN site.api via `load_transactions_with_fallback`.
#[derive(Template)]
#[template(path = "transactions.html")]
pub struct TransactionsTemplate {
    pub active_label: String,
    pub season_pretty: String,
    /// Pre-projected rows in chronological order (newest first).
    pub rows: Vec<TransactionRow>,
    pub total: usize,
    /// True when no rows landed AND no filters applied — we render
    /// a friendly "fetch transactions to populate" hint instead of
    /// an empty table.
    pub empty_unfiltered: bool,
    /// Active `?kind=` token (or empty for "all kinds"). Used to
    /// render the active state on filter chips.
    pub active_kind: String,
    /// Active `?team=` token uppercased (or empty for "all teams").
    pub active_team: String,
    /// True when the snapshot is missing AND we don't have a bundled
    /// or installed fallback. Triggers a yellow "out of coverage" note.
    pub out_of_coverage: bool,
    /// Earliest season ESPN archives, surfaced in the out-of-coverage
    /// note so the user knows what's available.
    pub earliest_season_pretty: String,
}

/// One row in the transactions feed, projected for the template so
/// askama doesn't reach into icelines-core types directly.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionRow {
    pub date: String,
    /// Empty string for league-wide rows.
    pub team: String,
    /// Lower-case label like "trade", "signing", "recall".
    pub kind_label: String,
    /// Pretty kind name for the chip ("Trade", "Signing").
    pub kind_pretty: String,
    pub description: String,
}

/// `compare.html` — UX.D. Side-by-side comparison of two players.
/// Linked from each player card's "Compare with…" form.
#[derive(Template)]
#[template(path = "compare.html")]
pub struct CompareTemplate {
    pub active_label: String,
    pub a: Option<ComparePlayerCard>,
    pub b: Option<ComparePlayerCard>,
    pub similar: Option<icelines_core::SimilarPlayersView>,
    /// Set when `?a=` or `?b=` is missing or malformed. Renders an
    /// error block + a hint to use the player-card "Compare" form.
    pub error: Option<String>,
    /// Sasq.8 — per-stat winner flags. The template applies a
    /// `winner` CSS class on the bolded side. Defaults to all-false
    /// (no bolding) when one or both players are missing.
    pub winners: CompareWinners,
}

/// Per-stat "which side has the better value" flags. `_a` is true
/// when player A wins (strictly), `_b` is true when player B wins.
/// Both false on equality. For lower-is-better stats (PIM,
/// giveaways) the comparator is flipped before storing.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CompareWinners {
    pub gp_a: bool,
    pub gp_b: bool,
    pub goals_a: bool,
    pub goals_b: bool,
    pub assists_a: bool,
    pub assists_b: bool,
    pub points_a: bool,
    pub points_b: bool,
    pub ppg_a: bool,
    pub ppg_b: bool,
    pub plus_minus_a: bool,
    pub plus_minus_b: bool,
    pub pim_a: bool,
    pub pim_b: bool, // lower-is-better
    pub shots_a: bool,
    pub shots_b: bool,
    pub shooting_pct_a: bool,
    pub shooting_pct_b: bool,
    pub hits_a: bool,
    pub hits_b: bool,
    pub blocks_a: bool,
    pub blocks_b: bool,
    pub takeaways_a: bool,
    pub takeaways_b: bool,
    pub giveaways_a: bool,
    pub giveaways_b: bool, // lower-is-better
    pub faceoff_pct_a: bool,
    pub faceoff_pct_b: bool,
    pub pp_goals_a: bool,
    pub pp_goals_b: bool,
    pub pp_points_a: bool,
    pub pp_points_b: bool,
    pub sh_goals_a: bool,
    pub sh_goals_b: bool,
    pub gwg_a: bool,
    pub gwg_b: bool,
    pub toi_per_game_a: bool,
    pub toi_per_game_b: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComparePlayerCard {
    pub nhl_id: u32,
    pub full_name: String,
    pub position: String,
    pub team: String,
    pub team_link: String,
    pub headshot_url: Option<String>,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ppg_str: String,
    pub plus_minus_str: String,
    pub pim_str: String,
    pub shots_str: String,
    pub shooting_pct_str: String,
    pub hits_str: String,
    pub blocks_str: String,
    pub takeaways_str: String,
    pub giveaways_str: String,
    pub faceoff_pct_str: String,
    pub pp_goals_str: String,
    pub pp_points_str: String,
    pub sh_goals_str: String,
    pub gwg_str: String,
    pub toi_per_game_str: String,
}

/// `docs.html` — King.8.1. Rendered COMMANDS.md.
#[derive(Template)]
#[template(path = "docs.html")]
pub struct DocsTemplate {
    pub active_label: String,
    /// Pre-rendered HTML from pulldown-cmark. The template uses
    /// `|safe` to skip askama's auto-escape.
    pub rendered_html: String,
}

/// `team.html` — King.4.1. Roster page for one team.
/// `scouting.html` - rendered `ReportView` for one player.
#[derive(Template)]
#[template(path = "scouting.html")]
pub struct ScoutingTemplate {
    pub active_label: String,
    pub title: String,
    pub player_href: String,
    /// Pre-rendered HTML from the report markdown body.
    pub rendered_html: String,
    /// Raw markdown body for the copy-as-markdown button.
    pub markdown_body: String,
}

#[derive(Template)]
#[template(path = "team.html")]
pub struct TeamTemplate {
    pub active_label: String,
    pub team_abbrev: String,
    pub skaters: Vec<LeaderRow>,
    pub goalies: Vec<GoalieRow>,
}

#[derive(Template)]
#[template(path = "team_season.html")]
pub struct TeamSeasonTemplate {
    pub active_label: String,
    pub team_abbrev: String,
    pub season_pretty: String,
    pub record_label: String,
    pub points: u32,
    pub points_percentage: String,
    pub goal_differential: String,
    pub home_record: String,
    pub away_record: String,
    pub one_goal_record: String,
    pub last_10_record: String,
    pub last_10_goal_differential: String,
    pub remaining_label: String,
    pub next_opponents: String,
    pub standings_label: String,
    pub schedule_strength_label: String,
    pub quality_ledger_label: String,
    pub warning: String,
    pub rows: Vec<TeamSeasonTemplateRow>,
}

#[derive(Debug, Clone)]
pub struct TeamSeasonTemplateRow {
    pub date: String,
    pub venue: String,
    pub opponent_abbrev: String,
    pub result: String,
    pub score: String,
    pub goal_differential: String,
    pub state_label: String,
    pub is_playoff: bool,
}

#[derive(Template)]
#[template(path = "records.html")]
pub struct RecordsTemplate {
    pub active_label: String,
    pub title: String,
    pub subtitle: String,
    pub back_href: String,
    pub back_label: String,
    pub json_href: String,
    pub subject_label: String,
    pub empty_hint: String,
    pub total: usize,
    pub rows: Vec<RecordsTemplateRow>,
}

#[derive(Debug, Clone)]
pub struct RecordsTemplateRow {
    pub key: String,
    pub label: String,
    pub count: u32,
    pub first_game_id: u64,
    pub first_date: String,
    pub last_game_id: u64,
    pub last_date: String,
}

/// `goalies.html` — King.5.1 minimum viable goalie leaderboard.
#[derive(Template)]
#[template(path = "goalies.html")]
pub struct GoaliesTemplate {
    pub active_label: String,
    pub rows: Vec<GoalieRow>,
    pub total: usize,
    pub qualified_threshold: u32,
}

/// `depth.html` — Phase Lady Byng follow-up. Cross-team line-value
/// rankings, mirroring the TUI Depth tab. Same data path
/// (`icelines_core::cross_team::compute_team_strength_views`).
#[derive(Template)]
#[template(path = "depth.html")]
pub struct DepthTemplate {
    pub active_label: String,
    pub rows: Vec<DepthRow>,
}

/// `poach.html` — Phase Selke.6. Fantasy add/stream/stash board
/// rendered from the shared `PoachBoardView` contract.
#[derive(Template)]
#[template(path = "poach.html")]
pub struct PoachTemplate {
    pub active_label: String,
    pub rows: Vec<PoachRow>,
    pub total: usize,
    pub scoring_scheme: String,
    pub categories: String,
    pub teams: String,
    pub positions: String,
    pub availability: String,
    pub source_note: String,
    pub empty_title: String,
    pub empty_detail: String,
}

/// `/fantasy` — active-league roster gaps rendered from
/// `FantasyRosterGapView`.
#[derive(Template)]
#[template(path = "fantasy.html")]
pub struct FantasyTemplate {
    pub active_label: String,
    pub league: String,
    pub team: String,
    pub scoring_scheme: String,
    pub categories: String,
    pub add_player: String,
    pub drop_player: String,
    pub rows: Vec<FantasyGapRow>,
    pub simulation_rows: Vec<FantasySimulationRow>,
    pub simulation_scenarios: Vec<FantasySimulationScenarioRow>,
    pub simulation_assumptions: Vec<String>,
    pub simulation_warnings: Vec<String>,
    pub warnings: Vec<String>,
    pub empty_title: String,
    pub empty_detail: String,
}

#[derive(Debug, Clone)]
pub struct FantasyGapRow {
    pub rank: usize,
    pub action: String,
    pub action_reason: String,
    pub category: String,
    pub roster_total: String,
    pub weight: String,
    pub player_id: u32,
    pub best_available: String,
    pub team: String,
    pub position: String,
    pub value: String,
    pub weighted_value: String,
    pub weighted_delta: String,
    pub replacement: String,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct FantasySimulationRow {
    pub rank: usize,
    pub team: String,
    pub owner: String,
    pub is_user_team: bool,
    pub projected_score: String,
    pub score_gap: String,
    pub games_remaining: u32,
    pub rostered_players: u16,
}

#[derive(Debug, Clone)]
pub struct FantasySimulationScenarioRow {
    pub action: String,
    pub label: String,
    pub add_player: String,
    pub drop_player: String,
    pub projected_score_delta: String,
    pub projected_games_delta: i32,
    pub confidence: String,
    pub explanation: String,
}

/// `favorites.html` — user-maintained players and teams rendered
/// from the shared local group store.
#[derive(Template)]
#[template(path = "favorites.html")]
pub struct FavoritesTemplate {
    pub active_label: String,
    pub player_count: usize,
    pub team_count: usize,
    pub players: Vec<FavoritePlayerRow>,
    pub teams: Vec<FavoriteTeamRow>,
}

#[derive(Debug, Clone)]
pub struct FavoritePlayerRow {
    pub key: String,
    pub display_name: String,
    pub player_url: String,
    pub stat_line: String,
}

#[derive(Debug, Clone)]
pub struct FavoriteTeamRow {
    pub key: String,
}

/// `watchlist.html` — poach/watch candidates persisted by the CLI/TUI.
#[derive(Template)]
#[template(path = "watchlist.html")]
pub struct WatchlistTemplate {
    pub active_label: String,
    pub player_count: usize,
    pub team_count: usize,
    pub players: Vec<WatchlistPlayerRow>,
    pub teams: Vec<WatchlistTeamRow>,
    pub rules: Vec<WatchRuleTemplateRow>,
    pub alerts: Vec<WatchlistAlertRow>,
}

#[derive(Debug, Clone)]
pub struct WatchlistPlayerRow {
    pub key: String,
    pub display_name: String,
    pub player_url: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct WatchlistTeamRow {
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct WatchRuleTemplateRow {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub enabled_label: String,
    pub next_enabled: bool,
    pub action_label: String,
}

#[derive(Debug, Clone)]
pub struct WatchlistAlertRow {
    pub fired_at: String,
    pub rule_id: String,
    pub entity: String,
    pub message: String,
}

/// `career.html` — cross-league cohort leaderboard.
#[derive(Template)]
#[template(path = "career.html")]
pub struct CareerTemplate {
    pub active_label: String,
    pub league: String,
    pub season_label: String,
    pub season: u32,
    pub sort: String,
    pub count: usize,
    pub total: usize,
    pub rows: Vec<CareerLeaderRow>,
}

#[derive(Debug, Clone)]
pub struct CareerLeaderRow {
    pub rank: usize,
    pub player_id: u32,
    pub name: String,
    pub team: String,
    pub gp: u32,
    pub goals: String,
    pub assists: String,
    pub points: String,
    pub points_per_game: String,
}

/// One pre-formatted row for the poach board template.
#[derive(Debug, Clone)]
pub struct PoachRow {
    pub rank: usize,
    pub player_id: u32,
    pub name: String,
    pub team: String,
    pub position: String,
    pub score: String,
    pub confidence: String,
    pub availability: String,
    pub category_fit: String,
    pub schedule: String,
    pub risk: String,
    pub why: String,
}

/// One row in the depth-rankings table. Pre-formatted strings so the
/// askama template doesn't need to do float casts.
#[derive(Debug, Clone)]
pub struct DepthRow {
    pub team: String,
    pub c_score: String,
    pub lw_score: String,
    pub rw_score: String,
    pub d_score: String,
    pub total: String,
    pub c_top: String,
    pub lw_top: String,
    pub rw_top: String,
    pub d_top: String,
}

/// One row in the goalie leaderboard.
#[derive(Debug, Clone)]
pub struct GoalieRow {
    pub nhl_id: u32,
    pub name: String,
    pub team: String,
    pub gp: u32,
    pub wins: u32,
    pub losses: u32,
    pub saves: u32,
    pub shutouts: u32,
    pub save_pct_str: String,
    pub gaa_str: String,
    pub headshot_url: String,
    pub headshot_fallback_url: String,
}

/// `player.html` — King.3.1 + King.3.2. Player card with career table.
///
/// UX.B (2026-05-04) expanded the active-season stat block: in
/// addition to the basic counting stats, the card now surfaces
/// +/-, PIM, SOG, shooting %, hits, blocks, takeaways, giveaways,
/// faceoff %, and special-teams totals. Each is `String` so the
/// handler can format `Option<>` / floats / `—` placeholders once
/// (askama doesn't support inline casts).
#[derive(Template)]
#[template(path = "player.html")]
pub struct PlayerTemplate {
    pub active_label: String,
    pub nhl_id: u32,
    pub full_name: String,
    pub position: String,
    pub team: String,
    /// Empty when the player has no team in the active (season, type)
    /// — used to suppress the "/team/" link.
    pub team_link: String,
    pub headshot_url: Option<String>,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ppg_str: String,
    /// Active-season detail block — pre-formatted strings so the
    /// template doesn't duplicate the "—" / decimal logic.
    pub plus_minus_str: String,
    pub pim_str: String,
    pub shots_str: String,
    pub shooting_pct_str: String,
    pub hits_str: String,
    pub blocks_str: String,
    pub takeaways_str: String,
    pub giveaways_str: String,
    pub faceoff_pct_str: String,
    pub pp_goals_str: String,
    pub pp_points_str: String,
    pub sh_goals_str: String,
    pub gwg_str: String,
    pub toi_per_game_str: String,
    /// YoY delta strings (Sasq.3). Pre-formatted with sign + comparison
    /// to the player's prior-season totals from the same season-type.
    /// Empty string when no prior-season row exists (rookies, retired
    /// players, etc.). Each is paired with a CSS class hint:
    /// "delta-up" / "delta-down" / "delta-flat" / "" so the template
    /// can color-code without parsing.
    pub goals_delta: String,
    pub goals_delta_class: String,
    pub assists_delta: String,
    pub assists_delta_class: String,
    pub points_delta: String,
    pub points_delta_class: String,
    pub gp_delta: String,
    pub gp_delta_class: String,
    /// Pretty label of the prior season for the YoY caption (e.g. "vs 2024-25").
    pub prior_season_label: String,
    /// Full career — one row per (season, type) the player has stats
    /// for. King.3.2 lands this; the row count for a 10-year veteran
    /// is ~10-20 (regular only) or ~15-30 (regular + playoff).
    pub career_rows: Vec<CareerRow>,
    /// Phase Calder.3 — pre-NHL career stints (junior / NCAA / AHL /
    /// European pro) loaded from `~/.icelines/career_history.json`.
    /// Empty Vec when the local store hasn't been populated; the
    /// template hides the section in that case.
    pub pre_nhl_career: Vec<PreNhlRow>,
    /// (name, nhl_id) pairs for every active skater + goalie in the
    /// active-season repo, used to populate the Compare-with
    /// datalist. UX.H — lets the user type a name with native browser
    /// autocomplete instead of memorizing a 7-digit NHL id. Sorted
    /// alphabetically by name.
    pub compare_suggestions: Vec<(String, u32)>,
}

/// Phase Calder.3 — one pre-NHL stint row in the player template.
#[derive(Debug, Clone)]
pub struct PreNhlRow {
    pub season: String,
    pub league: String,
    pub league_tier: String,
    pub team: String,
    pub gp: u32,
    pub goals_str: String,
    pub assists_str: String,
    pub points_str: String,
    pub ppg_str: String,
}

/// Phase Calder.3 — pure projector: CareerStint slice → PreNhlRow
/// slice with formatted strings (askama can't cast floats inline).
/// Sorts newest-first to match the existing NHL career-table convention.
pub fn project_pre_nhl_html_rows(
    stints: &[icelines_core::career_history::CareerStint],
) -> Vec<PreNhlRow> {
    use icelines_core::career_history::LeagueTier;
    let mut rows: Vec<PreNhlRow> = stints
        .iter()
        .map(|s| PreNhlRow {
            season: format!(
                "{}-{}",
                &s.season.0.to_string()[..4],
                &s.season.0.to_string()[6..]
            ),
            league: s.league.0.clone(),
            league_tier: match s.league.tier() {
                LeagueTier::Pro => "pro",
                LeagueTier::Junior => "junior",
                LeagueTier::College => "college",
                LeagueTier::International => "international",
                LeagueTier::Other => "other",
            }
            .to_owned(),
            team: s.team.clone(),
            gp: s.gp,
            goals_str: s.goals.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
            assists_str: s
                .assists
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            points_str: s
                .points
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            ppg_str: s
                .points_per_game()
                .map(|p| format!("{p:.2}"))
                .unwrap_or_else(|| "—".into()),
        })
        .collect();
    rows.sort_by(|a, b| b.season.cmp(&a.season));
    rows
}

/// One row in the player-card career table.
#[derive(Debug, Clone)]
pub struct CareerRow {
    /// Pretty season label e.g. "2024-25".
    pub season: String,
    /// "Regular" or "Playoff".
    pub season_type: String,
    /// Last team in that (season, type). For mid-season trades this
    /// is rendered as "SEA/NYR" (slash-joined) directly from the
    /// per-season stats row.
    pub team: String,
    /// Empty when `team` is "—", multi-team ("SEA/NYR"), or otherwise
    /// not a clean single-abbrev team. Drives the `<a href="/team/…">`
    /// rendering in the template.
    pub team_link: String,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ppg_str: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase Calder.3 — project_pre_nhl_html_rows ──────────────────

    fn cs_stint(
        season: u32,
        league: &str,
        team: &str,
        gp: u32,
        p: u32,
    ) -> icelines_core::career_history::CareerStint {
        use icelines_core::career_history::{CareerGameType, CareerStint, LeagueAbbrev};
        CareerStint {
            season: icelines_core::Season(season),
            league: LeagueAbbrev::new(league),
            team: team.into(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(p / 2),
            assists: Some(p - p / 2),
            points: Some(p),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    /// Calder.3 / l0_project_pre_nhl_html_rows_formats_strings
    /// — Float-cast happens in this projector (askama can't), so
    ///   ppg_str must be set; integer counts must round-trip.
    #[test]
    fn l0_project_pre_nhl_html_rows_formats_strings() {
        let stints = vec![cs_stint(20142015, "OHL", "Erie", 47, 120)];
        let rows = project_pre_nhl_html_rows(&stints);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.season, "2014-15");
        assert_eq!(r.league, "OHL");
        assert_eq!(r.league_tier, "junior");
        assert_eq!(r.team, "Erie");
        assert_eq!(r.gp, 47);
        assert_eq!(r.points_str, "120");
        assert_eq!(r.ppg_str, "2.55");
    }

    /// Calder.3 / l0_project_pre_nhl_html_rows_sorts_newest_first
    /// — Template renders rows in DOM order; newest-first matches
    ///   the NHL career section's convention above it on the page.
    #[test]
    fn l0_project_pre_nhl_html_rows_sorts_newest_first() {
        let stints = vec![
            cs_stint(20122013, "OHL", "Erie", 63, 66),
            cs_stint(20142015, "OHL", "Erie", 47, 120),
            cs_stint(20132014, "OHL", "Erie", 56, 99),
        ];
        let rows = project_pre_nhl_html_rows(&stints);
        assert_eq!(rows[0].season, "2014-15");
        assert_eq!(rows[1].season, "2013-14");
        assert_eq!(rows[2].season, "2012-13");
    }

    /// Calder.3 / l0_project_pre_nhl_html_rows_tier_classifications
    /// — Tier strings expose the league bucket so the template can
    ///   color-code without parsing — locks the contract.
    #[test]
    fn l0_project_pre_nhl_html_rows_tier_classifications() {
        let stints = vec![
            cs_stint(20132014, "AHL", "Bakersfield", 60, 50),
            cs_stint(20132014, "NCAA", "Boston U.", 36, 30),
            cs_stint(20132014, "OHL", "Erie", 60, 70),
        ];
        let rows = project_pre_nhl_html_rows(&stints);
        let tiers: Vec<&str> = rows.iter().map(|r| r.league_tier.as_str()).collect();
        assert!(tiers.contains(&"pro"));
        assert!(tiers.contains(&"college"));
        assert!(tiers.contains(&"junior"));
    }

    /// l0_home_template_renders
    /// — askama parses templates at build time, so a render failure
    ///   here means a runtime substitution issue (e.g. malformed
    ///   variable). Lock the rendered output contains both the
    ///   page-specific copy AND the base-inherited active-season
    ///   header label.
    #[test]
    fn l0_home_template_renders() {
        let tmpl = HomeTemplate {
            active_label: "25-26 · Regular".to_owned(),
            top_skaters: Vec::new(),
            top_goalies: Vec::new(),
        };
        let html = tmpl.render().expect("home template renders");
        // Page-specific content — the home dashboard has top-3
        // preview sections and section nav. Empty-vec branches
        // render the "no data loaded" copy.
        assert!(html.contains("IceLines"));
        assert!(html.contains("Top scorers"));
        assert!(html.contains("Top goalies"));
        // Base-inherited active-season header
        assert!(
            html.contains("25-26 · Regular"),
            "base template must render active_label, got: {html}"
        );
        // Sticky header structure
        assert!(html.contains("season-header"));
    }

    /// l0_home_template_renders_with_preview
    /// — when the handler populates top_skaters / top_goalies, the
    ///   preview tables MUST surface the player name + linked card
    ///   so a user clicks straight into /player/:id from the home.
    #[test]
    fn l0_home_template_renders_with_preview() {
        let tmpl = HomeTemplate {
            active_label: "25-26 · Regular".to_owned(),
            top_skaters: vec![LeaderRow {
                nhl_id: 8478402,
                name: "Connor McDavid".to_owned(),
                position: "C".to_owned(),
                team: "EDM".to_owned(),
                gp: 50,
                goals: 30,
                assists: 60,
                points: 90,
                ppg_str: "1.80".to_owned(),
                plus_minus_str: "+12".to_owned(),
                pim: 14,
                shots: 220,
                shooting_pct_str: "13.6%".to_owned(),
                hits_str: "30".to_owned(),
                blocks_str: "10".to_owned(),
                faceoff_pct_str: "53.0%".to_owned(),
                pp_points: 30,
                plus_minus: 12,
                shooting_pct: Some(0.136),
                hits: Some(30),
                blocks: Some(10),
                faceoff_pct: Some(0.530),
                headshot_url: "https://assets.nhle.com/mugs/nhl/20252026/EDM/8478402.png"
                    .to_owned(),
                headshot_fallback_url: "https://assets.nhle.com/mugs/nhl/default/8478402.png"
                    .to_owned(),
                points_per_60_str: "4.32".to_owned(),
                goals_per_60_str: "1.44".to_owned(),
                assists_per_60_str: "2.88".to_owned(),
                hits_per_60_str: "1.44".to_owned(),
                blocks_per_60_str: "0.48".to_owned(),
                points_per_60: Some(4.32),
                goals_per_60: Some(1.44),
                assists_per_60: Some(2.88),
                hits_per_60: Some(1.44),
                blocks_per_60: Some(0.48),
                points_delta: Some(12),
                points_delta_str: "+12".to_owned(),
                points_delta_class: "delta-up".to_owned(),
            }],
            top_goalies: vec![GoalieRow {
                nhl_id: 8476945,
                name: "Connor Hellebuyck".to_owned(),
                team: "WPG".to_owned(),
                gp: 40,
                wins: 30,
                losses: 8,
                saves: 1030,
                shutouts: 4,
                save_pct_str: "0.925".to_owned(),
                gaa_str: "2.20".to_owned(),
                headshot_url: "https://assets.nhle.com/mugs/nhl/20252026/WPG/8476945.png"
                    .to_owned(),
                headshot_fallback_url: "https://assets.nhle.com/mugs/nhl/default/8476945.png"
                    .to_owned(),
            }],
        };
        let html = tmpl.render().expect("home template renders with preview");
        assert!(html.contains("Connor McDavid"));
        assert!(html.contains("/player/8478402"));
        assert!(html.contains("Connor Hellebuyck"));
        assert!(html.contains("/player/8476945"));
        assert!(html.contains("0.925"));
        // UX.F — team-link class with team-{ABBREV} class for color
        // theming. If a future template refactor drops the class,
        // every team link site-wide loses its color and this test
        // catches it before it ships.
        assert!(
            html.contains("team-link team-EDM"),
            "skater team cell must carry team-link class for CSS theming"
        );
        assert!(
            html.contains("team-link team-WPG"),
            "goalie team cell must carry team-link class for CSS theming"
        );
    }

    /// l0_home_template_includes_a11y_baseline
    /// — broadcast finding: every page MUST carry viewport meta,
    ///   skip-link, and `<main>` landmark. The base template owns
    ///   these; this test asserts the inheritance still emits them.
    #[test]
    fn l0_home_template_includes_a11y_baseline() {
        let tmpl = HomeTemplate {
            active_label: "x".to_owned(),
            top_skaters: Vec::new(),
            top_goalies: Vec::new(),
        };
        let html = tmpl.render().unwrap();
        assert!(
            html.contains("name=\"viewport\""),
            "every page must include viewport meta"
        );
        assert!(
            html.contains("skip-link"),
            "every page must include skip-to-content link"
        );
        assert!(
            html.contains("id=\"main\"") || html.contains("id='main'"),
            "every page must include <main id='main'> landmark"
        );
        assert!(
            html.contains("lang=\"en\""),
            "every page must declare a lang"
        );
    }

    /// l0_home_template_links_static_assets
    /// — base must reference /static/style.css and /static/icelines.svg
    ///   from the King.1.3 vendored asset pipeline.
    #[test]
    fn l0_home_template_links_static_assets() {
        let tmpl = HomeTemplate {
            active_label: "x".to_owned(),
            top_skaters: Vec::new(),
            top_goalies: Vec::new(),
        };
        let html = tmpl.render().unwrap();
        assert!(html.contains("/static/style.css"));
        assert!(html.contains("/static/icelines.svg"));
    }
}
