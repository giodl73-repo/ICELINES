# IceLines TUI v2 — Design Spec

**Version**: 2.0
**Date**: 2026-04-28
**Status**: Draft — pre-implementation

> Current v1 state is in `tui.md`. This spec describes the v2 redesign.

---

## Goals

1. Reduce tab count from 8 to 6 — remove cognitive overhead
2. Move administrative screens (Fetch+Install) out of the nav bar
3. Add live game data: Scores, Schedule, Playoffs
4. Enable season time-travel across all screens
5. Merge thin tabs (Queries + Projections → Stats)

---

## Tab Structure

```
 League │ Stats │ Scores │ Schedule │ Groups │ Playoffs
   1        2       3         4         5         6
```

| # | Tab | v1 Equivalent | Change |
|---|-----|---------------|--------|
| 1 | **League** | League + Depth | Depth moves here as sub-view |
| 2 | **Stats** | Queries + Projections + Search | Merge into one tab |
| 3 | **Scores** | Tonight (stub) | Full live scores + date nav |
| 4 | **Schedule** | — | New screen |
| 5 | **Groups** | Groups | No change |
| 6 | **Playoffs** | — | New screen |

### Removed from nav

- `/Search` → stays as `/` overlay from any screen
- `Tonight` → merged into Scores (tab 3)
- `Projections` → sub-view within Stats (tab 2)
- `Fetch+Install` → admin overlay via `F` key or `:` command

---

## Season Time-Travel

A global feature available on any screen via `y`.

**Season picker overlay:**
```
┌─── Select Season ──────────────────────────────┐
│  ▶ 2025-26  (current — live data)               │
│    2024-25  ✓ installed                         │
│    2023-24  ✓ installed                         │
│    2022-23  ✓ installed                         │
│    2021-22  ✓ installed                         │
│    ─────────────────────────────────────────    │
│    2003-04  [not installed]                     │
│    2001-02  [not installed]                     │
│    ...38 seasons total...                       │
│                                                 │
│  ↑↓ · Enter confirm · i install · Esc cancel   │
└─────────────────────────────────────────────────┘
```

- Installed seasons in full color; uninstalled dimmed
- `i` on an uninstalled season → trigger install without leaving picker
- Selecting a season updates `App.active_season` and reloads all screens
- Nav bar shows season indicator when not current: `[2003-04] `
- Playoff tab for historical seasons shows that year's completed bracket

**App state changes:**
```rust
pub active_season: Season,   // default = CURRENT_SEASON
```

All data reads go through `active_season`. Current-season screens (live scores, tonight)
show a "Historical season — no live data" message when non-current season is active.

---

## Tab 1: League (Depth Rankings)

Combines current League (Home) and Depth tabs.

**Default sub-view**: Depth rankings (cross-team line value, was tab 7).
Rationale: this is the most analytically rich view; team roster list is a drill-down.

**Sub-views** (toggle with `←→` or number keys within tab):
```
[ Depth Rankings ]  [ Team List ]
```

**Depth Rankings** — same as current `Screen::Depth`:
- 32 teams ranked by depth score
- Scoring mode `s` toggle (Fantasy / Pts/82)
- `Enter` → team depth chart (LW | C | RW | LD | RD)

**Team List** — same as current `Screen::Home`:
- 32 teams with pace scores and fit class counts
- `Enter` → team roster

Both sub-views share a common `Enter → team depth chart` drill-down.

---

## Tab 2: Stats

**Sub-views:**
```
[ Projections ]  [ Queries ]  [ Comps ]
```
Toggle with `←→` within the tab.

**Projections** — current Projections screen, unchanged.

**Queries** — current Queries screen, unchanged.

**Comps** — type a player name, get similar-player list.
Currently only accessible from Player card (`c`); this makes it a first-class view.

**Search** remains a `/` overlay across all sub-views.

---

## Tab 3: Scores

See `scores.md` for full spec.

**Summary:**
- Default: today's games with live scores
- `←→` navigates dates (any date in history)
- Playoff games show series status prominently
- `Enter` on a game → boxscore detail (goals, penalties, key stats)
- Auto-refreshes every 30s when on this tab; `r` for manual refresh

---

## Tab 4: Schedule

See `schedule.md` for full spec.

**Summary:**
- Default: this week's schedule for all teams
- `/` search: type a team (`SEA`) or matchup (`SEA WSH`) to filter
- `Enter` on a game → game detail or team schedule view
- Covers full season; useful for "when do the Rangers play the Kraken"

---

## Tab 5: Groups

No change from v1 Groups screen.

**Potential v2 enhancement** (not blocking):
- `d` on a group → depth comparison (where each member sits across all teams)

---

## Tab 6: Playoffs

See `playoffs.md` for full spec.

**Summary:**
- Default: current (or selected season's) playoff bracket
- Visual bracket with round-by-round progression
- `Enter` on a series → series detail (game log, leading scorers)
- `y` time-travel shows historical brackets (Stanley Cup campaigns)
- Dimmed with "Playoffs begin [date]" during regular season

---

## Admin Overlay

Replaces Fetch+Install tab.

**Access:** `F` key from any screen, or `:` command prompt.

```
┌─── Admin ──────────────────────────────────────┐
│                                                  │
│  f  fetch all (current season)                   │
│  F  force-fetch all                              │
│  i  install season bundle                        │
│  l  list installed seasons                       │
│  r  remove season bundle                         │
│                                                  │
│  Esc to close                                    │
└──────────────────────────────────────────────────┘
```

The `:` command prompt accepts vim-style commands:
```
:fetch all
:install 20242025
:data list
```

---

## Global Key Binding Changes (v1 → v2)

| Key | v1 | v2 |
|-----|----|----|
| `1`–`6` | Jump to tabs 1-6 | Jump to tabs 1-6 (tabs renumbered) |
| `7`,`8` | Depth, Fetch | Removed |
| `y` | — | Open season picker |
| `F` | — | Open admin overlay |
| `:` | — | Command prompt |
| `s` | Depth screens only | Any stats screen (scoring mode toggle) |
| `←→` | Query field values | Query field values + sub-view navigation within a tab |

---

## Migration Notes

### v1 → v2 Screen mapping

| v1 Screen | v2 Location |
|-----------|-------------|
| Home (League list) | League tab, sub-view 2 |
| Team roster | League tab → drill-down |
| Depth (rankings) | League tab, sub-view 1 (default) |
| DepthTeam | League tab → drill-down |
| Queries | Stats tab, sub-view 2 |
| Projections | Stats tab, sub-view 1 |
| Tonight (stub) | Scores tab (fully wired) |
| Groups | Groups tab (unchanged) |
| Fetch+Install | Admin overlay (`F` key) |
| Search | `/` overlay (unchanged) |
| Player card | Unchanged (drill-down from any screen) |
| Comps | Stats tab sub-view 3 + Player card `c` |
| Playoffs | New tab 6 |
| Scores/Schedule | New tabs 3 and 4 |

---

## Implementation Phases

**Phase 1** (nav restructure — no new screens):
- Merge Queries + Projections into Stats tab with `←→` sub-views
- Move Depth into League tab as sub-view
- Move Fetch+Install to `F` admin overlay
- Update tab numbers + key bindings
- Add season indicator to nav bar

**Phase 2** (live data):
- Wire up Scores screen (NHL score API)
- Wire up Schedule screen
- Add `y` season picker

**Phase 3** (Playoffs):
- Playoff bracket screen
- Series detail view
- Historical bracket data bundled with season installs
