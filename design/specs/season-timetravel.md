# Season Time-Travel — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Draft — not yet implemented

---

## Purpose

Let users explore any of the 38 bundled NHL seasons. Pressing `y` on any screen
opens a season picker; selecting a season switches all screens to that year's data.
This makes IceLines a historical research tool, not just a current-season tracker.

Use cases:
- "Show me the 2003-04 depth charts" (last Cup before the lockout)
- "How did the 1987-88 Oilers rank?" (Gretzky era)
- "Which teams had the most buried stars in 2015-16?"
- "Navigate the 1994 Rangers playoff run game by game"

---

## Season Picker Overlay

Triggered by `y` on any screen.

```
┌─── Select Season ──────────────────────────────────────┐
│                                                          │
│  ▶ 2025-26  (current — live)                            │
│    2024-25  ✓ installed                                  │
│    2023-24  ✓ installed                                  │
│    2022-23  ✓ installed                                  │
│    2021-22  ✓ installed                                  │
│    ──────────────────────────────────────────────        │
│    2020-21  ✓ installed  (56-game COVID season)          │
│    2019-20  ✓ installed  (bubble playoffs)               │
│    2018-19  [not installed]                              │
│    2017-18  [not installed]                              │
│    2016-17  [not installed]                              │
│    ...                                                   │
│    2004-05  ✗ LOCKOUT — no season                       │
│    2003-04  [not installed]                              │
│    ...                                                   │
│    1987-88  [not installed]                              │
│                                                          │
│  ↑↓ select  ·  Enter confirm  ·  i install  ·  Esc cancel│
└──────────────────────────────────────────────────────────┘
```

**Legend:**
- `▶` = currently active season
- `✓ installed` = data available locally
- `[not installed]` = dim, selectable only after install
- `✗ LOCKOUT` = no season exists, not selectable

**`i` key** on an uninstalled season triggers install in background without
closing the picker. Progress shown as: `2003-04  [installing… 42%]`

---

## Behavior After Selection

When a season is selected:

1. `App.active_season` is updated
2. All screens reload from the new season's data
3. Nav bar shows season indicator: `[2003-04] `
4. Screens that require live data (Scores live feed) show historical mode message

**Screen-by-screen behavior:**

| Screen | Behavior in historical season |
|--------|-------------------------------|
| League | Shows that season's depth rankings |
| Stats (Queries, Projections) | Shows that season's player stats |
| Scores | Shows game results for that season (from bundle) |
| Schedule | Shows that season's full schedule with results |
| Playoffs | Shows that season's bracket (if completed, shows result) |
| Groups | Unchanged (groups are user data, not season-specific) |
| Player card | Shows that season's stats for the player |
| Comps | Compares against that season's active players |
| Depth chart | Shows cross-team rankings for that season |

---

## Season Indicator in Nav Bar

When active season is not the current season, a persistent indicator appears:

```
 League │ Stats │ Scores │ Schedule │ Groups │ Playoffs │  [2003-04]  ←y→  q:quit
```

`y` in the nav bar hint reminds users they can change the season.
The indicator is hidden when current season is active.

---

## App State Changes

```rust
pub active_season: Season,   // default = CURRENT_SEASON (20252026)
```

All data reads check `app.active_season`:
- Bundled data loader reads from `~/.icelines/seasons/{active_season}/`
- Current-season live API calls are skipped for historical seasons
- The season picker reads this to show the `▶` marker

---

## Season Data Requirements

Each season bundle (`icelines data install YYYYZZZZ`) must include:

```
~/.icelines/seasons/YYYYZZZZ/
  bios.json           # player bios (all teams)
  stats.json          # skater stats
  realtime.json       # hits, blocks, etc.
  schedule.json       # full season schedule with results
  playoffs.json       # bracket, series, game log (for playoff seasons)
  cup-facts.json      # champion, MVP, notable facts
```

**Data not available for historical seasons:**
- Live scores (no polling)
- Boxscore goal-by-goal detail (not bundled unless we add it)
- MoneyPuck advanced stats (only available for recent seasons via their API)

---

## Lockout / Shortened Season Handling

| Season | Situation | Display |
|--------|-----------|---------|
| 2004-05 | Full lockout — no games | `✗ LOCKOUT — no season` (not selectable) |
| 2012-13 | 48-game lockout season | `2012-13  (48 games)` — note in picker |
| 2019-20 | COVID bubble | `2019-20  (bubble playoffs)` |
| 2020-21 | 56-game COVID season | `2020-21  (56 games)` |

Stats from shortened seasons display normally; pace/82 normalization handles GP differences.

---

## Installation UX

**From picker:** `i` on an uninstalled season installs in background.
Shows progress in picker row without leaving the screen.

**From admin overlay:** `:install YYYYZZZZ`

**From CLI:** `icelines data install 20032004`

Installed seasons are recorded in `~/.icelines/seasons/manifest.json`:
```json
{
  "installed": ["20252026", "20242025", "20232024", "20222023", "20212022", "20032004"],
  "current": "20252026"
}
```

---

## 38 Bundled Seasons

Seasons available for install (1987-88 through 2025-26, minus 2004-05 lockout):

```
1987-88  1988-89  1989-90  1990-91  1991-92  1992-93  1993-94
1994-95* 1995-96  1996-97  1997-98  1998-99  1999-00  2000-01
2001-02  2002-03  2003-04  [2004-05 LOCKOUT]  2005-06  2006-07
2007-08  2008-09  2009-10  2010-11  2011-12  2012-13* 2013-14
2014-15  2015-16  2016-17  2017-18  2018-19  2019-20* 2020-21*
2021-22  2022-23  2023-24  2024-25  2025-26
```
`*` = shortened season (asterisk shown in picker)

Five seasons ship pre-installed (2021-22 through 2025-26).
Remaining 33 seasons available via `icelines data install`.

---

## Open Questions

1. **Pre-installed historical season** — ship one iconic historical season pre-installed
   (e.g., 2003-04 or 1993-94)? Would increase binary size but improve first-run experience.
2. **Season data hosting** — where are historical bundles stored? GitHub Releases (current
   plan for current-season bundles) works; confirm CDN strategy for 33 historical seasons.
3. **Per-season bundle size** — estimated 2-10 MB per season. Total if all installed: ~100 MB.
   Acceptable on disk; confirm size before bundling.
4. **Goalie data** — should historical season bundles include goalie stats? Currently
   goalies are not in the skater data model. Deferred to separate goalie spec.
