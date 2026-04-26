# IceLines Rust CLI — Phase 3 Implementation Plan

**Date**: 2026-04-25  
**Phase**: 3 of 3 — TUI, Projection Engine, Shift Data, Tonight  
**Spec references**:
- `docs/specs/rust-cli.md` — command surface, crate architecture, `tonight`, `schedule`, `trade`, `project`, `tui`
- `docs/specs/tui.md` — ratatui TUI, 8 screens, event loop, widget catalog, color contract, navigation model
- `docs/specs/projection-engine.md` — pace/regressed/composite modes, age factor table, schedule factor, confidence band, career PPG derivation
- `docs/specs/player-analysis.md` — `mates`, `scouting` command specs, `ShiftProfile`, linemate output format
- `docs/specs/position-engine.md` — boxscore cache layout (shared with shift fetcher)
- `docs/specs/fantasy-scheme.md` — scheme integration with `project` command (`--scheme` flag)
- `docs/specs/dashboard-engine.md` — `shifts` query source (Tier 3 data)

**Companion plans**:
- `docs/plans/2026-04-25-rust-cli-foundation.md` — Phase 1: workspace, icelines-core, icelines-fetch, `team` and `rank`
- `docs/plans/2026-04-25-phase2-site-analysis.md` — Phase 2: site generation, schemes, player analysis

---

## Background

Phase 2 delivered the complete site generation pipeline, fantasy scheme engine, dashboard engine,
player analysis commands (`players`, `class`, `peers`, `compare`, `group`, `history`), and the
SQLite database layer. The Python scripts are fully retired.

Phase 3 completes the IceLines feature set with three major additions:

**Projection engine**: Rest-of-season projections at three sophistication levels — pace (simple
PPG extrapolation), regressed (career-weighted blend with credibility factor α), and composite
(adds age curve and schedule difficulty). The engine lives in `icelines-core` as pure computation
and powers both the `icelines project` command and the Projections screen in the TUI.

**Shift data**: The `icelines fetch shifts` pipeline fetches the NHL shiftchart API and game-log
endpoints to build per-player `ShiftProfile` records — who each player actually skated with, how
often, and how those combinations performed. This powers `icelines mates` (linemate analysis) and
enhances `icelines scouting` with shift-derived context.

**TUI**: The full-screen terminal interface launched by `icelines` (no arguments) or
`icelines tui`. Built with `ratatui` + `crossterm`, it exposes all Phase 1–3 analytics through
8 keyboard-navigable screens: Home, Team, Player, Search, Tonight, Projections, Groups, Fetch.
The TUI reads exclusively from the local cache; all fetch operations run in tokio background tasks
with live progress bars.

By the end of Phase 3, IceLines is a complete, distributable CLI tool. A release pipeline
produces platform binaries for Windows, macOS, and Linux via GitHub Actions.

---

## Goals

- Implement `ProjectionMode`, `ProjectionResult`, and the full three-mode projection engine in
  `icelines-core`: pace, regressed (career-weighted α blend), composite (age factor + schedule
  difficulty)
- Implement confidence band computation (`σ_ppg × √remaining_games`) from per-game point totals
- Implement `GameSchedule` and `RemainingGames` in `icelines-core`; implement the schedule API
  client in `icelines-fetch`
- Implement `icelines project <PLAYER>` and `icelines project --team <TEAM>` commands
- Implement `icelines tonight` and `icelines schedule` commands
- Implement `icelines trade <PLAYER_OUT> for <PLAYER_IN>` command
- Implement `Shift`, `ShiftProfile`, `compute_linemates()` in `icelines-core`; implement
  `icelines fetch shifts` (shiftchart API client and game-log fetcher) in `icelines-fetch`
- Implement `icelines mates <PLAYER>` using shift-derived linemate data
- Implement `icelines scouting <PLAYER>` — full 8-section scouting report
- Implement the full ratatui TUI: `App` state, event loop, all 8 screens, all 5 widget types
- Implement `cargo install` packaging and the release pipeline (`.github/workflows/release.yml`)
  for Windows, macOS, and Linux cross-compiled binaries

---

## File Map

Files to create, by crate. All Phase 1 and Phase 2 files remain; this table lists additions only.

### `icelines-core/`

| File | Description |
|------|-------------|
| `src/projection/mod.rs` | `ProjectionMode` enum (Pace, Regressed, Composite); `ProjectionResult` struct with all fields from `projection-engine.md` §7: player_id, mode, alpha, current_ppg, career_ppg, remaining_games, projected_remaining, sigma, age_factor, schedule_factor |
| `src/projection/pace.rs` | `pace_project(current_ppg: f64, remaining_games: u32) -> f64` — `current_ppg × remaining_games`; no career data required |
| `src/projection/regressed.rs` | `compute_alpha(season_gp: u32) -> f64` — `min(season_gp as f64 / 50.0, 1.0)`; `regressed_project(current_ppg, career_ppg: Option<f64>, season_gp, remaining_games) -> (f64, f64)` — returns (proj_pts, alpha); falls back to pace when `career_ppg.is_none()` (rookie case) |
| `src/projection/composite.rs` | `age_factor(age: u8) -> f64` — lookup table per `projection-engine.md` §2.3 (0.92 at ≤22, 1.00 at 26–27, 0.87 at ≥35); `schedule_factor(remaining_opponents: &[TeamStrength], k: f64) -> f64` — `1.0 + k × mean(opponent_rank_deviation)`, default k=0.015; `composite_project(regressed_pts, age_factor, schedule_factor) -> f64` |
| `src/projection/confidence.rs` | `compute_sigma(per_game_points: &[u32], remaining_games: u32) -> Option<f64>` — returns `None` when fewer than 10 games; `std_dev(per_game_points) × sqrt(remaining_games as f64)` |
| `src/schedule.rs` | `GameSchedule { team: TeamAbbr, games: Vec<ScheduledGame> }`; `ScheduledGame { date: NaiveDate, opponent: TeamAbbr, is_home: bool, game_id: Option<u64> }`; `RemainingGames { team: TeamAbbr, count: u32, games: Vec<ScheduledGame> }`; `compute_remaining(schedule: &GameSchedule, today: NaiveDate) -> RemainingGames`; `TeamStrength { team: TeamAbbr, goals_against_per_game: f32, rank: u8 }` |
| `src/shift.rs` | `Shift { player_id: u32, partner_id: u32, game_id: u64, shared_seconds: u32 }`; `ShiftProfile { player_id: u32, season: u32, linemates: Vec<LinemateRecord> }`; `LinemateRecord { partner_id: u32, shared_shifts: u32, shared_toi_seconds: u32, gf_together: u32, ga_together: u32 }`; `compute_linemates(shifts: &[Shift], min_shared_shifts: u32) -> Vec<LinemateRecord>` — groups by partner_id, sums shared_shifts and shared_toi, filters by threshold, sorts by shared_shifts desc |
| `src/lib.rs` | Add `pub mod projection`, `pub mod schedule`, `pub mod shift` |

### `icelines-fetch/`

| File | Description |
|------|-------------|
| `src/shifts.rs` | `ShiftchartEntry { team_abbrev: String, period: u8, start_time: String, end_time: String, duration: String, player_id: u32 }`; `fetch_shiftchart(game_id: u64, client, cache) -> Result<Vec<ShiftchartEntry>, Error>` — `GET api-web.nhle.com/v1/shiftcharts/{GAME_ID}`; caches at `~/.icelines/cache/shifts/{SEASON}/{GAME_ID}.json`; completed games never expire; `parse_shift_overlap(entries: &[ShiftchartEntry]) -> Vec<Shift>` — for each pair of players on the same team in the same period, computes the time overlap in seconds from their start/end times |
| `src/schedule.rs` | `ScheduleApiResponse` serde struct for `api-web.nhle.com/v1/schedule/{DATE}`; `fetch_schedule_for_date(date: NaiveDate, client, cache) -> Result<Vec<ScheduledGame>, Error>` — 6-hour TTL; `fetch_remaining_schedule(team: &TeamAbbr, today: NaiveDate, season_end: NaiveDate, client, cache) -> Result<GameSchedule, Error>` — iterates dates from today through season_end, aggregates all games for the given team; cache per team at `~/.icelines/cache/schedule/{SEASON}/{TEAM}.json` |
| `src/lib.rs` | Add `pub mod shifts`, `pub mod schedule` |

### `icelines-cli/src/tui/`

| File | Description |
|------|-------------|
| `mod.rs` | `run_tui(config: &Config) -> Result<(), anyhow::Error>` — public entry point; sets up crossterm raw mode and alternate screen, creates `App`, runs the event loop, restores terminal on exit (including panic hook to ensure cleanup); delegates to `event::run_event_loop` |
| `app.rs` | `App` struct with all fields from `tui.md` §4: `screen_stack: Vec<Screen>`, `teams`, `players`, `position_profiles`, `tonight_games`, `search_query`, `search_active`, `search_results`, `selected_row: HashMap<Screen, usize>`, `show_help`, `fetch_progress: Option<FetchProgress>`, `color_mode: ColorMode`; `Screen` enum (Home, Team(TeamAbbr), Player(u32), Search, Tonight, Projections(Option<TeamAbbr>), Groups, Fetch); `ColorMode` enum (Full, NoColor); `App::new(config) -> Self`; `App::current_screen(&self) -> &Screen`; `App::push(screen)`, `App::pop()` |
| `event.rs` | `AppEvent` enum from `tui.md` §5 (Up, Down, Left, Right, Enter, Esc, Quit, Search, Refresh, Help, Char(char), Backspace, Resize(u16, u16)); `run_event_loop(terminal, app, data_rx) -> Result<(), anyhow::Error>` — 16ms crossterm poll (≈60fps), drain `data_rx` mpsc channel, dispatch to current screen handler, call `terminal.draw(render)`, check `should_quit`; `map_crossterm_event(event: crossterm::event::Event) -> Option<AppEvent>` |

### `icelines-cli/src/tui/screens/`

| File | Description |
|------|-------------|
| `mod.rs` | `pub mod home`, `pub mod team`, `pub mod player`, `pub mod search`, `pub mod tonight`, `pub mod projections`, `pub mod groups`, `pub mod fetch`; shared `render(frame: &mut Frame, app: &App)` dispatcher that calls the correct screen's render function based on `app.current_screen()` |
| `home.rs` | `render_home(frame, app)` — two-column 16-row table of all 32 teams: Rank, Team abbreviation (colored by dominant fit class), mean pace pts/82, E/S/B counts; `handle_home_event(event: AppEvent, app: &mut App)` — Up/Down moves selection, Left/Right switches columns, Enter pushes `Screen::Team`, `/` pushes `Screen::Search`; `f` key pushes `Screen::Fetch` |
| `team.rs` | `render_team(frame, app, team_abbr)` — 4×3 forward grid and 3×2 defense grid using `PlayerCell` widgets; below-MIN_GP section at bottom; `handle_team_event(event, app)` — arrow keys move grid cursor, Enter pushes `Screen::Player(player_id)`, Esc pops screen |
| `player.rs` | `render_player(frame, app, player_id)` — split layout: left panel (this season stats, projection panel, position profile), right panel (career trend `Sparkline`, peers list); `handle_player_event(event, app)` — `p` key cycles projection mode (pace → regressed → composite), recomputes projection from cached data without re-fetch; Esc pops screen |
| `search.rs` | `render_search(frame, app)` — search input line at top, results table below; `handle_search_event(event, app)` — `Char(c)` appends to `search_query` and filters `app.players` in-memory (case-insensitive substring on `name` and `name_normalized`); Backspace trims query; Enter on non-empty results pushes `Screen::Player`; Esc pops screen and clears query |
| `tonight.rs` | `render_tonight(frame, app)` — scrollable list of today's games, each showing game time, teams, and projected F1–F4 + D1–D3 lines from most recent cached boxscore; `handle_tonight_event(event, app)` — Up/Down navigates games, Enter on highlighted game pushes `Screen::Team` for highlighted team, `r` key triggers background re-fetch of today's schedule |
| `projections.rs` | `render_projections(frame, app, team)` — ranked table: Rank, Player, Pos, GP, Pts, PPG, α, Proj, ±1σ range; team total footer; `handle_projections_event(event, app)` — `t` opens inline team picker (searchable list), `m` cycles projection mode, `p` opens position filter inline, Enter pushes `Screen::Player`, Esc pops |
| `groups.rs` | `render_groups(frame, app)` — list of all saved player groups with name, member count, created date; opening a group shows a player rank table; `handle_groups_event(event, app)` — Enter opens group member list, `n` key opens inline text field for new group name, `d` deletes highlighted group with confirmation prompt, Esc pops |
| `fetch.rs` | `render_fetch(frame, app)` — cache status table (Data Set, Status, Last Fetched, Size) plus active progress bar when a fetch is running; `handle_fetch_event(event, app)` — `r` triggers stale-data fetch via background tokio task, `R` force-refreshes all, `p`/`s`/`b`/`l` trigger targeted fetches; progress updates arrive via the data mpsc channel |

### `icelines-cli/src/tui/widgets/`

| File | Description |
|------|-------------|
| `mod.rs` | `pub mod player_cell`, `pub mod team_card`, `pub mod progress_bar`, `pub mod help_overlay`, `pub mod sparkline` |
| `player_cell.rs` | `PlayerCell<'a> { player: &'a Player, selected: bool, color_mode: ColorMode }` implementing `ratatui::widgets::Widget`; renders 20-char truncated name, `XX.X pts/82`, `GP N` on two lines; background `Style` set to fit class color in `Full` mode; `[ELITE]`/`[SOLID]`/`[BURIED]`/`[STRETCH]` appended in `NoColor` mode |
| `team_card.rs` | `TeamCard<'a> { team: &'a TeamSummary, rank: usize, selected: bool, color_mode: ColorMode }` implementing `Widget`; 4-line compact widget: rank + team abbreviation (line 1), mean pts/82 (line 2), E/S/B/X counts (line 3), divider (line 4); background reflects dominant fit class |
| `progress_bar.rs` | `ProgressBar<'a> { label: &'a str, total: u64, completed: u64, eta_secs: Option<u64> }` implementing `Widget`; renders filled (█) and empty (░) segments, percentage, ETA string formatted as `Xm Ys`; adapts width to available terminal columns |
| `help_overlay.rs` | `HelpOverlay<'a> { bindings: &'a [(&'a str, &'a str)] }` implementing `Widget`; renders a bordered modal box centered over the current frame; each binding pair shown as `key` — `description`; press `?` or `Esc` to dismiss; overlay is rendered last so it appears on top |
| `sparkline.rs` | `Sparkline<'a> { seasons: &'a [SeasonBar] }` where `SeasonBar { label: String, pts_82: f64, gp: u32, is_short: bool }`; wraps `ratatui::widgets::BarChart`; bar color derived from fit class thresholds (≥65 green, ≥40 yellow, ≥20 blue, <20 red); short/COVID seasons rendered with `Modifier::DIM`; x-axis labels are two-digit season years |

### `icelines-cli/src/commands/`

| File | Description |
|------|-------------|
| `project.rs` | `run_project_player(player, args, config)` — resolves player; loads current stats from cache; fetches `PlayerLanding` via `career.rs` for career PPG; fetches game log for per-game point totals (confidence band); fetches remaining schedule for the player's team; calls `pace_project`, `regressed_project`, or `composite_project` per `--mode`; renders single-player output per `projection-engine.md` §5 example; `--season` flag renders career comparison table; `--json` outputs `ProjectionResult` as JSON; `run_project_team(team, args, config)` — runs single-player projection for each skater above MIN_GP on the team, sorts by projected_remaining desc, renders team table with total footer per §6 |
| `tonight.rs` | `run_tonight(args, config)` — fetches today's schedule from `schedule/now`; for each game, loads the most recent cached boxscore for each team; extracts forward lines and defense pairs from boxscore `playerByGameStats`; renders compact side-by-side view per `rust-cli.md` `tonight` example; `--team` filter; `--json` output |
| `trade.rs` | `run_trade(player_out, player_in, args, config)` — resolves both players; identifies `player_out`'s current team (or uses `--team`); builds the team's depth chart; removes `player_out` from depth chart, inserts `player_in` using depth chart builder algorithm (primary position first, then eligible positions); renders before/after diff with pace comparison, team total delta, and verdict line per `rust-cli.md` `trade` example; exits with error if `player_in` is not in cached data |
| `mates.rs` | `run_mates(player, args, config)` — resolves player; loads `ShiftProfile` from db or cache for the player; filters linemates by `--min-shifts` threshold; fetches partner names from cached player records; renders linemate table per `player-analysis.md` `mates` example with shared shifts, ES-TOI together, GF%, xGF% (xGF shown as `N/A` if unavailable); `--top <N>` limits output rows |
| `scouting.rs` | `run_scouting(player, args, config)` — aggregates all available data for one player and renders the 8-section scouting report: (1) Bio — age, nationality, draft, hand; (2) Current season pace stats; (3) Career trajectory — 3-year pace trend from `SeasonHistory`; (4) Peer group rank — draft class percentile via peer algorithm; (5) Linemate analysis — primary line partners from `ShiftProfile` if available, else `"shift data not available"`; (6) Depth chart position on own team; (7) Cross-team value — mean line position on other 31 teams; (8) Fit classification and interpretation; `--format terminal|markdown|json`; `--out <FILE>` writes to file |

### `.github/workflows/`

| File | Description |
|------|-------------|
| `.github/workflows/release.yml` | GitHub Actions release pipeline: triggers on `push` of version tags matching `v[0-9]+.[0-9]+.[0-9]+`; matrix: `x86_64-unknown-linux-musl` (ubuntu-latest), `x86_64-apple-darwin` + `aarch64-apple-darwin` (macos-latest), `x86_64-pc-windows-msvc` (windows-latest); steps: checkout, install toolchain via `dtolnay/rust-toolchain@stable`, install cross-compilation targets, `cargo build --release --target ${{ matrix.target }}`; upload binary to GitHub Release via `softprops/action-gh-release@v2`; binary named `icelines-{target}.{exe}` on Windows, `icelines-{target}` on others |

---

## Phase Breakdown

### Phase 1 — Projection Engine Core Types and Pace Mode

- [ ] Add `chrono` to `icelines-core/Cargo.toml` if not already present from Phase 2
- [ ] Create `icelines-core/src/projection/` module directory
- [ ] Implement `src/projection/mod.rs`: `ProjectionMode` enum (Pace, Regressed, Composite) with `Display` impl showing "Pace", "Regressed", "Composite"; `ProjectionResult` struct with all fields from spec §7 — all `Option<f64>` fields that are mode-specific; derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] Implement `src/projection/pace.rs`: `pace_project(current_ppg: f64, remaining_games: u32) -> f64` — single multiplication, always returns a value; document: "No regression applied — projects current pace forward linearly"
- [ ] Implement `src/projection/confidence.rs`: `compute_sigma(per_game_points: &[u32], remaining_games: u32) -> Option<f64>` — returns `None` when `per_game_points.len() < 10`; `std_dev` = sqrt(Σ(x − mean)² / n); sigma = std_dev × sqrt(remaining_games as f64)
- [ ] Implement `src/schedule.rs`: `GameSchedule`, `ScheduledGame`, `RemainingGames`, `TeamStrength`; `compute_remaining(schedule: &GameSchedule, today: NaiveDate) -> RemainingGames` — filters `schedule.games` to those with `date >= today` and `game_id.is_none()` (not yet played)
- [ ] Write unit tests in `src/projection/pace.rs`:
  - `pace_project(1.288, 30) == 38.64` (Draisaitl example, allow ε=0.01)
  - `pace_project(0.0, 30) == 0.0`
  - `pace_project(1.000, 0) == 0.0`
- [ ] Write unit tests in `src/projection/confidence.rs`:
  - `compute_sigma(&[], 30)` returns `None`
  - `compute_sigma(&[0; 9], 30)` returns `None` (exactly 9 games — below threshold)
  - `compute_sigma(&[0; 10], 30)` returns `Some(0.0)` (all zeros — no variance)
  - Known std_dev case: `[1, 0, 1, 0, 1, 0, 1, 0, 1, 0]` → std_dev=0.5, sigma=0.5×√30 ≈ 2.739 (document expected value)
- [ ] Write unit tests in `src/schedule.rs`:
  - `compute_remaining` returns empty when all games are in the past
  - Correctly counts games from today onward
- [ ] Verify: `cargo test -p icelines-core` passes

### Phase 2 — Regressed and Composite Modes, Schedule Integration

- [ ] Implement `src/projection/regressed.rs`:
  - `compute_alpha(season_gp: u32) -> f64` — `(season_gp as f64 / 50.0).min(1.0)`
  - `regressed_project(current_ppg: f64, career_ppg: Option<f64>, season_gp: u32, remaining_games: u32) -> (projected_pts: f64, alpha: f64, used_fallback: bool)` — when `career_ppg.is_none()`, sets `used_fallback=true` and uses `pace_project`; otherwise computes `proj_ppg = α × current_ppg + (1-α) × career_ppg.unwrap()`, then `proj_ppg × remaining_games`
- [ ] Implement `src/projection/composite.rs`:
  - `age_factor(age: u8) -> f64` — exhaustive match over the age table from `projection-engine.md` §2.3; handles `age <= 22` and `age >= 35` arms
  - `schedule_factor(remaining_opponents: &[TeamStrength], k: f64) -> f64` — `1.0 + k × mean_deviation` where `mean_deviation = mean(opponent.rank as f64 − league_median_rank)`; returns 1.0 for empty slice
  - `composite_project(regressed_pts: f64, age: u8, remaining_opponents: &[TeamStrength]) -> (f64, f64, f64)` — returns (composite_pts, age_factor, schedule_factor); calls `age_factor(age)` and `schedule_factor(remaining_opponents, 0.015)`
- [ ] Implement `icelines-fetch/src/schedule.rs`:
  - `ScheduleApiDay` serde struct for `api-web.nhle.com/v1/schedule/{DATE}` response — `game_week: Vec<ScheduleDay>`, each day has `date: String` and `games: Vec<ScheduleGame>`
  - `ScheduleGame { id: u64, season: u32, game_type: u8, game_date: String, start_time_utc: String, away_team: ScheduleTeam, home_team: ScheduleTeam }`
  - `fetch_schedule_for_date(date: NaiveDate, client, cache) -> Result<Vec<ScheduledGame>, Error>` — 6-hour TTL; returns only `game_type == 2` (regular season) games; maps `away_team` and `home_team` to `ScheduledGame`
  - `fetch_remaining_schedule(team: &TeamAbbr, today: NaiveDate, season_end: NaiveDate, client, cache) -> Result<GameSchedule, Error>` — iterates dates, collects games involving `team`, caches complete result per team at `~/.icelines/cache/schedule/{SEASON}/{TEAM}.json`
- [ ] Write unit tests in `src/projection/regressed.rs`:
  - `compute_alpha(0) == 0.0`
  - `compute_alpha(25) == 0.5`
  - `compute_alpha(50) == 1.0`
  - `compute_alpha(82) == 1.0` (clamped)
  - Regressed project with career_ppg: GP=52, current_ppg=1.288, career_ppg=1.241, α=1.00 → proj=1.288×30=38.64 (α=1.0 → identical to pace, documented in spec)
  - Rookie fallback: career_ppg=None → used_fallback=true, proj equals pace result
- [ ] Write unit tests in `src/projection/composite.rs`:
  - `age_factor(22) == 0.92`
  - `age_factor(27) == 1.00`
  - `age_factor(30) == 0.97`
  - `age_factor(35) == 0.87`
  - `schedule_factor(&[], 0.015) == 1.0` (empty slice)
  - `schedule_factor` with all opponents at median rank → 1.0
- [ ] Verify: `cargo test -p icelines-core` passes; all projection tests document expected values with calculation comments

### Phase 3 — `icelines project`, `icelines tonight`, `icelines trade` Commands

- [ ] Implement `src/commands/project.rs` in `icelines-cli`:
  - `run_project_player(player, args, config)`:
    - Resolve player name via `PlayerResolver`
    - Load current-season stats from cache (or call `icelines fetch stats` error if not cached)
    - Fetch `PlayerLanding` via `career.rs` → compute `career_ppg` via `compute_career_ppg()`
    - Fetch game log from `boxscore.rs` → extract per-game point totals (goals + assists per game) → `compute_sigma()`
    - Fetch remaining schedule via `schedule.rs` → `compute_remaining()`
    - Compute projection in requested mode; build `ProjectionResult`
    - Render terminal output matching `projection-engine.md` §5 example format exactly: bio line, mode line, career PPG line, remaining games line, projected pts with range, full-season total
    - `--season` flag: append career comparison table (season, team, G, A, Pts, GP, PPG)
    - `--json`: serialize `ProjectionResult` per §5 JSON example
  - `run_project_team(team, args, config)`:
    - Load all active skaters for the team from cache
    - Filter to players above MIN_GP
    - Run single-player projection for each (re-use career + schedule data where possible)
    - Sort by `projected_remaining` descending
    - Render team table per `projection-engine.md` §6 example: Rank, Player, Pos, GP, Pts, PPG, α, Proj, Range
    - Team total footer: sum of all `projected_remaining`, combined σ = √(Σσ²)
- [ ] Implement `src/commands/tonight.rs`:
  - `run_tonight(args, config)`:
    - Fetch today's schedule (`/v1/schedule/now`)
    - For each game: load most recent cached boxscore for each team; extract `forwards` and `defense` arrays; group by `toi` descending to infer line number (first 3 forwards by TOI = F1, next 3 = F2, etc.)
    - Render per `rust-cli.md` tonight example: game time, teams, projected lines side-by-side, caveat label
    - `--team <ABBREV>` filter: only show games involving that team
    - `--json`: array of game objects with `game_time`, `away_team`, `home_team`, projected lines, `data_source` (game_id of the boxscore used)
  - `run_schedule(args, config)`:
    - Fetch schedule for today through today + `--days` (default 7)
    - Render table: date, time, away, home per `rust-cli.md` schedule example
    - `--team` filter; `--json` output
- [ ] Implement `src/commands/trade.rs`:
  - `run_trade(player_out, player_in, args, config)`:
    - Resolve both players; error if `player_in` not in cached data with suggestion to run `icelines fetch stats`
    - Identify team from `player_out`'s cached record (or `--team` override)
    - Build "before" depth chart for the team
    - Build "after" depth chart: remove `player_out`, insert `player_in` using depth chart builder algorithm
    - Compute team pace totals before and after (sum of all `pace_82` values for rostered skaters above MIN_GP)
    - Render before/after diff per `rust-cli.md` trade example: player lines with pace values, delta, team total before/after, Verdict line
- [ ] Add `Schedule` and `Tonight` variants to the clap `Commands` enum in `src/cli.rs`; add `--games`, `--season`, `--mode` flags to `Project` subcommand; add `for` syntax to `Trade` subcommand (using clap's multi-arg support)
- [ ] Verify: `cargo run -- project "Leon Draisaitl"` produces output matching the spec example format; `cargo run -- tonight` renders today's schedule without errors (or graceful empty message if no games today); `cargo run -- trade "Jesse Puljujarvi" for "Rickard Rakell"` renders trade diff

### Phase 4 — Shift Data Fetch and ShiftProfile Computation

- [ ] Implement `icelines-fetch/src/shifts.rs`:
  - `ShiftchartEntry` serde struct matching `api-web.nhle.com/v1/shiftcharts/{GAME_ID}` response — `data: Vec<ShiftRow>` where `ShiftRow { playerId: u32, teamAbbrev: String, period: u8, startTime: String, endTime: String, duration: String }`
  - `fetch_shiftchart(game_id: u64, client, cache) -> Result<Vec<ShiftchartEntry>, Error>` — caches at `~/.icelines/cache/shifts/{SEASON}/{GAME_ID}.json`; completed games never expire
  - `parse_time_to_seconds(time: &str) -> Result<u32, Error>` — converts `"MM:SS"` to total seconds; errors on malformed input
  - `compute_shift_overlaps(entries: &[ShiftchartEntry]) -> Vec<Shift>` — for each pair of players on the same team in the same period: compute intersection = min(end_A, end_B) − max(start_A, start_B) in seconds; emit a `Shift` record when intersection > 0; O(n²) per period is acceptable (≤25 players per team per period)
  - `fetch_shifts_for_all_games(game_ids: &[u64], season: u32, client, cache) -> Result<Vec<Shift>, Error>` — fetches each game's shiftchart (deduplicated), parses overlaps, returns flat list
- [ ] Extend `icelines-cli/src/commands/fetch.rs`: add `shifts` subcommand variant — loads all player game logs from boxscore cache, collects unique game IDs for the season, calls `fetch_shifts_for_all_games`, computes `ShiftProfile` for each player via `compute_linemates()`, writes each profile to db (`upsert_shift_profile`) and to `~/.icelines/cache/shifts/{SEASON}/profiles/{PLAYER_ID}.json`
- [ ] Add `upsert_shift_profile(conn, profile: &ShiftProfile) -> Result<(), anyhow::Error>` and `load_shift_profile(conn, player_id: u32, season: u32) -> Result<Option<ShiftProfile>, anyhow::Error>` to `src/db.rs`; add Migration 004: `shift_profiles` and `linemate_records` tables
- [ ] Write unit tests in `src/shifts.rs`:
  - `parse_time_to_seconds("05:32") == 332`
  - `parse_time_to_seconds("00:00") == 0`
  - `parse_time_to_seconds("invalid")` returns `Err`
  - `compute_shift_overlaps` with two players both on-ice from 0:00–1:00 → overlap = 60 seconds
  - `compute_shift_overlaps` with non-overlapping shifts → no `Shift` records emitted
  - `compute_linemates` with known fixture: player A shared 412 shifts with player B → `LinemateRecord { partner_id: B, shared_shifts: 412, ... }`
- [ ] Add test fixture `tests/fixtures/api/shiftchart_sample.json` with realistic shift data for two known player IDs
- [ ] Verify: `cargo test -p icelines-fetch` passes; `cargo run -- fetch shifts` processes the sample fixture without panic

### Phase 5 — TUI App Skeleton, Event Loop, and Home Screen

- [ ] Add `ratatui` and `crossterm` to `icelines-cli/Cargo.toml`
- [ ] Create `src/tui/` directory structure: `mod.rs`, `app.rs`, `event.rs`, `screens/mod.rs`, `widgets/mod.rs`
- [ ] Implement `src/tui/app.rs`:
  - `App::new(config)` — loads teams and players from cache (sync, since the main thread starts before the event loop); initializes `screen_stack` with `[Screen::Home]`; sets `color_mode` from `config.no_color` or `NO_COLOR` env var
  - `App::push(screen)` — appends to `screen_stack`; no limit on stack depth
  - `App::pop()` — removes last element if stack length > 1; at length 1 (Home) sets `should_quit = true`
  - `App::current_screen()` — `screen_stack.last().unwrap()` (invariant: stack is never empty)
  - `App::should_quit` field; `App::handle_global_event(event) -> bool` — handles `Quit`, `Esc` at Home, `Help` toggle; returns true if consumed
- [ ] Implement `src/tui/event.rs`:
  - `map_crossterm_event(event: crossterm::event::Event) -> Option<AppEvent>` — maps `KeyCode::Up` → `AppEvent::Up`, etc.; maps `KeyCode::Char('q')` → `AppEvent::Quit`, `KeyCode::Char('/')` → `AppEvent::Search`, `KeyCode::Char('?')` → `AppEvent::Help`, `KeyCode::Char('r')` → `AppEvent::Refresh`; `KeyCode::Char(c)` → `AppEvent::Char(c)` for all other chars (used in search mode)
  - `run_event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App, data_rx: &mut mpsc::Receiver<DataMessage>) -> Result<(), anyhow::Error>` — the main loop: poll with 16ms timeout, drain channel, update app, draw, check `should_quit`
- [ ] Implement `src/tui/mod.rs`:
  - `run_tui(config: &Config) -> Result<(), anyhow::Error>` — install panic hook that calls `crossterm::terminal::disable_raw_mode()` and `execute!(stdout, LeaveAlternateScreen)` before the panic message; enter raw mode + alternate screen; create `Terminal<CrosstermBackend<Stdout>>`; create `App`; create tokio mpsc channel for background data; call `run_event_loop`; on return, restore terminal
- [ ] Implement `src/tui/screens/home.rs`:
  - `render_home(frame: &mut Frame, app: &App)` — outer `Layout::vertical([Constraint::Min(0), Constraint::Length(1)])` for main area and status bar; main area `Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])` for two columns; iterate teams sorted by mean pts/82 desc, fill column 0 with ranks 1–16 and column 1 with ranks 17–32; each row shows `Span::styled(team.abbr, style)` colored by dominant fit class; Pts/82 in dim white; E/S/B counts in compact format; selected row highlighted with `Style::default().add_modifier(Modifier::REVERSED)`
  - `handle_home_event(event: AppEvent, app: &mut App)` — Up/Down adjusts `app.selected_row[Screen::Home]` clamped to 0..31; Left/Right toggles column focus; Enter: push `Screen::Team(teams[selected].abbr.clone())`; `AppEvent::Char('f')`: push `Screen::Fetch`; `AppEvent::Search`: push `Screen::Search`
- [ ] Implement all five widgets (`player_cell.rs`, `team_card.rs`, `progress_bar.rs`, `help_overlay.rs`, `sparkline.rs`) per spec §8 widget catalog
- [ ] Implement `src/tui/screens/mod.rs` dispatch function `render(frame, app)` that matches on `app.current_screen()` and calls the appropriate render function; if `app.show_help`, render `HelpOverlay` on top of whatever the current screen renders
- [ ] Wire `icelines tui` and bare `icelines` (no args) in `src/main.rs` to call `run_tui(config)`
- [ ] Manual smoke test: `cargo run -- tui` enters the TUI, Home screen renders 32 teams in two columns, `q` exits cleanly and restores terminal
- [ ] Verify: terminal is always restored on panic — test by intentionally panicking in a branch and confirming shell is usable afterward

### Phase 6 — Remaining TUI Screens (Team, Player, Search, Tonight, Projections, Groups, Fetch)

- [ ] Implement `src/tui/screens/team.rs`:
  - Load `DepthChart` for the team from `app.players` filtered by team and position
  - Render 4×3 grid of `PlayerCell` widgets and 3×2 defense grid using `ratatui::layout::Layout` with equal-width constraints per column
  - Below-MIN_GP section: `Block::new().title("Below MIN_GP")` with a simple list
  - Selection cursor: a `u8` pair `(row, col)` stored in `app.selected_row` for Team screens; highlighted with reversed style
  - Enter: look up `player_id` from the selected grid cell and push `Screen::Player(player_id)`
- [ ] Implement `src/tui/screens/player.rs`:
  - Left panel: `Paragraph` widgets for each data section (This Season, Projection, Position Profile)
  - Right panel: `Sparkline` widget for career trend, `List` widget for peers (same team, same position group, above MIN_GP)
  - `p` key: cycle `app.player_projection_mode` (a new field in `App` — `Option<ProjectionMode>` defaulting to Regressed); recompute projection inline from cached data (no I/O — all data already in `app.players` and `app.histories`)
  - If career history data not loaded: show `"(press r to load career data)"` and trigger background fetch on `r`
- [ ] Implement `src/tui/screens/search.rs`:
  - Search input rendered as `Paragraph` with the current `app.search_query` and a cursor character
  - Filter `app.players` in-memory on every `AppEvent::Char` and `AppEvent::Backspace` event — no debouncing needed at ~800 player scale
  - Results rendered as a `Table` widget with rank, name, team, pos, GP, PPG, fit label/color
  - Enter on a result: push `Screen::Player(result.player_id)`
- [ ] Implement `src/tui/screens/tonight.rs`:
  - On screen push: if `app.tonight_games` is `None`, trigger background fetch via data channel; render `"Loading schedule..."` spinner (`|`, `/`, `-`, `\` cycling each tick) until data arrives
  - Once loaded: render scrollable `List` of game entries; each entry occupies ~10 lines (game header + 7 line rows); `Scrollbar` widget on the right edge
  - `r` key: clears `app.tonight_games`, triggers re-fetch
- [ ] Implement `src/tui/screens/projections.rs`:
  - On screen push with a team: compute projections for all team skaters above MIN_GP using cached data (sync — no I/O); store results in `App` field `projection_results: Vec<ProjectionResult>`
  - `t` key: render inline `Popup` (centered block over main area) with searchable list of 32 teams; selecting a team updates `App` and recomputes projections
  - `m` key: cycle through `ProjectionMode::Pace → Regressed → Composite`; recompute immediately
  - `p` key: render inline position filter popup; `All | Forwards | Defense | C | LW | RW | D`
  - Team total footer always visible at bottom of screen
- [ ] Implement `src/tui/screens/groups.rs`:
  - Load groups from db on screen push; render `Table` with name, count, date
  - Enter: load group members from db, compute current stats for each member from `app.players`, push a sub-view (same screen, different render mode) showing member rank table
  - `n` key: render inline text input for group name; on Enter, save to db via background task, refresh groups list
  - `d` key: render confirmation dialog `"Delete group 'X'? [y/N]"`; on `y`, delete from db
- [ ] Implement `src/tui/screens/fetch.rs`:
  - On render: stat each expected cache file, compute age, compare to TTL to determine STALE/OK/MISSING; render `Table` with status indicators
  - `r` key: trigger stale-data fetch background task via channel; render `ProgressBar` widget at bottom as updates arrive
  - `R` key: trigger force-refresh (all data sets, ignoring TTL)
  - `p`, `s`, `b`, `l` keys: trigger targeted fetch for positions, stats, boxscores, player landings respectively
  - Background fetch tasks use `tokio::task::spawn` and send progress updates as `DataMessage::FetchProgress { dataset, completed, total }` through the channel
- [ ] Run full TUI regression: all 8 screens navigate correctly; `Esc` from any non-Home screen returns to previous screen; `q` from Home quits; help overlay appears and dismisses; no terminal corruption on exit
- [ ] Verify: `cargo build --release` produces binary under 20 MB (if over, audit feature flags on ratatui/crossterm/rusqlite)

### Phase 7 — `icelines mates`, `icelines scouting`, Release Pipeline, and Packaging

- [ ] Implement `src/commands/mates.rs`:
  - `run_mates(player, args, config)`:
    - Resolve player
    - Load `ShiftProfile` from db (`load_shift_profile`) — if not present, error with `"No shift data found — run 'icelines fetch shifts' first"`
    - Filter linemates by `--min-shifts` (default 50)
    - Limit to `--top N` (default 5) by shared_shifts desc
    - Resolve partner names from `app.players` (cached)
    - Compute ES-TOI together: `shared_toi_seconds / 60` as minutes + seconds
    - Compute GF%: `gf_together / (gf_together + ga_together)` — show `N/A` if both are 0 (data not available)
    - Render table per `player-analysis.md` mates example; footer: primary line (top 3 partners)
- [ ] Implement `src/commands/scouting.rs`:
  - `run_scouting(player, args, config)`:
    - Resolve player; assemble all 8 sections:
    - §1 Bio: from `PlayerBio` — age (computed from birth_date), nationality (ISO alpha-3), draft info (round, pick, drafting team), hand
    - §2 Current season: pace stats (G, A, Pts, GP, PPG, pace_82, fit class)
    - §3 Career trajectory: load `SeasonHistory`, compute 3-year rolling pace, show trend arrow (▲ improving, ▼ declining, → stable — based on regression slope over last 3 seasons)
    - §4 Peer group rank: use draft class ±1 year peer method; compute player's percentile in group
    - §5 Linemate analysis: if `ShiftProfile` available, list top 3 linemates with shared TOI; else `"(shift data not loaded — run icelines fetch shifts)"`
    - §6 Depth chart: load team depth chart, identify which forward line or defense pair the player occupies
    - §7 Cross-team value: for each of the other 31 teams, simulate placing the player on that team's depth chart; compute the mean line number (1–4) across all teams; compare to current line assignment
    - §8 Fit classification: restate the fit class with a one-sentence interpretation matching the threshold rationale from `rust-cli.md` §5.2
    - `--format terminal`: render with section headers and comfy-table sub-tables
    - `--format markdown`: render as a markdown document with `##` section headers
    - `--format json`: serialize all sections as a flat JSON object
    - `--out <FILE>`: write to file instead of stdout; infer format from file extension if not specified
- [ ] Create `.github/workflows/release.yml`:
  - Trigger: `push` of tags matching `v[0-9]+.[0-9]+.[0-9]+`
  - `create-release` job: `softprops/action-gh-release@v2` with `draft: true`; extracts version from tag name for release title
  - Build matrix with `include` items: `{ target: x86_64-unknown-linux-musl, os: ubuntu-latest }`, `{ target: x86_64-apple-darwin, os: macos-latest }`, `{ target: aarch64-apple-darwin, os: macos-latest }`, `{ target: x86_64-pc-windows-msvc, os: windows-latest }`
  - Steps per matrix job: `actions/checkout@v4`; `dtolnay/rust-toolchain@stable` with `targets: ${{ matrix.target }}`; `Swatinem/rust-cache@v2`; install `musl-tools` on linux for musl target; `cargo build --release --target ${{ matrix.target }}`; rename binary to `icelines-${{ matrix.target }}` (`.exe` on windows); upload to release via `softprops/action-gh-release@v2` with `files:` pointing to renamed binary
- [ ] Update `Cargo.toml` workspace manifest with `[workspace.package]` section: `version`, `authors`, `license`, `repository` (GitHub URL); propagate to all four crate `Cargo.toml` files using `version.workspace = true`
- [ ] Add `[package] name = "icelines"` in `icelines-cli/Cargo.toml` with `[[bin]] name = "icelines" path = "src/main.rs"`
- [ ] Verify `cargo install --path icelines-cli` installs successfully from a clean checkout
- [ ] Run full integration test suite: `cargo test --workspace`
- [ ] Run `cargo clippy --workspace -- -D warnings` — zero warnings
- [ ] Run `cargo fmt --all --check` — no formatting diffs
- [ ] Tag `v0.1.0` and verify the release workflow produces binaries for all four targets in the GitHub Release draft
- [ ] Update `context/waves/WAVE02-APPMANAGER-ADOPTION.md` and `STATUS.md` with Phase 3 completion status

---

## Success Criteria

The following conditions must all be true before this plan is considered complete:

1. `cargo build --release` produces a single `icelines` binary with no errors
2. `cargo test --workspace` passes with zero failures
3. `cargo clippy --workspace -- -D warnings` produces zero warnings
4. `icelines tui` enters the TUI, renders the Home screen with all 32 teams in two columns, and exits cleanly with `q`
5. All 8 TUI screens are reachable via keyboard navigation: Home → Team → Player, Home → Search, Home → Fetch, `tonight` key → Tonight, `project` key → Projections, `groups` key → Groups
6. The TUI restores the terminal correctly on both clean exit and panic
7. `icelines project "Connor McDavid"` produces a formatted projection output matching the `projection-engine.md` example format (bio line, mode line, career PPG, remaining games, projected pts with range, full-season total)
8. `icelines project "Connor McDavid" --mode composite` produces age_factor and schedule_factor values consistent with the age table (age 28 → 0.99) and with `age_factor` and `schedule_factor` printed in output
9. `icelines project --team EDM` produces a ranked table for all Oilers skaters above MIN_GP with a team total footer
10. `icelines tonight` renders today's schedule (or a clean empty message if no games) without errors
11. `icelines schedule --team SEA --days 14` renders the next 14 days of Seattle Kraken games
12. `icelines trade "Jesse Puljujarvi" for "Rickard Rakell"` renders before/after depth chart diff with pace values, delta, team total change, and a Verdict line
13. `icelines fetch shifts` completes without errors on a season with cached boxscores, producing `ShiftProfile` records in the database
14. `icelines mates "Matty Beniers"` renders a linemate table from shift data (or a clear error directing to `icelines fetch shifts` if not available)
15. `icelines scouting "Leon Draisaitl"` renders all 8 sections with no placeholder text; linemate section populated when shift data is available
16. `cargo install --path icelines-cli` installs the `icelines` binary successfully from a clean checkout
17. The release workflow (`.github/workflows/release.yml`) produces binaries for all four targets (linux-musl, darwin-x86, darwin-arm64, windows-msvc) when a `v*.*.*` tag is pushed
18. No `unwrap()` in `icelines-core`, `icelines-fetch`, or `icelines-site` library code
19. Terminal rendering is correct at minimum size (80×24): no overflow, no truncation that makes the UI unusable
20. All unit and integration test assertions document expected values with calculation comments

---

## Test Coverage Requirements

See `docs/specs/test-strategy.md` for L0/L1/L2 definitions.
Phase 3 must not reduce L0 coverage below 95% on icelines-core.

### L0 — Unit Tests (Phase 3 additions)

| Test | Expected | Why |
|------|----------|-----|
| `pace_project(ppg=1.68, remaining=30)` | 50.4 pts | McDavid remaining-season |
| `alpha_at_10_gp` | 0.20 | min(10/50, 1.0) |
| `alpha_at_50_gp` | 1.00 | fully weighted to current |
| `alpha_at_75_gp` | 1.00 | clamped at 1.0 |
| `regressed_project(cur=1.68, career=1.45, gp=25)` | documented | α=0.5, weighted blend |
| `age_factor_at_26` | 1.00 | peak age |
| `age_factor_at_30` | ~0.96 | ~2% decline per year |
| `age_factor_at_35` | ~0.86 | cumulative decline |
| `age_factor_at_20` | ~0.92 | pre-peak discount |
| `confidence_band_width_increases_low_gp` | wider at 20 GP | small sample = more uncertainty |
| `shift_overlap_mcdavid_draisaitl` | high co-ice fraction | known linemates |
| `shift_profile_primary_linemates_top_3` | 3 partners | only top-3 returned |

**Property tests** (required):
```rust
proptest! {
    fn alpha_always_in_0_to_1(gp in 0u32..200) {
        let a = compute_alpha(gp);
        assert!(a >= 0.0 && a <= 1.0);
    }
    fn age_factor_always_positive(age in 18u8..45) {
        assert!(age_factor(age) > 0.0);
    }
    fn regressed_proj_between_pace_and_career(
        cur in 0.0f32..3.0, career in 0.0f32..3.0, gp in 10u32..82
    ) {
        let p = regressed_project(cur, career, gp, 30);
        let lo = cur.min(career) * 30.0;
        let hi = cur.max(career) * 30.0;
        assert!(p.total >= lo && p.total <= hi + 0.001);
    }
}
```

### L1 — Integration Tests (Phase 3 additions)

| Test | Verifies |
|------|---------|
| `projection_pipeline_pace_mode` | fixture player → documented projected pts |
| `projection_pipeline_regressed_mode` | career stats from fixture → blended result |
| `shift_profile_from_fixture_boxscores` | 5 boxscores → correct top-3 linemates |
| `shift_profile_min_shifts_filter` | < 50 shared shifts → excluded from partners |
| `tonight_schedule_parses_correctly` | fixture schedule JSON → game list |
| `tonight_projected_lines_from_last_boxscore` | last game lineup → projected lines |
| `trade_depth_chart_diff` | before/after chart → correct upgrade/downgrade detection |
| `tui_app_state_initial` | App::new() → Screen::Home, no panics |
| `tui_navigation_home_to_team` | Enter on team card → Screen::Team(team) |
| `tui_search_filters_live` | type "Beni" → Beniers appears, Eberle does not |

### L2 — System Tests (every Phase 3 command + TUI)

| Command / Scenario | Assertion |
|--------------------|-----------|
| `icelines project "Matty Beniers" --mode pace` | Exit 0, contains projected pts |
| `icelines project "Matty Beniers" --mode regressed` | Exit 0, different value than pace |
| `icelines project --team SEA` | Exit 0, shows all SEA skaters |
| `icelines tonight` | Exit 0, shows today's games |
| `icelines schedule --team SEA --days 7` | Exit 0, shows ≥ 1 SEA game |
| `icelines trade "Tolvanen" for "Brady Tkachuk"` | Exit 0, shows before/after depth |
| `icelines mates "Matty Beniers"` | Exit 0, shows linemates |
| `icelines scouting "Matty Beniers"` | Exit 0, 8 sections visible |
| `icelines fetch shifts --dry-run` | Exit 0, lists game IDs without fetching |
| TUI: `icelines tui` then `q` within 2s | Exit 0, terminal restored cleanly |
| TUI: `icelines` (no args) | Same as `icelines tui` |
| Binary: `icelines --version` | Matches `Cargo.toml` workspace version |
| Release: Linux binary runs on Ubuntu 22.04 | Exit 0, `--version` works |
| Release: macOS binary (arm64) runs | Exit 0, `--version` works |
| Release: Windows binary runs | Exit 0, `--version` works |

---

## Out of Scope

The following are explicitly out of scope for Phase 3 and for IceLines v0.1:

**Data sources not yet integrated:**
- Tier 4 advanced stats (Natural Stat Trick, MoneyPuck xGF, Evolving Hockey RAPM) — require
  third-party scraping and are not available via the NHL API
- Tier 5 social signals (Twitter/X, Reddit r/hockey sentiment) — no structured API
- Tier 6 beat media injury reports — unstructured text, no API

**Features deferred to v0.2+:**
- Mouse support in the TUI (crossterm mouse events not captured in v0.1)
- Live game score tracking — IceLines is a batch analytics tool, not a live dashboard
- Historical multi-season position tracking — position engine operates on current season only
- Windows installer / macOS `.app` bundle — distribution is `cargo install` and GitHub Releases
- Color theme customization — the four fit class colors are fixed in v0.1
- TUI split-pane / persistent sidebar layout
- `icelines dashboard preview` and `icelines dashboard new` CLI subcommands (stubs may be
  present from Phase 2 but are not fully implemented)
- Playoff projection modeling — rest-of-season engine covers regular season only
- Category-league fantasy scoring (PIM, PPP, hits, blocks as projection inputs) — points only
- Goalie scouting reports — goalies are parsed but not placed in lineup cards or projection tables
