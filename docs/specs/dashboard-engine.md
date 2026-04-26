# IceLines Dashboard Engine — Specification

**Version**: 0.1  
**Date**: 2026-04-25  
**Status**: Draft

The current site hardcodes one dashboard type: 32 team lineup cards.
The dashboard engine makes every view a first-class, declaratively defined artifact.
Any combination of query + template + layout produces a new dashboard — no Rust code required.

---

## Core Idea

A dashboard is three things:

```
Query      →  which players/teams/data to pull, with what filters
Template   →  how to render one record (a player cell, a team row, a chart bar)
Layout     →  how to arrange rendered records (grid, table, column, tabs)
```

These are defined in `.toml` files in a `dashboards/` directory.
`icelines build` reads all dashboard definitions and generates a site section for each.

---

## Dashboard Definition Format

Each dashboard is a `.toml` file in `dashboards/`:

```toml
# dashboards/draft-class-2022.toml

[dashboard]
id       = "draft-class-2022"
title    = "2022 Draft Class"
subtitle = "How the 2022 draft class is performing in year 3"
output   = "docs/dashboards/draft-class-2022/"
template = "class-card"         # references templates/class-card.html
layout   = "two-column-grid"    # references layouts/two-column-grid.html
nav      = true                 # include in site nav
```

```toml
[query]
source    = "players"           # players | teams | groups | shifts | history
# --- filters ---
draft_year  = 2022
gp_min      = 20
# position   = ["C", "LW", "RW"]   # optional
# team        = "SEA"               # optional
# age_max     = 24                  # optional
# --- sorting ---
sort_by   = "ppg_pace"          # ppg_pace | goals_pace | age | draft_pick | name
sort_dir  = "desc"
limit     = 64
```

```toml
[display]
# Fields available in the template as {{ player.X }}
fields = [
  "name", "team", "age", "nationality_flag",
  "draft_pick", "gp", "ppg_pace", "goals_pace",
  "assists_pace", "fit_class", "avg_other_line",
  "photo_url", "team_logo_url",
]
# Metric shown as the primary badge on each card
primary_metric = "ppg_pace"
primary_label  = "pts/82"
# Secondary badge
secondary_metric = "goals_pace"
secondary_label  = "g/82"
```

---

## Built-In Query Sources

| Source | Description | Key Filters |
|--------|-------------|-------------|
| `players` | All skaters/goalies in current dataset | pos, team, age, nationality, draft_year, gp_min, ppg_min, toi_min |
| `teams` | All 32 NHL teams | division, conference |
| `groups` | Members of a saved `icelines group` | group_name |
| `history` | Multi-season data for a player | player, seasons |
| `shifts` | Shift-derived linemate data | player, min_shared_shifts |
| `depth_chart` | A team's computed depth chart | team, pos |
| `leaderboard` | League-wide ranking at a position | pos, metric, limit |
| `buried` | Players with delta > threshold | pos, min_fpts, min_delta |
| `class` | Full draft class | draft_year, pos, round |
| `peers` | A player's peer group | player, method |

---

## Built-In Templates

Templates live in `templates/` and receive a single record (player, team, etc.).
New templates are Tera `.html` files — no Rust code needed.

### `player-lineup-cell`
The current team depth chart cell: photo, name, gp·ppg·proj, fit color.

### `player-card`
Larger card: photo, full bio line, pace stats, draft info, flag, team logo.

### `player-row`
Single table row: rank, name, team, pos, age, gp, ppg, g/82, fit badge.

### `class-card`
Draft pick card: pick number badge, name, team, pace stats, class rank.

### `team-summary-card`
Team tracker card: logo, rank badge, position bars (C/LW/RW/D), total score.

### `scouting-report`
Full single-player page: bio, career history table, peer rank, linemates, fit.

### `leaderboard-row`
Simple ranked row: position, name, team, stat value, trend arrow.

### `group-member-row`
Group member row: rank within group, name, pace stats, group-relative percentile.

---

## Built-In Layouts

Layouts arrange a collection of rendered templates.

### `two-column-grid`
32 items in 2 columns of 16 — the current tracker index.

### `four-column-grid`
Cards in 4 columns — good for draft class (30+ cards).

### `ranked-table`
Sortable HTML table — for leaderboards, filtered player lists.

### `single-page`
One record filling the page — for scouting reports, player profiles.

### `tabbed`
Multiple queries in tabs on one page (e.g., C / LW / RW / D tabs).

### `comparison`
Two queries side by side — for group compare, head-to-head.

---

## Example Dashboard Definitions

### Leaderboard: U23 Centers
```toml
# dashboards/u23-centers.toml
[dashboard]
id       = "u23-centers"
title    = "U23 Centers"
subtitle = "Under-23 centers ranked by PPG pace"
output   = "docs/dashboards/u23-centers/"
template = "player-row"
layout   = "ranked-table"

[query]
source    = "players"
pos       = ["C"]
age_max   = 23
gp_min    = 15
sort_by   = "ppg_pace"
sort_dir  = "desc"

[display]
fields         = ["rank", "name", "team", "age", "nationality_flag", "gp", "ppg_pace", "goals_pace", "draft_pick"]
primary_metric = "ppg_pace"
```

### Draft Class Dashboard
```toml
# dashboards/class-2022-all.toml
[dashboard]
id       = "class-2022-all"
title    = "2022 Draft Class"
output   = "docs/dashboards/class-2022/"
template = "class-card"
layout   = "four-column-grid"

[query]
source     = "class"
draft_year = 2022
gp_min     = 10
sort_by    = "draft_pick"
sort_dir   = "asc"

[display]
fields         = ["draft_pick", "name", "team", "pos", "age", "gp", "ppg_pace", "goals_pace", "fit_class"]
primary_metric = "ppg_pace"
primary_label  = "pts/82"
```

### Buried Trade Assets
```toml
# dashboards/buried-assets.toml
[dashboard]
id       = "buried-assets"
title    = "Buried Trade Assets"
subtitle = "Players playing significantly below their cross-team value"
output   = "docs/dashboards/buried-assets/"
template = "player-row"
layout   = "ranked-table"

[query]
source      = "buried"
min_fpts    = 80
min_delta   = 0.75
sort_by     = "delta"
sort_dir    = "desc"
limit       = 40

[display]
fields = ["rank", "name", "team", "pos", "ppg_pace", "own_line", "avg_other_line", "delta", "photo_url"]
```

### Peer Group Watchlist
```toml
# dashboards/sea-rebuild-targets.toml
[dashboard]
id       = "sea-rebuild-targets"
title    = "SEA Rebuild Targets"
subtitle = "Custom watchlist — potential trade acquisitions for Seattle"
output   = "docs/dashboards/sea-rebuild-targets/"
template = "player-card"
layout   = "two-column-grid"

[query]
source     = "groups"
group_name = "SEA Rebuild Targets"
sort_by    = "ppg_pace"
sort_dir   = "desc"

[display]
fields = ["name", "team", "pos", "age", "gp", "ppg_pace", "avg_other_line", "fit_class", "photo_url", "team_logo_url"]
```

### Tabbed Position Leaderboard
```toml
# dashboards/position-leaders.toml
[dashboard]
id     = "position-leaders"
title  = "League Leaders by Position"
output = "docs/dashboards/position-leaders/"
layout = "tabbed"

[[dashboard.tabs]]
label    = "Centers"
template = "leaderboard-row"
query    = { source = "leaderboard", pos = "C", sort_by = "ppg_pace", limit = 30 }
fields   = ["rank", "name", "team", "gp", "ppg_pace", "goals_pace"]

[[dashboard.tabs]]
label    = "Left Wing"
template = "leaderboard-row"
query    = { source = "leaderboard", pos = "LW", sort_by = "ppg_pace", limit = 30 }
fields   = ["rank", "name", "team", "gp", "ppg_pace", "goals_pace"]

[[dashboard.tabs]]
label    = "Right Wing"
template = "leaderboard-row"
query    = { source = "leaderboard", pos = "RW", sort_by = "ppg_pace", limit = 30 }
fields   = ["rank", "name", "team", "gp", "ppg_pace", "goals_pace"]

[[dashboard.tabs]]
label    = "Defense"
template = "leaderboard-row"
query    = { source = "leaderboard", pos = "D", sort_by = "ppg_pace", limit = 30 }
fields   = ["rank", "name", "team", "gp", "ppg_pace", "goals_pace"]
```

---

## CLI Integration

```bash
# Build all dashboards defined in dashboards/
icelines build

# Build only specific dashboards
icelines build --dashboard u23-centers
icelines build --dashboard class-2022-all

# Create a new dashboard from a template
icelines dashboard new --name "Scandinavian Wings" \
  --query "pos=LW,RW nationality=SWE,FIN,NOR" \
  --template player-row \
  --layout ranked-table

# List all defined dashboards
icelines dashboard list

# Preview a dashboard query result without generating HTML
icelines dashboard preview u23-centers --limit 5
```

---

## Rust Architecture

```
icelines-core/
  src/
    query/
      mod.rs           # QuerySpec struct (deserialized from TOML)
      executor.rs      # run_query(spec, &PlayerStore) -> Vec<Record>
      filters.rs       # PlayerFilter, TeamFilter, GroupFilter, etc.
    template/
      mod.rs           # TemplateEngine (Tera wrapper)
      registry.rs      # register built-in templates
    layout/
      mod.rs           # LayoutEngine — arrange N rendered templates
    dashboard/
      mod.rs           # DashboardSpec struct
      builder.rs       # load TOML → run query → render → write files
      nav.rs           # collect all dashboards → generate nav entries

icelines-cli/
  src/
    commands/
      build.rs         # walks dashboards/, calls DashboardBuilder for each
      dashboard.rs     # new, list, preview subcommands
```

**Key types:**

```rust
pub struct DashboardSpec {
    pub dashboard: DashboardMeta,
    pub query:     QuerySpec,
    pub display:   DisplaySpec,
    pub tabs:      Option<Vec<TabSpec>>,  // for tabbed layout
}

pub struct QuerySpec {
    pub source:   QuerySource,   // Players | Teams | Groups | ...
    pub filters:  PlayerFilter,
    pub sort_by:  SortField,
    pub sort_dir: SortDir,
    pub limit:    Option<usize>,
}

pub enum QuerySource {
    // Tier 1+2: available in Release 1
    Players,
    Teams,
    Groups    { name: String },
    Class     { draft_year: u16 },
    Leaderboard { pos: Position },
    Buried    { min_delta: f32 },
    Peers     { player: String, method: PeerMethod },
    DepthChart { team: String, pos: Option<Position> },
    History   { player: String, seasons: u8 },          // multi-season career data

    // Tier 3: available in Release 2 (requires shift data)
    Shifts    { player: String, min_shared_shifts: u32 }, // linemate analysis
}

pub struct Record {
    // Flat key-value map fed into Tera template
    pub fields: HashMap<String, TemplateValue>,
}
```

---

## Adding a New Dashboard (no Rust required)

1. Create `dashboards/my-dashboard.toml`
2. Define `[dashboard]`, `[query]`, `[display]` sections
3. Optionally create `templates/my-template.html` (Tera syntax)
4. Run `icelines build --dashboard my-dashboard`
5. Preview at `http://127.0.0.1:8000/dashboards/my-dashboard/`

New dashboard types that need new query sources or display fields
require a small Rust change in `icelines-core/src/query/executor.rs`.
Everything else is configuration.

---

## Non-Goals

- **Ad-hoc SQL**: Queries are structured filter specs, not raw SQL. Power users can export
  to JSON/CSV and use their own tools.
- **Real-time dashboards**: Pages are generated at build time, not streamed live.
- **Custom JavaScript charts**: The engine outputs static HTML/CSS. Plotly or D3 integration
  is a separate enhancement, not a core requirement.
- **Multi-sport support**: IceLines is NHL-specific. The query model uses hockey concepts
  (lines, pairs, PPG, TOI) that do not generalize.
