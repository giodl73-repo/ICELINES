# IceLines TUI — Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft — pre-implementation

---

## 1. Overview

The IceLines TUI is a full-screen terminal interface that exposes the same analytics as the
CLI commands but in a navigable, keyboard-driven environment. It is the default when `icelines`
is invoked with no arguments.

The TUI is not a wrapper around CLI output — it reads from the same cache and data layer as
the commands, rendering directly to the terminal via `ratatui` widgets.

**Entry points:**
```
icelines          # no args — launches TUI (same as tui)
icelines tui      # explicit
```

**Exit:** Press `q` or `Esc` from the Home screen to quit.

---

## 2. Dependencies

| Crate       | Role                                              |
|-------------|---------------------------------------------------|
| `ratatui`   | Terminal rendering, layout, widgets               |
| `crossterm` | Cross-platform terminal raw mode, event reading   |

These crates are added to `icelines-cli/Cargo.toml` only. The TUI is a CLI concern; no TUI
code lives in library crates (`icelines-core`, `icelines-fetch`, `icelines-site`).

`crossterm` is used as the `ratatui` backend. This gives cross-platform support on Linux,
macOS, and Windows without requiring `termion` or platform-specific terminal libraries.

---

## 3. Crate Structure

```
icelines-cli/src/tui/
  mod.rs           — public entry point: run_tui() called from main.rs
  app.rs           — App state struct, navigation stack, event dispatch
  event.rs         — event loop: crossterm input → AppEvent enum
  screens/
    mod.rs
    home.rs        — Screen: Home (league tracker)
    team.rs        — Screen: Team lineup card
    player.rs      — Screen: Player profile
    search.rs      — Screen: Fuzzy search
    tonight.rs     — Screen: Tonight's games
    projections.rs — Screen: Rest-of-season projections
    groups.rs      — Screen: Watchlists / peer groups
    fetch.rs       — Screen: Cache status and fetch operations
  widgets/
    mod.rs
    player_cell.rs  — Reusable player cell (name, pace, fit class, color)
    team_card.rs    — Compact team summary widget (for Home screen grid)
    progress_bar.rs — Fetch progress widget with percentage and ETA
    help_overlay.rs — Modal overlay showing keyboard shortcuts
    sparkline.rs    — Inline career trend bar chart (Player screen)
```

---

## 4. App State

`App` in `app.rs` holds all mutable state for the TUI session:

```rust
pub struct App {
    /// Navigation stack: last element is the current screen.
    /// Popping returns to the previous screen.
    pub screen_stack: Vec<Screen>,

    /// Loaded data (from cache — no blocking I/O on the main thread).
    pub teams: Vec<TeamSummary>,
    pub players: Vec<Player>,
    pub position_profiles: HashMap<u32, PositionProfile>,
    pub tonight_games: Option<Vec<Game>>,

    /// Search state (shared across screens that support /).
    pub search_query: String,
    pub search_active: bool,
    pub search_results: Vec<SearchResult>,

    /// Scroll / selection state per screen (indexed by Screen variant).
    pub selected_row: HashMap<Screen, usize>,

    /// Whether the help overlay is shown.
    pub show_help: bool,

    /// Fetch progress (updated from background task via channel).
    pub fetch_progress: Option<FetchProgress>,

    /// Terminal color mode.
    pub color_mode: ColorMode,
}

pub enum ColorMode {
    Full,    // ANSI colors + fit class colors
    NoColor, // Monochrome; fit classes shown as text labels [ELITE], [SOLID], etc.
}
```

**Navigation stack invariant**: The stack is never empty. The bottom of the stack is always
`Screen::Home`. Pressing `Esc` or `q` from Home quits the TUI.

---

## 5. Event Loop

The event loop runs on the main thread. All cache reads happen on a `tokio` thread pool
and results are sent back via a `tokio::sync::mpsc` channel to avoid blocking redraws.

```
loop:
  1. Poll crossterm for terminal events with 16ms timeout (≈60fps)
  2. Drain the data channel (receive loaded data from background tasks)
  3. Update App state based on events and received data
  4. Call terminal.draw(|frame| render(frame, &app))
  5. If app.should_quit → break
```

Input events are normalized into an `AppEvent` enum before being dispatched to the current
screen's handler:

```rust
pub enum AppEvent {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Quit,
    Search,       // / key
    Refresh,      // r key
    Help,         // ? key
    Char(char),   // for search input
    Backspace,    // for search input
    Resize(u16, u16),
}
```

Each screen implements a `handle_event(event: AppEvent, app: &mut App)` function that
modifies `App` state. Screens do not draw themselves — drawing is done by the render
function, which reads from `App` state.

---

## 6. Navigation Model

| Key        | Action                                                        |
|------------|---------------------------------------------------------------|
| `↑` / `↓` | Navigate rows in a table or menu items in a list              |
| `←` / `→` | Navigate columns in a grid (Home screen), or tab panels       |
| `Enter`    | Select the highlighted item / drill down to detail screen     |
| `Esc`      | Go back one level (pop screen stack)                          |
| `q`        | Go back / quit from Home                                      |
| `/`        | Activate search mode; subsequent characters filter the view   |
| `r`        | Refresh current screen (re-read from cache; trigger re-fetch if stale) |
| `?`        | Toggle help overlay                                           |

**Search mode**: When `/` is pressed, a search input line appears at the bottom of the
screen. Characters typed are appended to `search_query`; the view filters in real time.
`Esc` cancels search and clears the query. `Enter` in search mode confirms the top result
and navigates to it.

**Help overlay**: A modal box appears over the current screen listing all active key
bindings for that screen. Press `?` again or `Esc` to dismiss.

---

## 7. Screens

### 7.1 Home — League Tracker

**Purpose**: Overview of all 32 NHL teams ranked by pace score, giving a quick read on which
teams have the most fantasy-relevant depth.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ IceLines — 2025-26  │  32 Teams  │  [r] Refresh  [/] Search  [?] Help   │
├────────────────────────────────────┬─────────────────────────────────────┤
│  Rank  Team        Pts/82  E  S  B │  Rank  Team        Pts/82  E  S  B  │
│   1    EDM         94.3   4  6  2  │  17    WPG         81.2   2  5  5   │
│   2    FLA         92.1   4  5  3  │  18    PIT         80.7   2  4  6   │
│   ...                              │  ...                                 │
│  16    SEA         81.8   2  6  4  │  32    SJS         67.3   0  2  10  │
└────────────────────────────────────┴─────────────────────────────────────┘
│ ↑↓ navigate  Enter: open team  /: search  r: refresh                     │
└──────────────────────────────────────────────────────────────────────────┘
```

**Columns**: Rank, Team abbreviation, team's mean pace pts/82 (all rostered skaters above
MIN_GP), E=Elite count, S=Solid count, B=Buried count.

**Color coding**: Each row's Team abbreviation cell is colored by the team's dominant fit class
(the fit class held by the most players on the team). Ties broken by higher class.

**Navigation**: `↑`/`↓` moves through the 32 teams. `Enter` opens the Team screen for
the highlighted team. Left/right arrows move between the two columns.

**Sorting**: Default: rank by team mean pace score descending. A future iteration may allow
toggling sort by Elite count, total pts/82, etc. — not in v0.1.

---

### 7.2 Team — Lineup Card

**Purpose**: The full 4×3 forward grid and 3×2 defense grid for a single NHL team, matching
the web site's team page layout.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Edmonton Oilers — 2025-26  [Esc: back]  [r: refresh]  [?: help]         │
├──────────────────┬──────────────────┬──────────────────┐
│ LW               │ C                │ RW               │
│ [Zach Hyman    ] │ [Leon Draisaitl] │ [Connor Brown  ] │
│  58.3 pts/82 GP49│ [GREEN] 1.29 PPG │  42.1 pts/82 GP51│
│ [GREEN]          │                  │ [SOLID]          │
├──────────────────┼──────────────────┼──────────────────┤
│ [Ryan Nugent-H ] │ [Connor McDavid] │ [Evander Kane  ] │
│  44.1 pts/82 GP52│  [GREEN] 1.34 PPG│  38.0 pts/82 GP41│
│ [SOLID]          │                  │ [SOLID]          │
├──────────────────┼──────────────────┼──────────────────┤
│  ...             │  ...             │  ...             │
├──────────────────┴──────────────────┘                  │
│ DEFENSE                                                 │
│ [Evan Bouchard ] │ [Darnell Nurse ]                    │
│  58.9 pts/82 GP53│  28.1 pts/82 GP53                   │
│ [GREEN]          │ [SOLID]                              │
│  ...                                                    │
└──────────────────────────────────────────────────────────────────────────┘
│ ↑↓←→ navigate  Enter: player detail  Esc: back                          │
└──────────────────────────────────────────────────────────────────────────┘
```

**Color contract** (see §10): Each player cell's background reflects their fit class.

**Navigation**: Arrow keys move the selection cursor across the grid. `Enter` on a player
cell pushes the Player screen for that player. `Esc` returns to Home.

**Below MIN_GP players**: Listed in a separate section at the bottom of the screen, below
the grid, labeled "Below MIN_GP — not ranked". Shown with name, team, position, and GP.

---

### 7.3 Player — Player Profile

**Purpose**: Detailed view for a single player: bio, pace stats, position profile, career trend.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Leon Draisaitl  #29  EDM  C/LW  Age 28  [Esc: back]                     │
├────────────────────────────────────┬─────────────────────────────────────┤
│ THIS SEASON                        │ CAREER TREND (pts/82)               │
│ GP:    52                          │  150 ┤                               │
│ G:     28  A:  39  Pts: 67        │  130 ┤          ████                 │
│ PPG:   1.288                       │  110 ┤     ████ ████ ████           │
│ Pace:  105.6 pts/82               │   90 ┤████ ████ ████ ████ ████       │
│ Fit:   [GREEN — ELITE]            │   70 ┤████ ████ ████ ████ ████ ████  │
│                                    │      └─────────────────────────────  │
│ PROJECTION (Regressed, 30G left)  │      18 19 20 21 22 23 24 25 26     │
│ Projected pts: 39  (range 31–46)  │                                      │
│ Proj total:    106 pts            │ PEERS (EDM forwards ≥ 40 GP)         │
│                                    │  1. Draisaitl   105.6 pts/82 [ELITE]│
│ POSITION PROFILE                  │  2. McDavid     110.1 pts/82 [ELITE]│
│ Primary:  C                        │  3. Hyman        58.3 pts/82 [ELITE]│
│ Eligible: C, LW                   │  4. Nugent-H     44.1 pts/82 [SOLID]│
│ Appearances: C=45, L=25           │  ...                                 │
└────────────────────────────────────┴─────────────────────────────────────┘
│ r: refresh  p: toggle projection mode  Esc: back                        │
└──────────────────────────────────────────────────────────────────────────┘
```

**Career trend chart**: A `ratatui` `BarChart` widget showing pts/82 pace for each prior
season. Bars are color-coded: green if ≥ Elite threshold, yellow if ≥ Solid, blue if ≥ Buried,
red if below. Short or COVID-affected seasons are displayed with a dimmed bar and a `*` label.

**Peers panel**: Shows all other players on the same team at the same position group (forwards
or defense) who are above MIN_GP, ranked by pace. The current player is highlighted.

**`p` key**: Cycles the Projection panel through pace → regressed → composite modes
without leaving the screen.

---

### 7.4 Search — Fuzzy Player Search

**Purpose**: Find any player quickly by name fragment. Active search across all players in
the cached data.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Search: drai█                                                            │
├──────────────────────────────────────────────────────────────────────────┤
│ #   Player                Team  Pos  GP   Pts  PPG    Fit               │
│  1  Leon Draisaitl        EDM   C/LW  52   67  1.288  [GREEN — ELITE]  │
│  2  Tyler Drainey         BUF   LW    12    8  0.667  [BURIED]          │
│                                                                          │
│ (2 results)                                                              │
└──────────────────────────────────────────────────────────────────────────┘
│ ↑↓ navigate results  Enter: open player  Esc: clear search              │
└──────────────────────────────────────────────────────────────────────────┘
```

**Matching**: Case-insensitive substring match on display name and `name_normalized`
(diacritic-stripped). Results are ranked by match quality: exact prefix match first, then
substring match, then normalized match.

**Performance**: Matching runs over the in-memory `players` vec on each keystroke. With
~800 NHL skaters in cache, this is fast enough to filter synchronously on the main thread.
No background thread needed for search.

**Activation**: The Search screen is pushed onto the stack when `/` is pressed from any
screen (not just Home). `Esc` pops back to the previous screen and clears the query.

---

### 7.5 Tonight — Today's NHL Games

**Purpose**: Show tonight's NHL schedule with projected starting lines for each game.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Tonight — Saturday, April 25, 2026  │  8 Games  [r: refresh]            │
├────────────────────────────────────────────────────────────────────────── │
│ 7:00 PM ET    AWAY: Carolina Hurricanes  vs  HOME: NY Rangers            │
│               Projected Lines (CAR)       Projected Lines (NYR)          │
│  F1           Niederreiter–Aho–Svechnikov  Panarin–Trocheck–Kreider      │
│  F2           Kotkaniemi–Drury–Teravainen  Kakko–Chytil–Lafreniere       │
│  F3           Necas–Staal–Jarvis          Vesey–Goodrow–Blais           │
│  F4           Lorentz–Kostalek–Noesen     Reaves–Gauthier–Carpenter     │
│  D1           Slavin–Burns                Miller–Fox                     │
│  D2           Chatfield–DeAngelo          Trouba–Lindgren                │
│  D3           Orlov–Pesce                 Jones–Schneider                │
│                                                                          │
│ 7:30 PM ET    AWAY: Toronto Maple Leafs  vs  HOME: Ottawa Senators       │
│  ...                                                                     │
└──────────────────────────────────────────────────────────────────────────┘
│ ↑↓ navigate games  Enter: open team  r: refresh  Esc: back              │
└──────────────────────────────────────────────────────────────────────────┘
```

**Data source**: Schedule from `api-web.nhle.com/v1/schedule/now`. Projected lines from
the most recent cached boxscore for each team — the last game played gives the most recent
lineup, which is the best available proxy for tonight's starting configuration.

**Caveat label**: Each projected lineup is labeled `"Projected (last game)"` to make clear
this is not confirmed lineup data.

**`Enter` on a game**: Pushes the Team screen for the highlighted team (Away or Home,
depending on which column the cursor is in).

---

### 7.6 Projections — Rest-of-Season Projections

**Purpose**: Tabular rest-of-season projections for a selected team or player group.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Projections — EDM — Regressed — 30 Games Remaining                      │
│ [t: change team]  [m: change mode]  [p: filter position]  [Esc: back]   │
├──────────────────────────────────────────────────────────────────────────┤
│  Rank  Player              Pos  GP   Pts  PPG    α     Proj  ±1σ        │
│    1   Connor McDavid      C    54  73  1.352  1.00   41   [34–48]      │
│    2   Leon Draisaitl      C/LW 52  67  1.288  1.00   39   [31–46]      │
│    3   Zach Hyman          LW   51  45  0.882  1.00   26   [20–33]      │
│    4   Evan Bouchard       D    53  56  1.057  1.00   32   [26–37]      │
│  ...                                                                     │
│                                                                          │
│ Team total:  241 projected remaining pts (±34 at ±1σ)                   │
└──────────────────────────────────────────────────────────────────────────┘
│ ↑↓ navigate  Enter: player detail  t: team  m: mode  p: pos filter      │
└──────────────────────────────────────────────────────────────────────────┘
```

**`t` key**: Opens an inline team picker (list of 32 team abbreviations, searchable).
Selecting a team reloads the projection table for that team.

**`m` key**: Cycles through projection modes (pace → regressed → composite). Table
recomputes immediately from cached data (no re-fetch needed).

**`p` key**: Opens an inline position filter: All | Forwards | Defense | C | LW | RW | D.

---

### 7.7 Groups — Watchlists and Peer Groups

**Purpose**: Browse and manage saved player groups (watchlists, custom peer groups).

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Groups                              [n: new group]  [Esc: back]          │
├──────────────────────────────────────────────────────────────────────────┤
│  Group                       Players  Created                            │
│  My Fantasy Team (Yahoo)     15       2026-01-10                         │
│  Pacific Division Centers     8       2026-02-14                         │
│  Top-30 Under 25              28      2026-03-01                         │
│  Trade Target Watch            5       2026-04-18                        │
│                                                                          │
│ (4 groups)                                                               │
└──────────────────────────────────────────────────────────────────────────┘
│ ↑↓ navigate  Enter: open group  d: delete  n: new  Esc: back            │
└──────────────────────────────────────────────────────────────────────────┘
```

**Opening a group** (`Enter`): Pushes a list view of all players in the group, formatted
identically to the rank screen but filtered to group members. From there, `Enter` on a
player pushes the Player screen.

**Group storage**: Groups are stored in `~/.icelines/groups.toml` as TOML arrays of player IDs.
The groups file is not part of the cache (it is user data and is never invalidated).

**`n` key**: Opens an inline text field to name a new group. After naming, a search interface
allows adding players by name. New groups are saved immediately.

---

### 7.8 Fetch — Cache Status and Fetch Operations

**Purpose**: Show the age and completeness of the local cache, and allow the user to trigger
fetch operations with live progress bars.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Fetch Status — Season 2025-26                        [Esc: back]         │
├──────────────────────────────────────────────────────────────────────────┤
│  Data Set          Status       Last Fetched              Size           │
│  Rosters           OK           2026-04-24 09:14          1.2 MB         │
│  Stats (bios)      OK           2026-04-24 09:21          3.4 MB         │
│  Stats (summary)   OK           2026-04-24 09:21          2.8 MB         │
│  Positions         STALE        2026-04-20 18:00          8.1 MB         │
│  Boxscores         891 games    2026-04-24 22:10          94 MB          │
│  Player landings   712 players  2026-04-23 12:00          22 MB          │
│  Schedule          OK           2026-04-25 08:00          0.1 MB         │
│                                                                          │
│  [r] Fetch all (stale)   [R] Fetch all (force)   [p] Fetch positions    │
│  [s] Fetch stats         [b] Fetch boxscores     [l] Fetch landings     │
│                                                                          │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
│ r: refresh stale  R: force-refresh all  Esc: back                       │
└──────────────────────────────────────────────────────────────────────────┘
```

**Progress display**: When a fetch operation is running, a progress bar appears in the
bottom section:

```
Fetching boxscores: [████████████░░░░░░░░░░░░░░░░░░] 44% (396/891 games)  ETA: 2m14s
```

Fetch operations run on a tokio background task; progress updates are sent to the TUI
via the data channel. The TUI remains responsive (navigable) during a fetch — the fetch
screen shows progress but the user can press `Esc` to leave and the fetch continues.

**Stale definition**: A dataset is considered stale if its cache file is older than the
dataset's TTL (rosters: 24h, stats: 24h, positions: recomputed on demand, schedule: 6h).

---

## 8. Widget Catalog

### `PlayerCell`

A fixed-size (20-char name + stats line + fit class badge) widget used in Team screen grids.
Accepts a `&Player` and `ColorMode`. In `Full` mode, the cell's background is set to the
fit class color. In `NoColor` mode, a text label `[ELITE]`, `[SOLID]`, `[BURIED]`, or
`[STRETCH]` is appended.

```rust
pub struct PlayerCell<'a> {
    pub player: &'a Player,
    pub selected: bool,
    pub color_mode: ColorMode,
}
```

### `TeamCard`

A compact 4-line widget for the Home screen grid. Shows team abbreviation, mean pace score,
and E/S/B/X counts. Background reflects the team's dominant fit class.

```rust
pub struct TeamCard<'a> {
    pub team: &'a TeamSummary,
    pub rank: usize,
    pub selected: bool,
    pub color_mode: ColorMode,
}
```

### `ProgressBar`

A reusable progress bar widget used on the Fetch screen and during fetch operations anywhere.
Shows label, filled/empty segments, percentage, and optional ETA string.

```rust
pub struct ProgressBar<'a> {
    pub label: &'a str,
    pub total: u64,
    pub completed: u64,
    pub eta_secs: Option<u64>,
}
```

### `HelpOverlay`

A modal widget that renders a bordered box over the current screen with a list of key
bindings. Bindings are screen-specific and passed as a slice.

```rust
pub struct HelpOverlay<'a> {
    pub bindings: &'a [(&'a str, &'a str)], // (key, description)
}
```

### `Sparkline`

A bar chart widget used on the Player screen to show per-season pts/82. Uses `ratatui`'s
built-in `BarChart` widget with per-bar color derived from fit class thresholds.
Short seasons are rendered at reduced opacity using the `dim` modifier.

---

## 9. Color Contract

The same four fit class colors used by the web site are used in the TUI. This ensures
users who use both interfaces develop a single visual vocabulary.

| Fit class | Terminal color (ANSI)      | Hex (web reference) | Text label (no-color) |
|-----------|----------------------------|---------------------|-----------------------|
| Elite     | Green background (BG 2)    | #2e7d32             | `[ELITE]`             |
| Solid     | Yellow background (BG 3)   | #f9a825             | `[SOLID]`             |
| Buried    | Blue background (BG 4)     | #1565c0             | `[BURIED]`            |
| Stretch   | Red background (BG 1)      | #b71c1c             | `[STRETCH]`           |

Colors are applied to cell backgrounds, not foreground text. Foreground text is always
white (on color backgrounds) or default terminal color (in no-color mode).

`owo-colors` is used for ANSI color application in the `comfy-table` terminal renderer
used by non-TUI commands. In the TUI, `ratatui`'s `Style` and `Color` APIs are used
directly — `owo-colors` is not a dependency of the TUI module.

---

## 10. Accessibility — `--no-color` Flag

```
icelines tui --no-color
icelines --no-color        (applies globally including TUI)
```

When `--no-color` is set:
- `App.color_mode` is set to `ColorMode::NoColor` at startup
- No ANSI background colors are applied to any widget
- Fit class is communicated via text labels: `[ELITE]`, `[SOLID]`, `[BURIED]`, `[STRETCH]`
- These labels are placed in the position where the color would otherwise be the signal
- All other layout, navigation, and functionality is identical

The `--no-color` flag can also be set via the `NO_COLOR` environment variable (the
[no-color.org](https://no-color.org) convention). If `NO_COLOR` is set to any non-empty
value, color is disabled without requiring the flag.

---

## 11. Terminal Size Requirements

**Minimum**: 80 columns × 24 rows. Below this, the TUI renders a single-line message:
`"Terminal too small — resize to at least 80×24"` and stops drawing.

**Recommended**: 120 columns × 40 rows for the Team screen's full grid layout without
truncation. The Home screen's 2-column grid is readable at 80 columns.

**Resize handling**: `crossterm` emits resize events. The TUI handles `AppEvent::Resize(w, h)`
by re-checking the minimum, updating layout constraints, and forcing a full redraw.

---

## 12. Non-Goals

- **Mouse support.** The TUI is keyboard-only in v0.1. `crossterm` mouse events are not
  captured or handled.
- **In-TUI data editing.** Groups can be created and deleted, but player stats and cache
  files are read-only from within the TUI.
- **Real-time score updates.** The TUI does not poll for live game scores. The Tonight
  screen shows schedule data and projected lines, not live scores.
- **Split-pane layout.** Each screen occupies the full terminal. There is no persistent
  sidebar or split view in v0.1.
- **Color theme customization.** The four fit class colors are fixed and cannot be
  remapped by the user.
- **Screen recording or export.** There is no TUI-native export to PNG or HTML.
  Use the CLI commands and pipe output to files for export needs.
