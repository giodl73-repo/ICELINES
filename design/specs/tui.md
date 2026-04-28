# IceLines TUI — Specification

**Version**: 1.1 (v1 as-built)
**Date**: 2026-04-28
**Status**: Implemented — reflects current build

> v2 redesign (6-tab layout, Scores/Schedule/Playoffs) is in `tui-v2.md`.

---

## Overview

The IceLines TUI is a full-screen terminal interface launched by `icelines` with no
arguments. It shares the same data layer as the CLI commands and renders with `ratatui`.

```
icelines        # launch TUI
icelines tui    # explicit
```

---

## Current Tab Structure (v1)

Eight tabs in the nav bar. Keys `1`–`8` jump directly. `Tab` cycles forward.

```
 League │ /Search │ Queries │ Projections │ Tonight │ Groups │ Depth │ Fetch+Install
   1        2         3           4           5         6        7          8
```

| Tab | Screen | Purpose |
|-----|--------|---------|
| 1 | **League** | 32-team list ranked by pace; Enter → Team depth |
| 2 | **/Search** | Live fuzzy player search |
| 3 | **Queries** | Interactive filter/sort query builder |
| 4 | **Projections** | Pts/82 leaderboard, Enter → player card |
| 5 | **Tonight** | Placeholder — schedule stub (not wired to live data) |
| 6 | **Groups** | Saved player groups; Enter → group detail |
| 7 | **Depth** | Cross-team line value rankings; Enter → team depth chart |
| 8 | **Fetch+Install** | Season bundle install with progress bar |

---

## App State (`app.rs`)

```rust
pub struct App {
    pub screen:               Screen,
    pub prev_screen:          Option<Screen>,
    pub no_color:             bool,
    pub players:              Vec<Player>,
    pub load_state:           LoadState,
    pub install_state:        InstallState,
    pub tick:                 u64,
    pub selected:             usize,
    pub search_query:         String,
    pub status:               String,             // bottom status bar
    pub show_help:            bool,
    pub headshot_cache:       HeadshotCache,
    pub group_picker_open:    bool,
    pub group_picker_list:    Vec<String>,
    pub group_picker_player:  Option<(String, String)>,
    pub depth_mode:           ScoringMode,        // Fantasy | Pace
    pub query_fields:         Vec<QueryField>,
    pub query_field_idx:      usize,
    pub query_result_scroll:  usize,
    pub query_results_focused: bool,
    pub query_mode:           QueryMode,
    pub query_save_name:      String,
    pub query_saved_list:     Vec<(String, String)>,
}
```

`prev_screen` is a single-level back stack. Drill-down screens (Team, Player, Comps,
DepthTeam, GroupDetail) push onto it; Esc pops. Fallback parents are hardcoded in
`go_back()` for screens that may be reached without `prev_screen` being set.

---

## Screen Inventory

### League (`Screen::Home`)
- 32 teams in a scrollable list, ranked by top-player pace on the team
- `Enter` → `Screen::Team(abbrev)`
- `↑↓` wrap at top/bottom (circular)

### Team (`Screen::Team(String)`)
- Roster rows: Player | Pos | PPG | Pts/82
- `↑↓` select; `Enter` → `Screen::Player(idx)`
- `g` → group picker overlay; `f` → instant add to Favorites

### Player (`Screen::Player(usize)`)
- Left: ASCII headshot (fetched async, braille dither)
- Right: full stat card (PPG, proj/82, G/A/Pts, PP, TOI, SH%, +/-)
- `c` → `Screen::Comps(idx)`; `g`/`f` group/favorites; `Esc` back

### Comps (`Screen::Comps(usize)`)
- Target player stats on left
- Right: similar active players sorted by PPG-pace distance
- Same position category (F or D); green rows = within 0.020 PPG
- `Enter` → that player's card; `g`/`f` on selected comp

### Search (`Screen::Search`)
- `/` key from anywhere opens this; clears on open
- Live filter as you type; `Enter` → player card; `Esc` → back

### Queries (`Screen::Queries`)
- Left panel: 10 filter fields (Sort, Position, Age max/min, GP min,
  Nationality, Draft year/round, Seasons, Show top)
- Right panel: result rows with rank, player, team, pos, metric value
- `Space` toggles focus; `↓` past last field auto-focuses results;
  `↑` from first result returns to fields
- `s` = save query; `l` = load; `r` = reset; `Enter` = player card

### Projections (`Screen::Projections`)
- All players with pace_score, sorted pts/82 desc
- Columns: Rank | Player | Team | Pos | PPG | Pts/82 | GP
- `Enter` → player card; `g`/`f` on row

### Groups (`Screen::Groups`)
- Lists all groups from SQLite; `Enter` → `Screen::GroupDetail`

### GroupDetail (`Screen::GroupDetail(String)`)
- Members of one group; `Enter` → player card; `g`/`f` on member

### Depth — League (`Screen::Depth`)
- Teams ranked by depth score (top-4 LW + C + RW + top-3 LD + top-3 RD)
- `s` toggles scoring mode (Fantasy pts ↔ Pts/82)
- Bars, green top-8 / yellow mid-16 / red bottom-8
- `Enter` → `Screen::DepthTeam(abbrev)`

### Depth — Team (`Screen::DepthTeam(String)`)
- 5-column grid: LW | C | RW | LD | RD
- Each cell: player name + score + fit symbol (★ ~ ↑ ↓)
- Forwards: greedy position assignment; excess Centers spill to
  natural wing by shooting hand (lefty→LW, righty→RW)
- Defense: split by shoots_catches (L→LD, R→RD)
- `s` toggles scoring mode; `g`/`f` on any player

### Fetch (`Screen::Fetch`)
- Season list; `↑↓` select; `i` install; progress bar during install
- `Esc` leaves screen; install continues in background

### Tonight (`Screen::Tonight`)
- **Stub** — currently shows help text pointing to CLI commands
- Not wired to live NHL API data (see tui-v2.md)

### Help overlay
- `?` on any screen; any key dismisses

---

## Global Key Bindings

| Key | Action |
|-----|--------|
| `1`–`8` | Jump to tab |
| `Tab` | Cycle tabs (Home→Queries→Projections→Tonight→Groups→Depth→Fetch→Home) |
| `q` | Quit |
| `?` | Toggle help overlay |
| `/` | Jump to Search tab |
| `r` | Refresh / reset (screen-dependent) |
| `g` | Open group picker (any player-list screen) |
| `f` | Instant add to Favorites (any player-list screen) |
| `c` | Open comps (Player card only) |
| `s` | Toggle scoring mode (Depth/DepthTeam only) |
| `Space` | Toggle query panel focus (Queries screen only) |
| `Esc` | Back / close overlay |
| `↑↓` | Navigate rows; wrap at ends on League screen |
| `←→` | Cycle query field values (Queries screen) |
| `Enter` | Drill into selected item |

---

## Universal `g` / `f` Detection

Pressing `g` or `f` on any player-list screen opens the group picker or adds to
Favorites. The `get_selected_player(&self) -> Option<(String, String)>` method on
`App` detects which player is highlighted on the current screen:

| Screen | Player resolution |
|--------|------------------|
| `Player(idx)` | `players[idx]` |
| `Team(abbrev)` | `selected`th player on that team |
| `Projections` | `selected`th in pts/82-sorted list |
| `Search` | `selected`th in filtered results |
| `Queries` | row at `query_result_scroll + selected` |
| `GroupDetail` | `selected`th member resolved to Player |
| `Comps(idx)` | `selected`th comp for `players[idx]` |

---

## Cross-Team Line Value Algorithm

Used by both Depth screens. Implemented in `icelines-core/src/cross_team.rs`.

```
for each player P with position pos and score S:
  own_line  = rank of S among own team's pos group
  avg_line  = mean rank of S across all 32 teams' pos groups
  delta     = own_line - avg_line   (positive = buried)

  ★ Elite:   avg ≤ own + 0.5
  ~ Solid:   avg ≤ own + 1.25
  ↑ Buried:  delta > 0.75
  ↓ Stretch: avg > own + 1.25
```

Two scoring modes:
- **Pace**: `pace_score.sort_key()` (pts/82)
- **Fantasy**: `G×3 + A×2 + PPG×1 + PPA×0.5 + SHG×1 + SHA×0.5 + GWG×0.5 + HIT×0.5 + BLK×0.5`

---

## Headshot Rendering

Fetched async in `tui/headshot.rs`. Stored in `HeadshotCache` (Arc<DashMap>).
URL derived from `nhl_id + team + CURRENT_SEASON_STR` — no roster fetch needed.
Rendered as braille dither (4× effective resolution vs block characters).

---

## Known Gaps (v1 → v2)

- Tonight screen is a stub — not wired to live NHL data
- No live score polling
- No season time-travel
- No playoff bracket
- No schedule search
- Fetch+Install is a main nav tab (should be admin-only)
- Tab count (8) is at the limit of usability

See `tui-v2.md` for the redesign addressing these.
