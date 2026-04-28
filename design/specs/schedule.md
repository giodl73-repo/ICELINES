# Schedule Screen — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Draft — not yet implemented

---

## Purpose

Show the full NHL schedule — past results and upcoming games — filterable by team
or matchup. The primary use case: "When do the Rangers play the Kraken?" and
"Show me Seattle's remaining schedule."

---

## Layout

### Default — this week

```
┌─── Schedule  ·  /: search team or matchup  ·  ←→: week ──────────────────┐
│                                                                              │
│  Mon  Apr 28    NYR @ WSH     7:05 PM   ◉ Tonight  [PLAYOFFS G5]          │
│                 EDM @ CGY     7:35 PM   ◉ Tonight  [PLAYOFFS G5]          │
│                 VGK @ LAK    10:00 PM   ◉ Tonight  [PLAYOFFS G4]          │
│                                                                              │
│  Tue  Apr 29    TBL @ BOS     7:00 PM                                       │
│                 FLA @ TOR     7:30 PM                                       │
│                                                                              │
│  Wed  Apr 30    (no games scheduled)                                         │
│                                                                              │
│  Thu  May 1     CAR @ NYI     7:00 PM                                       │
│                 COL @ DAL     9:30 PM                                       │
│                                                                              │
│  ────────────────────────────────────────────────────────────────────────── │
│  ← Apr 21-27                                                  Apr 29-May 5 →│
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Search / Filter Mode

Press `/` to open the search bar at the bottom.

**Team filter** — type one team abbreviation:
```
  Search: SEA█
```
→ Filters schedule to only SEA games:
```
  Mon  Apr 28    SEA @ VGK    10:00 PM   ◉ Tonight
  Sat  May 3     SEA vs CGY    7:00 PM
  Tue  May 6     SEA @ EDM     9:30 PM
  ...
```

**Matchup filter** — type two team abbreviations:
```
  Search: NYR WSH█     (or "NYR vs WSH" or "NYR @ WSH")
```
→ Shows only NYR/WSH games with results + upcoming:
```
  Oct 12   WSH 3-2 NYR   Final
  Nov 18   NYR 4-1 WSH   Final
  Jan 5    NYR 2-3 WSH   Final (OT)
  Apr 28   NYR @ WSH     ◉ Tonight [PLAYOFFS G5]
  May 1    WSH @ NYR     Upcoming  [PLAYOFFS G6]
```

**Clear search**: `Esc` clears filter and returns to week view.

---

## Team Schedule View

Press `Enter` on any game row — if a team filter is active, goes to full team schedule.

```
┌─── SEATTLE KRAKEN — 2025-26 Schedule  ────────────────────────────────────┐
│  82 games  ·  Played: 71  ·  Remaining: 11                                 │
│                                                                             │
│  ✓ Oct 10   SEA  3 – 2  ARI  (OT)     W   1- 0- 0                        │
│  ✓ Oct 12   SEA  1 – 4  EDM           L   1- 1- 0                        │
│  ✓ Oct 14   SEA  4 – 3  VAN           W   2- 1- 0                        │
│  ...                                                                        │
│  ✓ Apr 25   SEA  2 – 1  CGY           W  42-22- 7                        │
│  ◉ Apr 28   SEA     @   VGK  10:00 PM    ← tonight                        │
│  ○ May 1    SEA  vs  CGY   7:00 PM                                          │
│  ○ May 3    SEA     @   EDM   9:30 PM                                       │
│  ...                                                                        │
│                                                                             │
│  Record: 42W-22L-7OT  (91 pts)  3rd Pacific                               │
│                                                                             │
│  Esc: back to schedule                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

Rows are color-coded: green = W, red = L, yellow = OT loss.

---

## Matchup History View

When a two-team search filter is active and `Enter` is pressed:

```
┌─── NYR vs WSH — 2025-26  ─────────────────────────────────────────────────┐
│  Season series: NYR 1-3 WSH  (Playoffs: WSH leads 3-1)                     │
│                                                                             │
│  Regular Season                                                             │
│  Oct 12   WSH 3-2 NYR   Final                                               │
│  Nov 18   NYR 4-1 WSH   Final                                               │
│  Jan 5    NYR 2-3 WSH   Final (OT)                                          │
│  Feb 22   WSH 4-2 NYR   Final                                               │
│                                                                             │
│  Playoffs (First Round)                                                     │
│  Apr 20  Game 1  WSH 4-2 NYR                                                │
│  Apr 22  Game 2  WSH 3-1 NYR                                                │
│  Apr 24  Game 3  NYR 5-2 WSH                                                │
│  Apr 26  Game 4  WSH 3-2 NYR (OT)                                           │
│  Apr 28  Game 5  NYR @ WSH  7:05 PM  ◉ Tonight                             │
│  Apr 30  Game 6  WSH @ NYR  7:00 PM  (if needed)                            │
│                                                                             │
│  Esc: back to schedule                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Data Sources

| Data | NHL API Endpoint | Notes |
|------|-----------------|-------|
| Weekly schedule | `GET /v1/schedule/{YYYY-MM-DD}` | 7-day window |
| Team schedule | `GET /v1/club-schedule-season/{team}/{season}` | Full season |
| Game results | Included in schedule response | Scores for completed games |

---

## Key Bindings

| Key | Action |
|-----|--------|
| `↑↓` | Select game row |
| `←→` | Previous / next week |
| `/` | Open search (team or matchup) |
| `Enter` | Game detail or team schedule |
| `Esc` | Clear search / back |
| `t` | Jump to today's week |

---

## App State

```rust
pub schedule_query:  String,         // current search filter
pub schedule_week:   String,         // "YYYY-MM-DD" of week start (Monday)
pub schedule_data:   ScheduleCache,  // Arc<Mutex<ScheduleState>>
```

Data is fetched per-week and cached. Navigating to a week not in cache triggers a fetch.
Pre-fetch: current week + 2 weeks ahead on first Schedule tab open (3 requests, parallelized).

---

## Search Input Validation (resolved from TAPE + BENCH blockers)

Team code validation:
- Input normalized to uppercase before matching
- Validated against canonical 32-team list in `icelines-core::teams`
- Unknown code: `Unknown team: 'XYZ'. Try: SEA, NYR, EDM, ...`
- Case-insensitive: `nyr` → `NYR`

Edge cases:
| Input | Behavior |
|-------|----------|
| `SEA SEA` (same team twice) | Error: "Cannot search same team vs itself" |
| `NYR INVALID` (one valid, one invalid) | Error for the invalid code; not partial match |
| `nyr wsh` (lowercase) | Normalized to `NYR WSH`, works |
| Empty string, `Esc` | Clear filter, return to week view |
| Single space | Treated as empty, ignored |
| `Backspace` | Remove last character from query |
| `Ctrl+U` | Clear entire search input |

---

## Partial Fetch Degradation (resolved from WIRE blocker)

When a week fetch fails:
- Show explicit message: `Schedule unavailable for week of Apr 28 [r: retry]`
- Do NOT show empty rows — empty rows are indistinguishable from "no games scheduled"
- Other cached weeks remain accessible via `←→`
- Retry is triggered by `r` key; exponential backoff: 1s, 2s, 4s, max 3 attempts

---

## Season Time-Travel

When `active_season` is not current:
- Data from bundled `schedule.json` in that season's bundle
- If bundle lacks `schedule.json`, shows "Schedule data not bundled for this season"
- Future games show as "not played" (obvious from historical context)
- Useful for reviewing how a playoff run unfolded game by game

---

## Decisions (Open Questions resolved)

1. **Pre-fetch**: Current week + 2 weeks ahead (3 parallel requests). Enough for near-future
   planning; avoids over-fetching the full season on first open.
2. **Playoff schedule integration**: Playoff games appear naturally in schedule (they are
   in the NHL schedule API). No separate integration needed — the game_type field distinguishes
   them. Playoffs bracket (Series view) is a separate screen; link via `Enter` on playoff game.
3. **Division/conference filter**: Deferred to v2.1. Not blocking.
