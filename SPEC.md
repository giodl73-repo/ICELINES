# IceLines TUI — v2 Specification

> **Status**: Design draft — review before implementation  
> **Date**: 2026-04-28  
> **Scope**: Navigation redesign, new screens, season time-travel, live data

---

## Vision

IceLines is a terminal-native NHL analytics and live-game tracker for people who want
depth beyond the league app. Two audiences share the same tool:

- **Fantasy players** — depth charts, projections, queries, group management
- **Hockey fans** — live scores, playoff tracker, historical season deep-dives

Both audiences benefit from **season time-travel**: navigate any of the last 38 seasons
and see that year's depth charts, stats, playoff results, and Stanley Cup campaign.

---

## Navigation v2

Six main tabs replace the current eight. `Fetch+Install` moves out of the nav bar.

```
[ League ]  [ Stats ]  [ Scores ]  [ Schedule ]  [ Groups ]  [ Playoffs ]
   1            2          3            4             5           6
```

### Tab summary

| # | Tab | Purpose |
|---|-----|---------|
| 1 | **League** | Team depth chart rankings, drill into any team's LW/C/RW/LD/RD chart |
| 2 | **Stats** | Player search, query builder, projections, comps — all analytics |
| 3 | **Scores** | Tonight's games, live scores, date navigation, game detail |
| 4 | **Schedule** | Full season schedule, searchable by team or matchup |
| 5 | **Groups** | Favorites and custom fantasy groups |
| 6 | **Playoffs** | Bracket, series tracker, historical Stanley Cup campaigns |

### Removed from nav bar

- `/Search` — becomes a `/` overlay on any screen (already works this way)
- `Projections` — merged into **Stats** tab
- `Fetch+Install` — moved to admin command (see §Admin)

### Key bindings (global)

| Key | Action |
|-----|--------|
| `1`–`6` | Jump to tab |
| `Tab` | Cycle tabs forward |
| `/` | Open search overlay |
| `q` | Quit |
| `?` | Help overlay |
| `Esc` | Back / close overlay |
| `s` | Toggle scoring mode (Fantasy pts ↔ Pts/82) on Stats/League/Depth screens |
| `y` | Open season picker (time-travel) on any screen |
| `r` | Refresh live data (Scores screen) |
| `:` | Admin command prompt (fetch, install) |

---

## Season Time-Travel

A core feature across all tabs. At any point, press `y` to open a season picker overlay:

```
┌─── Select Season ─────────────────────┐
│  ▶ 2025-26  (current)                  │
│    2024-25                              │
│    2023-24                              │
│    2022-23                              │
│    2021-22                              │
│    ────────────────────                 │
│    2003-04  (last Cup before lockout)   │
│    ...                                  │
│    1987-88  (Gretzky era)               │
│                                         │
│  ↑↓ select · Enter confirm · Esc cancel │
└─────────────────────────────────────────┘
```

- **Installed seasons** shown in full color; uninstalled shown dim with `[not installed]`
- Selecting a season switches ALL tabs to that season's data
- A season indicator shows permanently in the nav bar: `[2003-04]`
- Current season (live) always available; historical requires `icelines data install`
- **Playoff tab** uses the selected season's bracket — great for revisiting historic runs

### Season indicator in nav

```
 League │ Stats │ Scores │ Schedule │ Groups │ Playoffs │  [2003-04]  Tab:cycle  q:quit
```

---

## Screen Specs

### 1. League

**Purpose**: Team depth chart rankings and per-team drill-down.

**Default view** — ranked table:
```
  Rk  Team    LW      C      RW      LD      RD    Total   ████████████████
   1  EDM    312    298     276     184     167    1237    ████████████████
   2  TBL    287    265     241     176     158    1127    ████████████░░░░
  ...
```

- Sorted by total (top-4 LW + top-4 C + top-4 RW + top-3 LD + top-3 RD)
- Scoring mode toggle: `s` switches between Fantasy pts and Pts/82
- Green top-8 / yellow middle-16 / red bottom-8
- `Enter` → team depth chart

**Team depth chart view**:
- 5 columns: LW | C | RW | LD | RD
- Each player: name, score, fit label (★ ~ ↑ ↓)
- Fit label uses cross-team avg line rank (existing algorithm)
- `g`/`f` on selected player → group/favorites
- `Enter` on player → player card

**Fit labels**:
| Symbol | Color | Meaning |
|--------|-------|---------|
| ★ | Green | Elite — true caliber for this line |
| ~ | Yellow | Solid — slightly above their level |
| ↑ | Cyan | Buried — would play higher on most other teams |
| ↓ | Red | Stretch — overextended on this line |

---

### 2. Stats

**Purpose**: All analytics in one tab — search, queries, projections, comps.

**Sub-views** (toggle with `Tab` within the screen or `←→`):

```
[ Projections ]  [ Query Builder ]  [ Comps ]
```

**Projections sub-view**:
- Sorted pts/82 leaderboard, same as current Projections screen
- Columns: Rank | Player | Team | Pos | PPG | Pts/82 | GP
- `Enter` → player card

**Query Builder sub-view**:
- Same as current Queries screen (field editor left, results right)
- Space toggles focus between panels

**Comps sub-view** (new at tab level):
- Type a player name → shows their similar players
- Currently `c` from player card; also accessible here directly

**Player card** (drill-down from any sub-view):
- Headshot + full stats
- `c` → comps screen
- `h` → history (career arc)
- `g`/`f` → group/favorites

---

### 3. Scores

**Purpose**: Live scores for tonight, date navigation, game detail.

**Default view** — today's games:
```
  TONIGHT — Tuesday Apr 28, 2026                    [PLAYOFFS R1]

  ◉ LIVE  7:05 PM   NYR  2 – 3  WSH   · Game 5 · WSH leads 3-1
  ◉ LIVE  7:35 PM   EDM  1 – 0  CGY   · Game 5 · Tied 2-2
  ○ 10:00 PM        VGK        LAK    · Game 4 · VGK leads 2-1

  ← yesterday                              tomorrow →
```

- `←`/`→` navigate dates
- `r` refreshes live scores (polls NHL API)
- Playoff games show series status prominently
- Regular season games show team records
- `Enter` on a game → game detail (boxscore summary)

**Game detail view**:
```
  NYR 2 – WSH 3  ·  Final  ·  Game 5  ·  WSH leads series 3-1

  GOALS
  1st  14:22  Ovechkin (WSH) — Kuznetsov, Carlson
  1st  18:44  Zibanejad (NYR) — Trocheck
  2nd   5:11  Panarin (NYR) — Trocheck, Fox
  ...

  SERIES
  Game 1  WSH 4-2 NYR
  Game 2  WSH 3-1 NYR
  Game 3  NYR 5-2 WSH
  Game 4  WSH 3-2 NYR (OT)
  Game 5  WSH 3-2 NYR  ← tonight
```

**Data sources**:
- `/v1/score/now` — live scores + goals
- `/v1/schedule/now` — tonight's games
- `/v1/gamecenter/{gameId}/boxscore` — game detail

---

### 4. Schedule

**Purpose**: Full season schedule, searchable, filterable by team or matchup.

**Default view** — all games this week:
```
  SCHEDULE  ·  /search matchup  ·  ←→ week  ·  t: team filter

  Mon Apr 28   NYR @ WSH    7:05 PM    ◉ Tonight
               EDM @ CGY    7:35 PM    ◉ Tonight
  Tue Apr 29   TBL @ BOS    7:00 PM
               VGK @ LAK   10:00 PM
  Wed Apr 30   (no games)
  ...
```

**Search** (`/`): type a team or matchup:
- `SEA` → filter to Kraken games only
- `SEA WSH` or `SEA vs WSH` → filter to SEA/WSH matchups specifically
- Shows past results + upcoming with predicted start times

**Team schedule view** (`Enter` on a game or search result):
```
  SEATTLE KRAKEN — 2025-26 Schedule

  ✓  Oct 10  SEA 3-2 ARI (OT)
  ✓  Oct 12  SEA 1-4 EDM
  ...
  ◉  Apr 28  SEA @ VGK  10:00 PM  ← tonight
  ○  Apr 30  SEA vs CGY   7:00 PM
  ...
```

---

### 5. Groups

**Purpose**: Fantasy group management, player watchlists, favorites.

Same as current Groups screen. No major changes.

**Enhancements**:
- `d` on a group → show depth comparison of group members (where each sits across all teams)
- Export group to CSV for fantasy import

---

### 6. Playoffs

**Purpose**: Current (or historical) playoff bracket, series tracking, time-travel to any Cup run.

**Bracket view** (default):
```
  2025-26 STANLEY CUP PLAYOFFS

  FIRST ROUND          SECOND ROUND         CONF FINAL      FINAL
  ┌─────────────┐
  │ (A1) FLA    │──┐
  │ (WC2) TBL   │  ├──(FLA 4-2)──┐
  └─────────────┘  │              │
  ┌─────────────┐  │              ├──────┐
  │ (A2) TOR    │──┘              │      │
  │ (A3) BOS    │                 │      ├──────── FINAL ──
  └─────────────┘                 │      │
  ...                             │      │
```

- `Enter` on a series → series detail (game-by-game results, leading scorers)
- `y` to time-travel to a historical bracket (1987-88, 1993-94, 2003-04, etc.)
- Historical seasons show complete bracket with all results
- Greyed out when no playoffs active (regular season) with message "Playoffs begin [date]"

**Series detail view**:
```
  FLA vs TBL  ·  FLA wins 4-2

  Game 1   FLA 4-2 TBL  (FLA leads 1-0)
  Game 2   TBL 3-2 FLA OT  (Tied 1-1)
  Game 3   FLA 3-1 TBL  (FLA leads 2-1)
  ...

  LEADING SCORERS (this series)
  1.  Reinhart (FLA)   3G 5A  8Pts
  2.  Barkov (FLA)     2G 4A  6Pts
  3.  Point (TBL)      1G 5A  6Pts
```

**Historical Cup runs** (time-travel use case):
- 1993-94 Rangers
- 2003-04 Tampa (last Cup before lockout)
- 2004-05 (lockout — no season)
- Any of the 38 bundled seasons

---

## Admin

`Fetch+Install` moves out of the nav bar. Access via:

- **`:fetch all`** — fetch latest NHL data
- **`:install 20242025`** — install a season bundle
- **`:data list`** — show installed seasons
- **`F` key** on any screen → opens admin overlay

The `:` command prompt is a lightweight input overlay (similar to vim's `:`).

---

## Data Requirements

| Screen | Source | Notes |
|--------|--------|-------|
| League / Stats | Bundled season data | Already working |
| Scores (today) | `/v1/score/now` | Need to wire up |
| Scores (live goals) | `/v1/gamecenter/{id}/boxscore` | New endpoint |
| Schedule | `/v1/schedule/{date}` or `/v1/club-schedule/{team}/week/{date}` | New |
| Playoffs (current) | `/v1/playoff-bracket/{year}` | New endpoint |
| Playoffs (historical) | Bundled with season data | Need to add to bundles |
| Player card (live) | Already working | |

---

## Open Questions

1. **Scoring toggle scope** — should `s` (Fantasy ↔ Pts/82) apply globally across all tabs
   or only on League/Stats? Global is more consistent; local is less surprising.

2. **Historical playoff data** — the bundled data currently only includes skater stats.
   Should playoff bracket/series results be bundled too, or fetched on demand?

3. **Live score polling** — how frequently should Scores refresh?
   Suggestion: every 30s automatically when on Scores tab, manual `r` otherwise.

4. **Schedule search** — should team schedule be integrated into the existing `/` search
   or a dedicated `Schedule` tab search?

5. **`:` admin prompt** — is this discoverable enough, or should there be an `Admin`
   overlay accessible from the help screen?
