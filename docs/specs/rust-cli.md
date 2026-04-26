# IceLines Rust CLI — Full Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft — pre-implementation

---

## 1. Problem Statement

IceLines began as a collection of Python scripts stitched together with shell glue: one script
to read the Yahoo CSV, one to hit the NHL API, one to render Jinja2 templates, one to invoke
mkdocs. Each script works in isolation. The pipeline is not.

Problems with the current approach:

- **No shared data model.** Each script reinvents player record parsing, producing subtle
  inconsistencies in field naming, position normalization, and GP handling.
- **No error surface.** A player whose name fails to match the NHL API is silently dropped.
  There is no structured error type, no recovery, and no user-facing message.
- **No caching.** Every run hits the NHL API for all players. During a 500-player CSV run,
  this means hundreds of HTTP requests, rate-limit exposure, and no resumability.
- **No CLI contract.** Users run scripts directly with hardcoded paths. There is no `--help`,
  no argument validation, no subcommand structure.
- **No testability.** The scoring logic is embedded in script globals, untestable in isolation.

The Rust CLI solves all of these with a workspace of focused crates, a typed data model, a
cache-first fetch layer, and a clap-based CLI with clear subcommands. The site generation
output (mkdocs) remains the same format, but the generator is now a Rust function called from
`icelines build`, not a Python script.

**Why Rust?**

- The scoring engine (pace projection, fit classification, depth chart construction) is a
  pure computation with no I/O. Rust makes invalid states unrepresentable and eliminates
  a category of runtime errors.
- The fetch layer (NHL API client) benefits from async/await and reqwest, with the
  cache layer implemented as simple file I/O — no external dependencies.
- The site generator does not need a Python runtime or venv management.
- The CLI is a single binary that can be distributed without interpreter installation.

---

## 1a. Replacing Python Scripts

The table below documents which Python script each `icelines` command replaces. This
serves as the migration checklist: when a command is implemented and tested, the
corresponding Python script can be deleted.

| Python script           | Rust CLI command             | Notes                                      |
|-------------------------|------------------------------|--------------------------------------------|
| `scripts/fetch_rosters.py` | `icelines fetch rosters`  | One request per team, caches to JSON       |
| `scripts/fetch_gp.py`   | `icelines fetch stats`       | Paginated bios + summary, merges on ID     |
| `scripts/gen_site.py`   | `icelines build`             | Tera templates replace Jinja2              |
| `scripts/deploy.bat`    | `icelines deploy`            | Calls `mkdocs gh-deploy` via subprocess    |
| *(new — no prior script)* | `icelines fetch positions` | Aggregates boxscores for position profiles |
| *(new — no prior script)* | `icelines tonight`         | Schedule + projected lines from boxscores  |
| *(new — no prior script)* | `icelines tui`             | Full-screen terminal UI                    |

**Yahoo CSV dependency**: The Yahoo Fantasy Hockey CSV is no longer used for any position
data. Positions come exclusively from the NHL API — `positionCode` from the bios endpoint
for primary position, and the boxscore `position` field per game for actual deployment
tracking. See `docs/specs/position-engine.md` for the full algorithm.

---

## 2. Commands Specification

### `icelines fetch`

Fetch all NHL data needed to run IceLines. Replaces three Python scripts
(`fetch_rosters.py`, `fetch_gp.py`) with a single composable command.
All subcommands use `https://api-web.nhle.com/v1/` or
`https://api.nhle.com/stats/rest/en/` (new NHL API — not the deprecated `/api/v1/`).

```
icelines fetch <SUBCOMMAND> [OPTIONS]

Subcommands:
  rosters    Fetch all 32 team rosters + headshot URLs  (replaces fetch_rosters.py)
  stats      Fetch season stats for all skaters          (replaces fetch_gp.py)
  all        Run rosters then stats in sequence           (default if no subcommand)

Common Options:
  --season <YEAR>    Season in YYYYZZZZ format [default: current, e.g. 20252026]
  --refresh          Invalidate cache and re-fetch everything
  --dry-run          Show what would be fetched without API calls
  -v, --verbose      Show per-player status
```

#### `icelines fetch rosters`

```
GET https://api-web.nhle.com/v1/roster/{TEAM}/{SEASON}
```

- Fetches all 32 NHL team rosters (one request per team)
- Stores player bio + headshot URL keyed by normalized name
- Headshot URL format: `https://assets.nhle.com/mugs/nhl/{SEASON}/{TEAM}/{player_id}.png`
- Writes to `~/.icelines/cache/rosters/{SEASON}/{TEAM}.json`

#### `icelines fetch stats`

Two bulk-paginated endpoints, no per-player requests:

```
# Bios: GP, player ID, team, position, birth data
GET https://api.nhle.com/stats/rest/en/skater/bios
    ?cayenneExp=seasonId={SEASON}%20and%20gameTypeId=2&limit=100&start={N}

# Stats: G, A, points, TOI, PPG, shots
GET https://api.nhle.com/stats/rest/en/skater/summary
    ?cayenneExp=seasonId={SEASON}%20and%20gameTypeId=2&limit=100&start={N}
```

- Paginates until `start + page_size >= response.total`
- Merges bios + stats on `playerId`
- Writes to `~/.icelines/cache/stats/{SEASON}/bios.json` and `stats.json`

**All stats (G, A, GP, TOI) come from the NHL API — not from any CSV.**

**Behavior:**
- Checks local cache before each API call
- Writes fetched GP data to cache with timestamp
- Reports players that could not be resolved (name mismatch, not found in API)
- Exits 0 if all players resolved, exits 1 if any player could not be resolved (unless `--allow-missing`)

---

### `icelines build`

Generate the mkdocs site from cached data and CSV.

```
icelines build [OPTIONS]

Options:
  --csv <PATH>       Path to Yahoo Fantasy Hockey CSV [default: data/fantasy.csv]
  --out <DIR>        Output directory for generated markdown [default: docs/teams/]
  --min-gp <N>       Minimum GP threshold for pace ranking inclusion [default: 10]
  --no-site          Generate markdown only, do not run mkdocs
  --season <YEAR>    NHL season year for site title [default: current]
```

**Behavior:**
- Reads CSV, loads cached GP data, computes pace projections and fit classifications
- Generates one markdown file per NHL team (32 files) in `--out`
- Generates index page at `docs/index.md` with tier summary per team
- Optionally invokes `mkdocs build` via subprocess (requires mkdocs in PATH)
- Fails if any player in the CSV has no cached GP data (run `icelines fetch` first)

---

### `icelines serve`

Build and serve the site locally for review.

```
icelines serve [OPTIONS]

Options:
  --csv <PATH>       Path to Yahoo Fantasy Hockey CSV [default: data/fantasy.csv]
  --port <PORT>      Port for mkdocs serve [default: 8000]
  --min-gp <N>       Minimum GP threshold [default: 10]
```

**Behavior:**
- Runs `icelines build` then invokes `mkdocs serve --dev-addr 127.0.0.1:<PORT>`
- Requires mkdocs and mkdocs-material in PATH

---

### `icelines deploy`

Build and deploy the site to GitHub Pages.

```
icelines deploy [OPTIONS]

Options:
  --csv <PATH>       Path to Yahoo Fantasy Hockey CSV [default: data/fantasy.csv]
  --remote <NAME>    Git remote to deploy to [default: origin]
  --min-gp <N>       Minimum GP threshold [default: 10]
```

**Behavior:**
- Runs `icelines build` then invokes `mkdocs gh-deploy --remote-name <remote>`
- Requires mkdocs and mkdocs-material in PATH, git remote configured

---

### `icelines team <TEAM>`

Display a lineup card for a single NHL team in the terminal.

```
icelines team <TEAM> [OPTIONS]

Arguments:
  <TEAM>             NHL team abbreviation (e.g. EDM, COL, SEA) or full name (partial match)

Options:
  --csv <PATH>       Path to Yahoo Fantasy Hockey CSV [default: data/fantasy.csv]
  --min-gp <N>       Minimum GP threshold [default: 10]
  --no-color         Disable ANSI color output
```

**Behavior:**
- Renders the 4×3 forward grid and 3×2 defense grid for the specified team
- Color-codes each player cell by fit classification
- Shows pace projection (1 decimal), GP, and position in each cell
- Reports players on the team who are below MIN_GP (listed separately, not on the card)

**Example output:**
```
Colorado Avalanche — 2023-24 — IceLines Lineup Card

FORWARDS          LW              C               RW
Line 1      [Lehkonen   ] [MacKinnon  ] [Rantanen   ]
            83.4 pts/82  109.3 pts/82  96.7 pts/82
            [GREEN]       [GREEN]       [GREEN]

Line 2      [Rodrigues  ] [Tomas      ] [O'Connor   ]
            ...

DEFENSE           LD              RD
Pair 1      [Makar      ] [Toews      ]
            ...
```

---

### `icelines rank`

Display a pace-adjusted ranking table for all players in the CSV.

```
icelines rank [OPTIONS]

Options:
  --csv <PATH>       Path to Yahoo Fantasy Hockey CSV [default: data/fantasy.csv]
  --min-gp <N>       Minimum GP threshold [default: 10]
  --position <POS>   Filter by position: F, D, C, LW, RW [default: all]
  --team <ABBR>      Filter by team abbreviation
  --top <N>          Show top N players [default: 50]
  --no-color         Disable ANSI color output
```

**Behavior:**
- Ranks all players by pace projection descending, goals/GP as tiebreaker
- Displays rank, name, team, position, GP, pace projection, goals/82, fit class
- Color-codes each row by fit classification
- Players below MIN_GP are listed at the bottom with a "< MIN_GP" marker, not ranked

---

### `icelines compare <T1> <T2>`

Side-by-side comparison of two NHL teams' lineup cards.

```
icelines compare <T1> <T2> [OPTIONS]

Arguments:
  <T1>               First team abbreviation
  <T2>               Second team abbreviation

Options:
  --csv <PATH>       Path to Yahoo Fantasy Hockey CSV [default: data/fantasy.csv]
  --min-gp <N>       Minimum GP threshold [default: 10]
  --no-color         Disable ANSI color output
```

**Behavior:**
- Renders two lineup cards side-by-side in the terminal (requires ≥140 column terminal)
- Highlights slots where T1 player's pace projection exceeds T2's by >10 pts/82 (or vice versa)

---

### `icelines tonight`

Show today's NHL games with projected starting lines for each game.

```
icelines tonight [OPTIONS]

Options:
  --team <ABBREV>    Filter output to games involving this team only
  --json             Output JSON instead of formatted text
```

**Behavior:**

Fetches today's schedule from `https://api-web.nhle.com/v1/schedule/now`. For each game,
retrieves the most recent cached boxscore for each team to derive the projected starting
lines — the last game played is the best available proxy for tonight's lineup.

Output is a compact side-by-side view for each game:

```
7:00 PM ET  Carolina Hurricanes @ NY Rangers

  CAR Projected Lines         NYR Projected Lines
  F1  Niederreiter–Aho–Svech  F1  Panarin–Trocheck–Kreider
  F2  Kotkaniemi–Drury–Tera   F2  Kakko–Chytil–Lafreniere
  F3  Necas–Staal–Jarvis      F3  Vesey–Goodrow–Blais
  F4  Lorentz–Kostalek–Noes   F4  Reaves–Gauthier–Carpenter
  D1  Slavin–Burns             D1  Miller–Fox
  D2  Chatfield–DeAngelo       D2  Trouba–Lindgren
  D3  Orlov–Pesce              D3  Jones–Schneider

  Note: Lines projected from last game boxscore — not confirmed.
```

Lines are constructed by reading the forward and defense arrays from each team's most
recent boxscore and grouping by line number / pair number where available. If a team
has no cached boxscore (has not played a game yet this season), their projected lines are
shown as `"(no boxscore data)"`.

`--team`: If provided, only games involving `<ABBREV>` are shown. Other games are omitted.

**JSON output (`--json`)**: An array of game objects, each with `game_time`, `away_team`,
`home_team`, `away_projected_lines`, `home_projected_lines`, and `data_source` (the game
ID of the most recent boxscore used for each team).

---

### `icelines schedule`

Show the upcoming NHL schedule for a team or the full league.

```
icelines schedule [OPTIONS]

Options:
  --team <ABBREV>    Show schedule for this team only
  --days <N>         Number of days ahead to show [default: 7]
  --json             Output JSON instead of formatted table
```

**Behavior:**

Fetches the schedule by iterating `https://api-web.nhle.com/v1/schedule/{DATE}` for each
date from today through today + `--days`. Filters to the specified team if `--team` is
provided.

Output lists each game with date, time, home, and away team:

```
NHL Schedule — Next 7 Days

Sat Apr 25    7:00 PM ET    CAR @ NYR
Sat Apr 25    7:30 PM ET    TOR @ OTT
Sat Apr 25   10:00 PM ET    SEA @ VGK
Sun Apr 26    3:00 PM ET    EDM @ CGY
...
```

If `--team` is provided, only that team's games are shown. Games the team has already
played (today's earlier games) are included if they appear in the schedule feed.

---

### `icelines trade`

Analyze the depth chart impact of a trade for the team losing the outgoing player.

```
icelines trade <PLAYER_OUT> for <PLAYER_IN> [OPTIONS]

Arguments:
  <PLAYER_OUT>       Player being traded away (name or NHL player ID)
  <PLAYER_IN>        Player being acquired (name or NHL player ID)

Options:
  --team <ABBREV>    Team perspective [default: player_out's current team]
  --json             Output JSON instead of formatted tables
```

**Behavior:**

Shows the depth chart for `--team` (or `player_out`'s current team) in two states:
before the trade (current) and after the trade (simulated). The diff highlights:

- Which line slot `player_out` occupied
- Where `player_in` is placed given their primary and eligible positions
- Whether the replacement is an upgrade or downgrade (pace comparison)
- How team totals (sum of pace_82 for all rostered skaters) change

Example output:

```
Trade Analysis — EDM perspective
  OUT: Jesse Puljujarvi (RW, 23.4 pts/82, Line 3 RW)
  IN:  Rickard Rakell   (RW/LW, 51.2 pts/82)

  BEFORE           →    AFTER
  Line 3 RW                 Line 3 RW
  [Puljujarvi] 23.4         [Rakell] 51.2  +27.8 pts/82

  Team total before: 312.4 pts/82
  Team total after:  340.2 pts/82  (+27.8)

  Verdict: Upgrade — Rakell slots directly into Puljujarvi's line 3 RW slot.
```

**Scope**: This command evaluates only the team losing `player_out` (i.e., the team specified
by `--team`). The other team's depth chart change is not shown. To evaluate the other side,
run `icelines trade <PLAYER_IN> for <PLAYER_OUT> --team <OTHER_TEAM>` separately.

**Positioning**: `player_in` is placed in the depth chart using the same algorithm as the
depth chart builder (primary position first, then eligible positions). If `player_in` is not
in IceLines' cached data (e.g., they are a minor-league player not in the NHL API stats
feed), the command exits with an error and a suggestion to run `icelines fetch stats`.

---

### `icelines project`

Rest-of-season projection for a player or all skaters on a team. Full specification in
`docs/specs/projection-engine.md`.

```
icelines project <PLAYER|--team TEAM> [OPTIONS]

Arguments:
  <PLAYER>           Player name (partial match) or NHL player ID
                     Mutually exclusive with --team

Options:
  --team <ABBREV>    Project all skaters on this team (mutually exclusive with <PLAYER>)
  --mode <MODE>      pace | regressed | composite  [default: regressed]
  --games <N>        Remaining games to project [default: computed from schedule API]
  --season           Show by-season career comparison (single-player mode only)
  --pos <POSITIONS>  Comma-separated position filter for --team: C,LW,RW,D  [default: all]
  --json             Output JSON instead of formatted table
```

**Single-player output**: Player bio, current pace, career PPG, α weight, projected
remaining points, ±1σ confidence band, and projected full-season total. With `--season`,
also shows a by-season career comparison table.

**Team output**: Ranked table of all skaters on the team above MIN_GP, sorted by projected
remaining points descending. Includes team total projected remaining and ±1σ band.

See `docs/specs/projection-engine.md` for the full algorithm, formula definitions, and
data source specification.

---

### `icelines tui`

Launch the IceLines full-screen terminal user interface. Full specification in
`docs/specs/tui.md`.

```
icelines tui [OPTIONS]
icelines         (no arguments — same as icelines tui)

Options:
  --no-color     Disable ANSI colors; use text labels for fit classes instead
```

**Behavior**: Enters terminal raw mode, initializes the `ratatui` + `crossterm` event loop,
and renders the Home screen (league tracker). All navigation is keyboard-driven.

The TUI reads exclusively from the local cache — it does not make NHL API calls during
normal operation. Use `icelines fetch` (or the Fetch screen inside the TUI, key `f` from
Home) to refresh stale data.

Press `q` or `Esc` from the Home screen to exit the TUI and return to the shell.

See `docs/specs/tui.md` for the full screen inventory, widget catalog, navigation model,
and color contract.

---

## 3. Crate Architecture

The IceLines workspace is structured as four crates with a strict dependency DAG:

```
icelines-core
    ↑
icelines-fetch       icelines-site
    ↑                    ↑
         icelines-cli
```

### `icelines-core`

**Role**: Pure domain logic. No I/O, no async, no network.

**Owns**:
- All data model types (`Player`, `Team`, `DepthChart`, `LineAssignment`, `FitClass`, `PaceScore`)
- Scoring engine: pace projection, fit classification, tiebreaker sorting
- Depth chart builder: assigns players to line positions given a team's roster
- Position resolver: given a Yahoo position string, returns primary NHL position
- Name normalizer: Unicode NFC normalization, diacritic stripping fallback

**Does NOT own**:
- File I/O, network I/O, environment variables
- CLI argument parsing
- Template rendering
- Error types that wrap `std::io::Error` (those live in fetch/site)

**Error type**: `icelines_core::Error` (thiserror enum covering scoring failures, invalid player
state, position parse errors)

---

### `icelines-fetch`

**Role**: All async I/O. NHL API client, local cache, CSV loader.

**Owns**:
- NHL API HTTP client (reqwest, async) — base URL: `https://api.nhle.com/stats/rest/en/`
- Bulk bio fetch: `fetch_all_bios(season) -> Vec<PlayerBio>` — paginates `/skater/bios` using `limit=100&start={N}` until `start + page_size >= total`
- Bulk stats fetch: `fetch_all_stats(season) -> Vec<SkaterStats>` — paginates `/skater/summary` using `limit=100&start={N}` until `start + page_size >= total`
- Local cache layer (`~/.icelines/cache/`, JSON files with TTL stamps)
- CSV parser (reads Yahoo Fantasy CSV into `Vec<Player>`)
- Player ID resolution (name → NHL API player ID)
- Error types for network, cache, schema validation failures

**Depends on**: `icelines-core` (for `Player`, `Team`, position types)

**Does NOT own**:
- Scoring logic
- Template rendering
- CLI argument parsing

**Error type**: `icelines_fetch::Error` (thiserror enum covering HTTP errors, cache errors,
CSV parse errors, player ID resolution failures, schema validation failures)

---

### `icelines-site`

**Role**: Site and markdown generation. Replaces `scripts/gen_site.py`.

**Owns**:
- Lineup card markdown renderer (Tera templates)
- Index page renderer
- Team page generator (one markdown file per team)
- mkdocs.yml generator/updater
- Dashboard TOML loader and page generator (see `docs/specs/dashboard-engine.md`)

**Does NOT own**:
- Terminal rendering — that belongs in `icelines-cli`
- Business logic, scoring, fit classification
- Network I/O
- File path resolution outside the site output directory

**Depends on**: `icelines-core` (for all data types and scoring results)

**Error type**: `icelines_site::Error` (template rendering errors, file write errors)

---

### `icelines-cli`

**Role**: Binary entry point. Wires everything together. The single binary
replaces all Python scripts: `fetch_rosters.py`, `fetch_gp.py`, `gen_site.py`,
`deploy.bat`. Users run `icelines <subcommand>`, not Python scripts.

**Owns**:
- `main.rs` with clap derive subcommand enum
- Command handler functions (thin wrappers that call fetch/core/site)
- tokio runtime setup
- Top-level error formatting (convert crate errors to user-facing messages)
- Config loading (`~/.icelines/config.toml` or project-local `.icelines.toml`)
- **Terminal renderer**: `src/render/terminal.rs` — colored rows via `owo-colors`,
  tabular layout via `comfy-table`. Terminal rendering is CLI concern, not site concern.

**Depends on**: All three library crates.

**Error type**: `anyhow::Error` (binary — error context chaining via anyhow is appropriate here)

---

## 4. Data Model

All types live in `icelines-core`. All types implement `Debug`, `Clone`, `serde::Serialize`,
`serde::Deserialize` unless noted.

```rust
/// A player as read from the Yahoo Fantasy CSV and enriched with NHL API data.
pub struct Player {
    pub name: String,              // Display name (normalized, not stripped)
    pub name_normalized: String,   // Diacritic-stripped, lowercase, for matching
    pub nhl_id: Option<u32>,       // NHL player IDs fit in u32 (range ~6000–9000000)
    pub team: TeamAbbr,            // Current NHL team abbreviation (3-letter canonical)
    pub position: Position,        // Primary position (from Yahoo column, normalized)
    pub yahoo_positions: Vec<Position>, // All Yahoo-eligible positions
    pub season_goals: u32,         // Goals from NHL API SkaterStats.goals
    pub season_assists: u32,       // Assists from NHL API SkaterStats.assists
    pub season_points: u32,        // Goals + assists from NHL API SkaterStats
    pub season_gp: Option<u32>,    // GP from NHL API PlayerBio.gamesPlayed (None if not yet fetched)
    pub pace_score: Option<PaceScore>, // Computed by scoring engine (None if GP < MIN_GP)
    pub fit_class: Option<FitClass>,   // Computed by scoring engine
}

/// NHL team abbreviation — a newtype over String, validated against 32-team canonical list.
pub struct TeamAbbr(String);

/// Player position on ice.
pub enum Position {
    Center,
    LeftWing,
    RightWing,
    Defense,
    Goalie,
}

/// Season identifier — always 8-digit YYYYZZZZ format (e.g. 20252026).
/// Newtype prevents silent confusion with 4-digit year u32 values.
pub struct Season(pub u32);

impl Season {
    pub fn current() -> Self { /* derive from current date */ todo!() }
    pub fn as_str(&self) -> String { self.0.to_string() }
}

/// A complete depth chart for one NHL team.
pub struct DepthChart {
    pub team: TeamAbbr,
    pub season: Season,       // 8-digit YYYYZZZZ, e.g. Season(20252026)
    pub forward_lines: [[Option<Player>; 3]; 4],  // [line][LW=0, C=1, RW=2]
    pub defense_pairs: [[Option<Player>; 2]; 3],  // [pair][LD=0, RD=1]
    pub unplaced: Vec<Player>, // Players on team who didn't fit the standard structure
    pub below_min_gp: Vec<Player>, // Players below MIN_GP threshold
}

/// A player's assigned position within a depth chart.
pub struct LineAssignment {
    pub line: u8,        // 1-4 for forwards, 1-3 for defense
    pub slot: Slot,
}

pub enum Slot {
    LeftWing,
    Center,
    RightWing,
    LeftDefense,
    RightDefense,
}

/// Fit classification for a player's roster slot.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum FitClass {
    Elite,   // Green  — pace projection exceeds Elite threshold for position group
    Solid,   // Yellow — pace projection between Solid and Elite threshold
    Buried,  // Blue   — pace projection below Solid threshold (underused for slot)
    Stretch, // Red    — pace projection inconsistent with assigned slot (overextended)
}

/// Pace-adjusted scoring result for a player.
pub struct PaceScore {
    pub ppg: f64,              // Points per game (season_points / season_gp)
    pub pace_82: f64,          // PPG × 82 (projected full-season points)
    pub goals_per_game: f64,   // Tiebreaker value
    pub gp: u32,               // Games played (from NHL API)
}
```

---

## 5. Scoring Algorithm

### 5.1 Pace Projection Formula

```
pace_82 = (season_points / season_gp) × 82
```

Where:
- `season_points` = goals + assists from NHL API SkaterStats (fields: `goals`, `assists`)
- `season_gp` = NHL API current-season GP from PlayerBio (not Yahoo's cached GP column)
- 82 = nominal NHL regular season length

**Tiebreaker**: When two players have identical `pace_82` to two decimal places, rank by
`goals_per_game` descending. Secondary tiebreaker: alphabetical by last name ascending.

**MIN_GP**: Players with `season_gp < 10` are excluded from pace ranking and fit classification.
They appear on the lineup card only if they are on the team roster in the CSV, marked with a
"<MIN_GP" badge instead of a pace projection.

### 5.2 Fit Classification Thresholds

Thresholds are calibrated to the approximate percentile distribution of NHL player PPG × 82 by
position group in a typical season. All values below are for forwards; defense thresholds are
lower (forwards score at higher rates than defensemen).

| Class   | Color  | Forward threshold       | Defense threshold       |
|---------|--------|------------------------|------------------------|
| Elite   | Green  | pace_82 ≥ 65 pts/82    | pace_82 ≥ 45 pts/82    |
| Solid   | Yellow | pace_82 ≥ 40 pts/82    | pace_82 ≥ 28 pts/82    |
| Buried  | Blue   | pace_82 ≥ 20 pts/82    | pace_82 ≥ 14 pts/82    |
| Stretch | Red    | pace_82 < 20 pts/82    | pace_82 < 14 pts/82    |

**Rationale for 65/40/20 (forwards)**:
- 65 pts/82 represents approximately the 80th percentile of forward production — genuine top-six
  talent with consistent offensive contribution.
- 40 pts/82 represents approximately the 50th percentile — a reliable middle-six player.
- 20 pts/82 represents approximately the 20th percentile — a player who produces at a bottom-six
  or fourth-line rate.
- Below 20 pts/82 projected, a player is either in a depth role or underperforming their slot.

**Rationale for 45/28/14 (defense)**:
- Scaled from forward thresholds by the approximate ratio of mean forward PPG to mean defense PPG
  in the modern NHL (~0.69). 65 × 0.69 ≈ 45, 40 × 0.69 ≈ 28, 20 × 0.69 ≈ 14.

These thresholds are **provisional**. They should be reviewed against actual season data
annually and updated in the assumptions log with the observed percentile at each boundary.

---

## 6. Site Generation Approach

The site generator in `icelines-site` uses Tera templates to produce mkdocs-compatible markdown.
Templates live in `icelines-site/templates/`.

**Key templates:**
- `team.md.tera` — one team lineup card page. Receives a `DepthChart` and renders the 4×3
  forward grid and 3×2 defense grid with mkdocs-material card grid layout and CSS class names
  corresponding to fit classes.
- `index.md.tera` — index page with all 32 teams, sorted by Elite player count descending.
  Each team entry shows tier distribution (N Elite, N Solid, N Buried, N Stretch).

**CSS fit classes** (defined in `docs/assets/icelines.css`):
- `.fit-elite` — background: #e8f5e9, border-left: 4px solid #2e7d32
- `.fit-solid` — background: #fffde7, border-left: 4px solid #f9a825
- `.fit-buried` — background: #e3f2fd, border-left: 4px solid #1565c0
- `.fit-stretch` — background: #ffebee, border-left: 4px solid #b71c1c

These classes are applied to mkdocs-material `div.grid.cards` items. The card displays:
- Player name (bold, 20-char truncation)
- Team abbreviation + position
- Pace projection: `XX.X pts/82`
- GP badge: `(XX GP)`

---

## 7. Error Handling Strategy

IceLines uses a layered error handling approach consistent with Rust idioms:

**Library crates** (`icelines-core`, `icelines-fetch`, `icelines-site`):
- Each defines its own error enum using `thiserror`
- All public functions return `Result<T, CrateError>`
- No `unwrap()` in library code — every potential panic site uses `?` or explicit `expect()` with
  a documented invariant
- No `Box<dyn Error>` in public return positions

**CLI binary** (`icelines-cli`):
- Uses `anyhow` for error context chaining
- Translates crate errors into user-facing messages with context:
  - "Could not resolve player 'Slafkovsky' to NHL API ID — did you mean 'Juraj Slafkovský' (ID 8482078)?"
  - "NHL API returned HTTP 503 for player Connor McDavid — the API may be down. Try again later."
  - "Cache entry for player Tolvanen (ID 8481601) is 3 days old and --refresh was not passed."
- Exits with non-zero exit code on any unrecoverable error
- All player-level resolution failures are collected and reported together, not one at a time

**NHL API schema changes**:
- `serde(deny_unknown_fields)` on all API response types
- Schema validation failure produces a specific, actionable error message (see WIRE role)

---

## 8. Non-Goals

The following are explicitly out of scope for the IceLines Rust CLI:

- **Real-time game tracking.** IceLines is a batch analytics tool, not a live dashboard.
  GP data is fetched on demand and cached; it does not auto-refresh.
- **Goalie analysis.** The lineup card covers skaters only (forwards and defensemen).
  Goalies in the Yahoo CSV are parsed but not placed on lineup cards.
- **Trade value / dynasty ranking.** IceLines ranks current-season pace, not future value.
  Age curves, contract status, and draft capital are out of scope.
- **Points league vs. category league scoring.** The pace projection is points-based.
  Fantasy leagues using goals/assists/PIM/PPP categories are not modeled.
- **Team-strength adjustment.** Pace projections are raw PPG × 82, unadjusted for opponent
  quality, zone starts, or line-mate quality. This is a named assumption (A1), not an oversight.
- **Historical season comparison.** The CLI operates on one season at a time. Multi-season
  trend analysis is not implemented.
- **User account management or auth.** There is no login, no cloud sync, no user profile.
  All data is local files.
- **Windows installer or macOS .app bundle.** Distribution is via `cargo install` or
  pre-built binaries on the GitHub releases page.
