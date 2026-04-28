# Playoffs Screen — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Draft — not yet implemented

---

## Purpose

Display the NHL playoff bracket for the current season or any historical season.
This is the time-travel screen — users can relive any Stanley Cup run from the
38 bundled seasons. During the live playoffs, it shows current series status and
updates automatically.

---

## Layout

### Bracket view (default)

```
┌─── 2025-26 Stanley Cup Playoffs  ──────────────────────────────────────────┐
│  First Round          Second Round       Conf Final      Stanley Cup Final  │
│                                                                              │
│  EASTERN CONFERENCE                                                          │
│  ┌─────────────┐                                                            │
│  │(A1) FLA 4-2 │──┐                                                         │
│  │(WC2) TBL    │  ├─ FLA ──┐                                                │
│  └─────────────┘  │         │                                                │
│  ┌─────────────┐  │         ├─ ? ──┐                                        │
│  │(A2) TOR 2-2 │──┘         │      │                                        │
│  │(A3) BOS     │            │      ├──────── ? ──────────                  │
│  └─────────────┘            │      │                                        │
│                              │      │                                        │
│  ┌─────────────┐             │      │                                        │
│  │(M1) WSH 3-1 │──┐         │      │                                        │
│  │(WC1) NYR    │  ├─ WSH ───┘      │                                        │
│  └─────────────┘  │                 │                                        │
│  ┌─────────────┐  │                 │                                        │
│  │(M2) CAR 3-2 │──┘                 │                                        │
│  │(M3) NJD     │                    │                                        │
│  └─────────────┘                    │                                        │
│                                                                              │
│  WESTERN CONFERENCE  (similar layout)                                        │
│  ...                                                                         │
│                                                                              │
│  ↑↓ select series · Enter: series detail · y: change season · Esc: back   │
└──────────────────────────────────────────────────────────────────────────────┘
```

Series box contents:
- `(A1) FLA 4-2` = seeding, winning team, series score
- `(WC2) TBL` = seeding, eliminated team
- In-progress: `(A2) TOR 2-2` with `*` or blinking indicator

---

## Series Detail View

Press `Enter` on any series box.

```
┌─── Florida Panthers vs Tampa Bay Lightning  ────────────────────────────────┐
│  Eastern Conference First Round  ·  FLA wins 4-2                             │
│                                                                               │
│  GAMES                                                                        │
│  Game 1  Apr 19   FLA 4 – 2  TBL   FLA leads 1-0                            │
│  Game 2  Apr 21   TBL 3 – 2  FLA   Tied 1-1                                 │
│  Game 3  Apr 23   FLA 3 – 1  TBL   FLA leads 2-1                            │
│  Game 4  Apr 25   FLA 5 – 2  TBL   FLA leads 3-1                            │
│  Game 5  Apr 27   TBL 4 – 1  FLA   FLA leads 3-2                            │
│  Game 6  Apr 29   FLA 3 – 2  TBL   FLA wins 4-2  ●                         │
│                                                                               │
│  LEADING SCORERS (this series)                                                │
│   1  Reinhart   FLA   3G  5A   8 Pts                                          │
│   2  Barkov     FLA   2G  4A   6 Pts                                          │
│   3  Point      TBL   1G  5A   6 Pts                                          │
│   4  Stamkos    TBL   3G  2A   5 Pts                                          │
│   5  Tkachuk M  FLA   2G  3A   5 Pts                                          │
│                                                                               │
│  ↑↓ select game · Enter: game detail (scores only for historic) · Esc: back  │
└───────────────────────────────────────────────────────────────────────────────┘
```

---

## Historical Season View (Time-Travel)

Accessed via `y` season picker. Any of 38 bundled seasons.

```
┌─── 1993-94 Stanley Cup Playoffs  ─────────────────────────────────────────┐
│  [Historical season — read only]                                             │
│                                                                              │
│  STANLEY CUP CHAMPION:  New York Rangers                                     │
│  MVP (Conn Smythe):     Brian Leetch                                         │
│                                                                              │
│  (Full bracket rendered as above with completed results)                     │
│                                                                              │
│  Conference Finals:  NYR def. NJ (4-3)  ·  VAN def. TOR (4-1)              │
│  Stanley Cup Final:  NYR def. VAN (4-3)                                      │
│                                                                              │
│  Notable:  Rangers' first Cup in 54 years                                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

Notable Cup wins and facts shown for the selected season (static data bundled per season).

---

## Off-Season State

When no playoffs are active and the current season is regular season:

```
┌─── Playoffs ───────────────────────────────────────────────────────────────┐
│                                                                              │
│  Regular season in progress.                                                 │
│  Playoffs begin approximately:  Apr 19, 2026                                │
│                                                                              │
│  Current playoff picture (projected):                                        │
│  Eastern: FLA · TBL · TOR · BOS · WSH · NYR · CAR · OTT                    │
│  Western: EDM · VGK · DAL · COL · MIN · WPG · SEA · VAN                    │
│                                                                              │
│  y: browse historical playoffs                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Data Sources

| Data | Source | Notes |
|------|--------|-------|
| Current bracket | `GET /v1/playoff-bracket/{year}` | e.g. year=2026 |
| Series status | `GET /v1/schedule/now` `seriesSummary` | In schedule response |
| Game results | `GET /v1/score/{date}` or bundled | Per-game results |
| Series leaders | `GET /v1/gamecenter/{id}/boxscore` | Aggregated per series |
| Historical brackets | Bundled with season data | Offline, time-travel |
| Cup/MVP facts | Static bundled JSON | One record per season |

---

## Data Bundling for Historical Seasons

Each installed season bundle includes:
```
~/.icelines/seasons/20032004/
  bios.json
  stats.json
  playoffs.json     ← bracket + all series results + game log
  cup-facts.json    ← champion, MVP, notable facts
```

`playoffs.json` structure:
```json
{
  "season": "20032004",
  "champion": "TBL",
  "conn_smythe": "Brad Richards",
  "rounds": [
    {
      "round": 1,
      "series": [
        {
          "top_seed": "TBL", "bottom_seed": "WSH",
          "winner": "TBL", "games": 5,
          "results": [
            { "date": "2004-04-08", "home": "TBL", "away": "WSH",
              "home_score": 3, "away_score": 1, "series_after": "TBL 1-0" },
            ...
          ]
        }
      ]
    }
  ]
}
```

---

## Key Bindings

| Key | Action |
|-----|--------|
| `↑↓` | Move between series boxes |
| `←→` | Move between rounds (in bracket view) |
| `Enter` | Open series detail |
| `y` | Open season picker (time-travel) |
| `r` | Refresh (live playoffs only) |
| `Esc` | Back / close detail |

---

## App State

```rust
pub playoffs_season:  Season,         // which season's bracket to show
pub playoffs_cache:   PlayoffsCache,  // Arc<Mutex<PlayoffsState>>
pub playoffs_cursor:  (usize, usize), // (round, series) selection
```

---

## Open Questions

1. **Bracket visual layout** — the ASCII bracket is complex. Should we use a simplified
   list-style bracket instead for v1 of this screen?
2. **Series leaders** — requires aggregating boxscore data per series. Expensive to
   compute for historical seasons. Defer to v2 of this screen?
3. **Projected playoff picture** — during regular season, show projected bracket based
   on current standings? Nice-to-have, not blocking.
4. **Cup facts database** — who maintains and updates this? Propose: a `cup-facts.json`
   committed to the repo, one entry per season, maintained by hand.
