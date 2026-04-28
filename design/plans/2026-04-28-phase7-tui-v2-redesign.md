# Phase 7 — TUI v2 Redesign

**Status**: Draft
**Date**: 2026-04-28
**Specs**: `design/specs/tui-v2.md`, `scores.md`, `schedule.md`, `playoffs.md`, `season-timetravel.md`

---

## Goal

Restructure the TUI from 8 flat tabs to 6 purposeful tabs, wire the Tonight stub to live NHL
data, add season time-travel, and build the three new screens: Scores, Schedule, Playoffs.

---

## Blocking Order

```
Phase 7a (Nav restructure)
  └── Phase 7b (Season picker + bundle loading)
        ├── Phase 7c (Scores tab — live API)
        ├── Phase 7d (Schedule tab)
        └── Phase 7e (Playoffs tab)
```

Phase 7b must complete before 7c/7d/7e because all three new screens use `active_season`
from the time-travel system.

---

## Phase 7a — Nav Restructure

**No new screens. Pure reorganization.**

Tasks:
- Merge Queries + Projections into Stats tab (sub-views with `←→`)
- Move Depth into League tab as sub-view 1 (default); League list as sub-view 2
- Move Fetch+Install to `F` admin overlay + `:` command prompt
- Update tab numbers 1–6, key bindings, nav bar hints
- Update `cycle_screen()` and `GoToTab` handler

Acceptance criteria:
- All 6 tabs reachable by number key and Tab cycle
- Fetch+Install no longer in nav bar
- All existing screens function identically within new structure
- Tests: `l0_tui_tab_cycles_through_6_screens`, `l0_admin_overlay_opens_on_F`

---

## Phase 7b — Season Picker + Bundle Loading

**Adds global `active_season` and the season picker overlay.**

Tasks:
- Add `active_season: Season` to `App` struct
- Implement `y` key → season picker overlay
- Season picker reads `~/.icelines/seasons/manifest.json` for installed seasons
- Selecting a season updates `active_season` and triggers data reload
- Nav bar shows `[YYYY-YY]` indicator when non-current season active
- Lockout 2004-05 is rendered as unselectable with `✗` symbol; Enter on it is rejected
- `i` key in picker triggers background install with inline progress indicator

Single-threaded clarification: The TUI event loop runs on a single tokio thread. All
`App` state mutations (including `active_season` changes) happen in the event handler,
not in background tasks. Background tasks (install, data load) communicate results via
`mpsc` channel drained in the main loop. No `Arc<Mutex>` needed for `active_season`.

Acceptance criteria:
- Season picker opens on `y` from any screen
- Selecting 2021-22 loads that season's bios/stats from bundle
- Uninstalled seasons are dimmed; selecting one prompts to install
- 2004-05 shows `✗ LOCKOUT` and cannot be selected (rejected in handler, not just dim)
- Nav bar shows `[2021-22]` when that season is active
- Tests: `l0_season_picker_rejects_lockout`, `l0_season_picker_updates_active_season`

---

## Phase 7c — Scores Tab

**Replaces Tonight stub with live NHL scores and date navigation.**

Tasks:
- Implement `Screen::Scores` with `TonightCache` (Arc<Mutex<TonightState>>)
- `maybe_fetch()` triggered when Scores tab becomes active
- Auto-poll `/v1/score/now` every 30s while tab is active; pause when inactive
- Date navigation: `←→` changes `scores_date`, triggers new fetch
- Playoff detection: `game.game_type == 3` → show series status
- Implement `Screen::GameDetail(game_id)` for Enter → boxscore
- WIRE: cache key = `(date_str, game_type)`. Stale after 35s for live games;
  permanent for completed games. Network failure → show stale data with `[stale]` tag.

Cache policy:
- Today's games: 30s TTL while active, stale-while-revalidating
- Past dates: permanent (games don't change after final)
- `r` key clears cache and forces immediate refresh

Error states:
- Network timeout → show last-known data with `[Last updated Xm ago]`
- API returns malformed game state → skip that game, show others
- Date with no games → show `No games scheduled for this date`

Acceptance criteria:
- Today's playoff games show series status (Game N, series score)
- Past dates show final scores from cache
- Network failure shows stale data, not crash
- Auto-refresh stops when navigating away from tab
- Tests: `l0_scores_shows_playoff_series_status`, `l0_scores_stale_on_network_failure`

---

## Phase 7d — Schedule Tab

**New screen: full-season schedule with team/matchup search.**

Tasks:
- Implement `Screen::Schedule` with weekly view default
- `/` search: parse single-team `SEA` or two-team `SEA WSH` filter
- Team filter → full team schedule view
- Matchup filter → head-to-head game log (regular season + playoff)
- `←→` week navigation; `t` jumps to current week
- WIRE: cache per week (`schedule_{YYYY-MM-DD_weekstart}.json`, 6h TTL for future weeks, permanent for past)

Input validation:
- Team codes validated against canonical 32-team list; unknown codes show `Unknown team: XYZ`
- Two-team search: if same team twice (`SEA SEA`), show error `Cannot search same team vs itself`
- Case-insensitive: `nyr` treated same as `NYR`

Partial fetch degradation:
- If week fetch fails, show `Schedule unavailable for this week [retry: r]`
- Do not show blank/empty rows for missing weeks; show explicit failure message
- Other weeks (already cached) remain accessible

Acceptance criteria:
- `/SEA` filters to all Kraken games
- `/NYR WSH` shows full regular season + playoff head-to-head
- Invalid team code shows error, does not crash
- Network failure for one week does not affect other cached weeks
- Tests: `l0_schedule_search_single_team`, `l0_schedule_search_matchup`, `l0_schedule_invalid_team_error`

---

## Phase 7e — Playoffs Tab

**New screen: bracket, series detail, historical Cup campaigns.**

Tasks:
- Implement `Screen::Playoffs` with bracket view (simplified list-style for v1)
- Series box: seeding, teams, win-loss record
- `Enter` on series → `Screen::SeriesDetail(series_id)`
- Series detail: game log + leading scorers (aggregated from bundled game data)
- `y` opens season picker for historical bracket navigation
- Off-season state: show projected playoff picture from current standings

Bracket design decision (resolves Open Q#1 from playoffs.md):
- **v1: list-style bracket** — simpler, works at 80 columns, less fragile than ASCII art
- **v2: ASCII bracket** — deferred to after v1 ships and bracket data is validated

Historical data structure: each season bundle includes `playoffs.json` per the schema
in `playoffs.md`. If a season's bundle lacks `playoffs.json`, the Playoffs tab shows
`Historical playoff data not available for this season` with an install prompt.

Series leaders computed from game-by-game results in `playoffs.json`. Each game entry
includes goal scorers (name only, not detailed stats). Assists are not included in v1
series leaders — only goals. Assists added in v2 when per-game boxscore data is bundled.

Error states:
- Series detail with no game data → show series score only, no game log
- Missing bracket data → show message, not crash
- In-progress series with Game N not yet in schedule → show `Game N (if needed)` only after
  Game N-1 is completed and series not yet decided

Acceptance criteria:
- Current playoff bracket shows all first-round series
- Series detail shows game log for completed series
- Historical 1993-94 bracket loads from bundle and shows NYR as champion
- Off-season shows projected standings-based playoff picture
- Tests: `l0_playoffs_shows_series_detail`, `l0_playoffs_historical_bracket_loads`

---

## Test Strategy

Each phase must deliver:
- L0 unit tests for new App state transitions
- L1 integration tests for new data fetch paths (against mock NHL API)
- L2 tests for end-to-end screen rendering (against fixture data)

No phase ships without all three tiers green.

---

## Timeline

Phase 7a can begin immediately — no new data sources required.
Phases 7b–7e require phase 7a complete and are parallelizable after 7b.
