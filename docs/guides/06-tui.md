# The Interactive TUI

IceLines ships an interactive terminal UI built on ratatui. The default TUI is
the Jack Adams dashboard: a scores ribbon, Favorites/watchlist pane, central
workspace, Schedule/context pane, and command bar, all driven by the same data
the CLI uses. Launch it with:

```bash
icelines tui
```

The TUI loads instantly with the bundled dataset; no fetch required. Use
`icelines tui --classic` for the older tabbed single-document UI.

---

## Dashboard command bar

Press `:` to open the command bar. It accepts the same product language as the
CLI, but stays inside the dashboard:

```text
stats
goalies
team EDM
player Connor McDavid
query g >= 30 AND age <= 25
box EDM@BOS
gaps cats=hits,blocks,shots top=8
poach rw cats=hits,blocks free top=12
fantasy poach top=8 available
simulate add=Connor_McDavid drop=Bench_Forward weeks=3
fantasy simulate add Connor_McDavid drop Bench_Forward
simulate clear
roster
class 2024
```

Press `?` in dashboard mode for the full command reference. `Ctrl+H` toggles
Favorites, `Ctrl+L` toggles Schedule, and `/help` opens the same reference from
the command bar.

---

## Classic tabs

```
┌────────┬────────┬────────┬──────────┬────────┬──────────┐
│ League │ Stats  │ Scores │ Schedule │ Groups │ Playoffs │
└────────┴────────┴────────┴──────────┴────────┴──────────┘
   1         2        3        4          5        6
```

| Tab | Default sub-view | Sub-views (←/→) |
|-----|-------------------|------------------|
| 1 League   | Team rankings | Home ↔ Depth (cross-team strength) |
| 2 Stats    | Projections   | Projections ↔ Queries (interactive builder) |
| 3 Scores   | Today's NHL games (live) | — |
| 4 Schedule | This week | weekly grid; date nav; team / matchup search |
| 5 Groups   | Watchlists | list ↔ member detail |
| 6 Playoffs | Bracket (v2 stub) | — |

`Tab` cycles forward through tabs; `1`–`6` jumps directly. `←/→` switches
sub-views inside the current tab — except on Schedule, where they navigate
between weeks.

---

## Global keys

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `?` | Help overlay (any key dismisses) |
| `Tab` | Next tab |
| `1`–`6` | Jump to tab N |
| `←/→` | Sub-view switch · on Schedule, prev/next week |
| `↑↓` | Move cursor / select row |
| `Enter` | Drill down |
| `Esc` | Back / cancel search |
| `/` | Search (player on most screens; team / matchup on Schedule) |
| `r` | Refresh / retry current view |
| `g` | Add highlighted player to a group |
| `f` | Add highlighted player to **Favorites** instantly |
| `y` | Open the season picker (time-travel) |
| `F` | Admin overlay (install status, fetch hints) |
| `t` | On Schedule: jump to today's week |

The active season shows in the nav bar as `[2021-22]` whenever you're not on
the current season.

---

## League tab

Default view: 32 teams ranked by aggregate skater pace score. Press `Enter`
on a team to drill into the lineup card; `Enter` on a player opens the player
profile with career arc, percentiles, and projection.

`←/→` toggles to **Depth**: cross-team line value rankings. Press `s` on
either Depth view to switch the scoring mode (Fantasy ↔ Pace).

```
Press → from League → Depth
Press Enter on EDM → team lineup card
Press Enter on McDavid → full player profile
Press c on a player profile → similar-player comps
Press g → add to a group
```

---

## Stats tab

Two sub-views joined by `←/→`:

**Projections** — top players by projected points/82, with their PPG and games
played. `↑↓` scrolls; `Enter` opens the player.

**Queries** — interactive query builder. Each row is a filter (position, age,
pts/82 range, etc.); `←/→` cycles values, `Space` toggles focus between the
filter editor and the results pane. Press:

- `s` → save the current query under a name
- `l` → load a saved query
- `r` → reset to defaults
- `Enter` (in results) → open the player

Saved queries persist in `~/.icelines/icelines.db`.

---

## Scores tab

Live NHL games for any date. The default is today's slate, fetched from
`api-web.nhle.com/v1/schedule/now`. Each game shows start time (ET), home
/ away team abbrevs, and — for playoff games — the series score and
game number:

```
  TONIGHT
  ────────────────────────────────────────────────────────────────
   7:05 PM   NYR @ WSH   NYR 2-2 WSH · Game 5
             NYR leads series 2-2
   7:35 PM   EDM @ CGY   EDM 3-1 CGY · Game 5
             EDM leads series 3-1

  ←  2026-04-27    2026-04-29  →
  Times shown in ET  ·  data from NHL public API
```

| Key | Action |
|-----|--------|
| `↑↓`   | Select a game row |
| `←/→`  | Previous / next day |
| `t`    | Jump back to today (live) |
| `d`    | Open the date picker (`YYYY-MM-DD` or `MM/DD`) |
| `Enter`| Open the boxscore for the highlighted game |
| `r`    | Retry the fetch for the active date |

### Game detail

`Enter` on any game opens its boxscore — every goal with scorer, assists,
period and time, plus goalies (saves / shots / decision). Playoff games
also surface the series score line:

```
┌─── NYR 2 – 3 WSH · OT · Esc back ────────────────────────────────┐
│   NYR 2-3 WSH · Game 5                                            │
│   WSH leads series 3-2                                            │
│                                                                    │
│   GOALS                                                            │
│     1st  08:14  Alex Ovechkin   (WSH) — Kuznetsov, Carlson  0-1   │
│     1st  17:55  Mika Zibanejad  (NYR) — Trocheck, Fox       1-1   │
│     2nd  11:44  Artemi Panarin  (NYR) — Trocheck            2-1   │
│     3rd  19:58  Dylan Strome    (WSH) — Backstrom, Jensen   2-2   │
│     OT   03:22  Tom Wilson      (WSH) — Ovechkin            2-3   │
│                                                                    │
│   GOALTENDERS                                                      │
│     Igor Shesterkin (NYR)  32 saves / 35 shots (L)                │
│     Charlie Lindgren (WSH) 28 saves / 30 shots (W)                │
│                                                                    │
│   Esc to return to scores                                          │
└────────────────────────────────────────────────────────────────────┘
```

### Date picker

Press `d` to type any date directly:

```
┌─── Go to date — Enter applies, Esc cancels ─────┐
│   Go to: 2026-01-15█                            │
└──────────────────────────────────────────────────┘
```

Accepts full ISO (`2026-01-15` / `2026/01/15`) or month-day shorthand
(`01/15` / `01-15`, current year inferred). Invalid input keeps the
picker open with an inline error so you can correct.

### Caching

Each date's games and each game's boxscore are cached independently —
past dates are permanent (final scores don't change), today and live
games revalidate on `r`. Switching back to a previously visited date is
instant. If the API is unreachable, the tab keeps the last known data
and shows an error line; press `r` to retry.

---

## Schedule tab

The full NHL schedule, navigable by week. The default view is the current
week (Monday–Sunday) grouped by date:

```
┌─── Schedule · Week of Apr 27 — May 3 · /:search ←→:week t:today ────┐
│                                                                       │
│   Mon Apr 27                                                          │
│     Final OT  3-2    SEA @ VGK                                       │
│     Final     1-4    NYR @ WSH                                       │
│                                                                       │
│   Tue Apr 28                                                          │
│     7:05 PM          NYR @ WSH    NYR 2-2 WSH · Game 5              │
│                                                                       │
│   ...                                                                 │
│                                                                       │
│   3 game(s) shown · times in ET                                       │
└──────────────────────────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| `←/→` | Previous / next week |
| `t`   | Jump to today's week |
| `/`   | Open the search bar |
| `Enter` (with filter) | Open team-season or matchup view |
| `r`   | Retry the current week if the fetch failed |
| `↑↓`  | Select a game row |

### Search: team or matchup

Press `/` to open the search bar at the bottom. Type one or two team
abbreviations:

| Input | Result |
|-------|--------|
| `SEA` | Filter the week to Kraken games |
| `NYR WSH` | Filter to NYR vs WSH games (also `nyr vs wsh`, `nyr @ wsh`) |

Lowercase is normalized; `Enter` applies the filter. Validation errors
("Unknown team", "Cannot search same team vs itself", "Too many teams")
keep the search bar open so you can correct your input.

After a filter is applied, **press `Enter` again** (outside search mode) to
drill down:

- **Team filter** → Full-season schedule for that team with W-L-OT record
  and color-coded results (green = win, yellow = OT loss, red = regulation
  loss).
- **Matchup filter** → Head-to-head game log with regular-season and
  playoffs sections, plus a record line from the first team's perspective.

```
┌─── NYR vs WSH — Season Series ─────────────────────────────────────┐
│   Regular season: NYR 1-1 WSH    ·    Playoffs: NYR 1-2 WSH         │
│   ──────────────────────────────────────────────────────────────────│
│   Regular Season                                                     │
│     Tue Nov 18  WSH 3-2 NYR  Final                                  │
│     Mon Jan 5   NYR 4-1 WSH  Final                                  │
│                                                                      │
│   Playoffs                                                           │
│     Sun Apr 20  WSH 4-2 NYR  Final  Game 1                          │
│     Tue Apr 22  WSH 3-1 NYR  Final  Game 2                          │
│     Thu Apr 24  NYR 5-2 WSH  Final  Game 3                          │
│     Sat Apr 26  WSH 3-2 NYR  Final (OT)  Game 4                     │
│                                                                      │
│   4 matchup(s) · Esc back                                            │
└──────────────────────────────────────────────────────────────────────┘
```

`Esc` clears the filter and returns to the week view.

### Caching and refreshing

The Schedule tab maintains a per-week cache. Opening the tab pre-fetches
the current week plus the next two; switching to a previously-visited week
is instant. Past weeks are cached permanently (final scores don't change);
the current week revalidates on `r`.

Network failure on a single week shows an explicit error message — other
cached weeks remain accessible via `←/→`.

---

## Groups tab

Watchlists backed by `~/.icelines/icelines.db`:

```bash
# Manage groups from the CLI
icelines group create "My Watchlist"
icelines group add "My Watchlist" "McDavid"
icelines group show "My Watchlist"
```

In the TUI, `Enter` on a group opens its members. From any player-list
screen, press `g` to open a picker overlay and add the highlighted player
to an existing group, or `f` to instantly add to the auto-created
**Favorites** group.

Members appear with their team, position, and projected pts/82. Players
who aren't in the currently loaded season show as `(not in current data)`.

---

## Playoffs tab

A list-style bracket of the current postseason, drawn from
`api-web.nhle.com/v1/playoff-bracket/{year}`. Rounds appear as a header
strip; the active round's series are listed below, grouped by conference:

```
┌─── Playoffs · 25-26 · ↑↓:series ←→:round Enter:detail r:retry y:season ──┐
│                                                                              │
│   First Round  │  Second Round  │  Conference Final  │  Stanley Cup Final   │
│   ─────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│   EASTERN CONFERENCE                                                         │
│     (A1 ) FLA  vs  (WC2) TBL    FLA 4-2 TBL · FLA wins                      │
│     (M1 ) WSH  vs  (WC1) NYR    WSH leads 3-1                               │
│                                                                              │
│   WESTERN CONFERENCE                                                         │
│     (P1 ) EDM  vs  (WC2) VAN    EDM 4-1 VAN · EDM wins                      │
│                                                                              │
│   Round 1 of 4 · 8 series · Enter for series detail                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| `↑↓` | Move between series in the active round |
| `←/→` | Switch round (clamps at first / last) |
| `Enter` | Open the highlighted series |
| `r` | Retry if the bracket fetch failed |
| `y` | Season picker — switch to a historical year |
| `Esc` | (in series detail) back to the bracket |

Series rows are color-coded: green when a winner is decided, white when
games have been played, dim when the series hasn't started.

### Series detail

`Enter` on a series opens its detail view:

```
┌─── Series A · Esc back ────────────────────────────────────────────────────┐
│                                                                              │
│   Florida Panthers (A1)  vs  Tampa Bay Lightning (WC2)                       │
│   Eastern Conference                                                         │
│                                                                              │
│   FLA 4-2 TBL · FLA wins                                                     │
│   ────────────────────────────────────────────────────────                  │
│                                                                              │
│   GAMES                                                                      │
│     6 game(s) played so far                                                  │
│                                                                              │
│   Per-game scores + scorers ship with bundled playoffs.json (v2).           │
│                                                                              │
│   Esc to return to bracket                                                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

For an in-progress series the next game is labelled either **upcoming**
(mandatory — the trailing team's wins force it) or **(if needed)** —
deferred until the previous game decides whether it's required.

### Off-season and historical seasons

When the bracket endpoint returns no rounds (off-season), the tab shows a
short message instead of an empty grid. Press `y` to switch to a
historical year. Per-game logs and leading scorers for historical
seasons require the bundled `playoffs.json` per season — that bundling is
deferred to v2; today the live API is the authoritative source.

---

## Season time-travel

Press `y` from any screen to open the season picker:

```
┌─── Select Season — ↑↓ · Enter · i:install · Esc:cancel ───┐
│  ▶ 2025-26  (current)                                       │
│  ✓ 2024-25                                                  │
│  ✓ 2023-24                                                  │
│  ✓ 2022-23                                                  │
│  ✓ 2021-22                                                  │
│    2020-21  (COVID bubble)              [not installed]     │
│    ...                                                       │
│    ✗ 2004-05  LOCKOUT — no season                           │
│  ...                                                         │
└──────────────────────────────────────────────────────────────┘
```

- Five seasons (2021-22 through 2025-26) ship bundled in the binary.
- Selecting an uninstalled season prompts you to install — press `i` from
  the picker, or run `icelines data install <season>` in your shell.
- The 2004-05 lockout row is marked unselectable; pressing `Enter` on it
  is rejected with a status-bar message.
- Once a season is loaded, the nav bar shows `[YYYY-YY]` until you switch
  back to the current season.

Live tabs (Scores, Schedule) remain on real-time data regardless of the
selected analytical season — historical bracket / schedule playback ships
with the Playoffs work.

---

## Admin overlay

Press `F` (capital F) to open the admin overlay — install status, fetch
hints, and links to the underlying CLI commands:

```bash
icelines fetch all              # rosters + stats from NHL API
icelines data list              # show installed seasons
icelines data install 20032004  # install a specific historical season
```

`Esc` closes the overlay. The overlay is non-modal for navigation —
background install progress continues to animate in the status bar even
after dismissal.

---

## Status bar

The bottom line of the screen shows the current status — what loaded, the
result of the last action, install progress, validation errors, and so on.
It's the primary feedback channel for non-modal events:

```
  ✓ Added Connor McDavid to Favorites
  Installing 20122013… ⠋
  ⚠ Unknown team: 'XYZ'. Try: SEA, NYR, EDM, ...
  Filter: NYR vs WSH — Enter for head-to-head
```

---

## Tips

- **Quick player lookup**: `/`, type a name fragment, `Enter` — works from
  any tab.
- **Fast group adds**: highlight any player and press `f` to add to
  Favorites with one keystroke.
- **Compare two players**: open one's profile, press `c` for similar
  players, drill into a comp.
- **Fantasy research flow**: Stats tab → Queries → save query → switch to
  Groups → confirm watchlist → back to Schedule for matchup planning.

---

## Next

- [Getting Started](00-getting-started.md) — installation and first queries
- [Query Engine](01-query.md) — the CLI side of `query leaders / player / compare`
- [Team Depth Charts](02-team-depth.md) — what powers the Depth sub-view
- [Data & History](04-data.md) — managing historical seasons (install,
  uninstall, time-travel)
