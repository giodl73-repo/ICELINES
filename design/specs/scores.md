# Scores Screen — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Draft — not yet implemented

---

## Purpose

Show live NHL game scores for any date. During the playoffs, surface series context
(who leads, what game number). Support date navigation so users can look back at any
game from any season.

---

## Layout

### Default — today's games

```
┌─── Scores — Tuesday Apr 28, 2026  [PLAYOFFS R1]  s:mode  r:refresh ───────┐
│                                                                              │
│  ◉ LIVE  7:05 PM    NYR  2 – WSH  3    Overtime              Game 5        │
│          WSH leads series 3-1                                                │
│                                                                              │
│  ◉ LIVE  7:35 PM    EDM  1 – CGY  0    2nd period 14:22      Game 5        │
│          Series tied 2-2                                                     │
│                                                                              │
│  ○  PRE  10:00 PM   VGK       –  LAK                         Game 4        │
│          VGK leads series 2-1                                                │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────  │
│                          ←  Mar 27    Mar 29  →                             │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Regular season game row
```
  ✓ FINAL   7:00 PM    TOR  4 – BOS  2    Final                W 4-2
  ○  PRE    9:30 PM    VGK     –  LAK                          (14-18-5)
```

### Game status indicators
| Symbol | Meaning |
|--------|---------|
| `◉ LIVE` | Game in progress |
| `✓ FINAL` | Game complete |
| `○  PRE` | Not yet started |

---

## Game Detail View

Press `Enter` on any game row.

```
┌─── NYR 2 – WSH 3  ·  OT  ·  Game 5  ·  WSH leads series 3-1 ──────────────┐
│                                                                               │
│  GOALS                                                                        │
│  1st  08:14  Ovechkin (WSH)  — Kuznetsov, Carlson         WSH 1-0           │
│  1st  17:55  Zibanejad (NYR) — Trocheck, Fox              TIE 1-1           │
│  2nd  11:44  Panarin (NYR)   — Trocheck                   NYR 2-1           │
│  3rd  19:58  Strome (WSH)    — Backstrom, Jensen          TIE 2-2           │
│  OT   03:22  Wilson (WSH)    — Ovechkin                   WSH 3-2  ●FINAL   │
│                                                                               │
│  SERIES RESULTS                                                               │
│  Game 1  WSH 4-2 NYR                                                         │
│  Game 2  WSH 3-1 NYR                                                         │
│  Game 3  NYR 5-2 WSH                                                         │
│  Game 4  WSH 3-2 NYR (OT)                                                    │
│  Game 5  WSH 3-2 NYR (OT)  ●                                                 │
│                                                                               │
│  GOALTENDERS                                                                  │
│  Shesterkin (NYR)  32 saves / 35 shots                                        │
│  Lindgren (WSH)    28 saves / 30 shots                                        │
│                                                                               │
│  Esc: back                                                                    │
└───────────────────────────────────────────────────────────────────────────────┘
```

For regular season games, series results section is replaced by standings context:
```
│  STANDINGS CONTEXT                                                            │
│  TOR  26-18-5  (61 pts)  3rd Atlantic                                        │
│  BOS  30-12-7  (67 pts)  2nd Atlantic                                        │
```

---

## Date Navigation

- `←` / `→` arrows move one day back or forward
- Any date with games is navigable, back to the start of the selected season
- Non-current dates show `FINAL` for all games (no live state)
- Dates with no games show: `No games scheduled`

**Jump to date**: `d` opens an inline date picker:
```
  Go to: 2026-01-15█   (YYYY-MM-DD or MM/DD)
```

---

## Data Sources

| Data | NHL API Endpoint | Notes |
|------|-----------------|-------|
| Schedule | `GET /v1/schedule/now` | Today's games |
| Schedule (date) | `GET /v1/schedule/{YYYY-MM-DD}` | Any date |
| Live scores | `GET /v1/score/now` | Updates every ~20s |
| Game detail | `GET /v1/gamecenter/{gameId}/boxscore` | Goals, goalies |
| Series status | In schedule JSON `seriesSummary` | Playoff only |

---

## Key Bindings

| Key | Action |
|-----|--------|
| `↑↓` | Select game row |
| `←→` | Previous / next date |
| `Enter` | Open game detail |
| `r` | Manual refresh (live scores) |
| `d` | Jump to specific date |
| `Esc` | Back / close game detail |

---

## Auto-Refresh

When the Scores tab is active:
- Poll `GET /v1/score/now` every **30 seconds** automatically
- Show last-updated timestamp in the nav bar: `Updated 14s ago`
- `r` forces immediate refresh
- Polling pauses when Scores tab is not active (no background drain)

---

## App State

```rust
pub tonight_cache:  TonightCache,   // Arc<Mutex<TonightState>>
pub scores_date:    String,         // "YYYY-MM-DD", defaults to today
```

`TonightState`:
```rust
pub enum TonightState {
    Idle,
    Loading,
    Loaded(Vec<ScheduledGame>),
    Error(String),
}
```

Fetch is triggered when the Scores tab is first shown (`Idle → Loading`).
Date changes clear the cache and trigger a new fetch.

---

## Playoff vs Regular Season

**Detection rule**: A date is a playoff date if `game.game_type == 3` for any game on
that date. This is the authoritative check — not calendar date, not standings. If a date
has both regular (type=2) and playoff (type=3) games (possible during transition), playoff
games show series context and regular games show records; both are shown.

Section header logic:
- All games type=3 → `[PLAYOFFS R{round}]` where round is read from `seriesSummary.seriesAbbrev`
- All games type=2 → `[REGULAR SEASON]`
- Mixed → `[PLAYOFFS + REGULAR]` (rare, end-of-regular/start-of-playoffs overlap)
- Type=1 (preseason) → shown dimmed with `[PRE]` label

**Game status symbols** — terminal compatibility:
| Status | Primary | ASCII fallback |
|--------|---------|---------------|
| In progress | `◉ LIVE` | `* LIVE` |
| Final | `✓ FINAL` | `+ FINAL` |
| Not started | `○ PRE` | `- PRE` |

Fallback used when `TERM` is `dumb` or `NO_COLOR` is set. On Windows Terminal, primary
symbols are tested and render correctly in Cascadia Code; on other fonts, fallback is safer.

**Series status layout**: Series status line must fit in 80 columns alongside team names.
Maximum format: `{ABBR} leads {N}-{M}` (14 chars max). If terminal width <80, truncate
to `{N}-{M}` only.

---

## Cache Policy (resolved from WIRE blocker)

- Cache key for live scores: `scores_live_{YYYY-MM-DD}` — keyed by date, not global
- Cache survives tab navigation: if user leaves Scores tab and returns within 30s, existing
  cache is used; beyond 30s, a fresh fetch is triggered
- `r` key always clears cache regardless of TTL and triggers immediate fetch
- Past dates (`scores_date < today`): permanent cache, never re-fetched
- Completed games (`game.gameState == "OFF"` or `"FINAL"`): cached permanently

---

## Date Navigation Edge Cases (resolved from BENCH blocker)

| Scenario | Expected behavior |
|----------|-----------------|
| First day of season | `←` shows "No earlier games in this season" |
| Last day of season | `→` shows "No later games in this season" |
| Off-season date (July) | "No games scheduled. Next season begins {date}" |
| Date with no games | "No games scheduled for {date}" — arrows still work |
| Partial API response (some games missing) | Show available games + "[N games unavailable]" |
| In-progress game, API times out | Show last-known score with `[stale]` tag |

---

## Season Time-Travel

When `active_season` is not the current season:
- Schedule data comes from bundled `schedule.json` (not live API)
- All games show as `FINAL` (no live state)
- Game detail shows goal log if bundled; otherwise shows score only
- Top of screen shows: `[Historical — 2003-04]`
- Fantasy scoring mode unavailable for historical seasons where scheme is not bundled
  (shows `[Fantasy mode N/A — no historical scheme bundled]`)

---

## Decisions (Open Questions resolved)

1. **Goal detail**: Show scorer + primary assist only. Secondary assist omitted in v1 to
   fit 80-column layout. Secondary assist added in v2 when wider terminal is assured.
2. **Penalty log**: Not shown in v1. Goals only in game detail. Penalties added in v2.
3. **Goalie stats**: Shown only for completed games (`gameState == "OFF"`). In-progress
   games show saves/shots in real time if available in boxscore response; omitted if not.
4. **Push notifications**: Out of scope for TUI. Noted for future mobile companion.
