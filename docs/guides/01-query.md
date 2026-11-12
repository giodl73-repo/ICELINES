# The Query Engine

## Date-range team scoring

Use `query leaders` for inclusive NHL game-date windows, including windows
that cross seasons. It counts only scoring earned for the requested team;
players who later left that team remain in the result. Regular-season and
playoff games are queried separately.

```powershell
icelines query leaders --team BOS --date-from 2026-03-01 --date-to 2026-11-11 --refresh
icelines query leaders --team BOS --date-from 2026-03-01 --date-to 2026-11-11 --sort ev-points --json --out scoring.json
icelines serve
```

Open **IceStats → Query a date range**, or `/leaders`. Choose a team,
start date, end date, and points/EV-points sort, then **Refresh data & run**.
**Run cached query** replays the exact saved window without network access.
The web view includes all skaters, points and EV ranks (ties share ranks), CSV,
JSON with source URLs/digests, and print/save-PDF styling. A bookmark preserves
the query; optional `highlight=<NHL-player-ID>` emphasizes a player. The JSON
and CSV endpoint is `/api/v1/leaders` with the same query parameters
and optional `format=csv`.

Date-range reports share the browser workbench's **Night Game**, **Fresh Ice**,
and **Hockey Club** palettes. Appearance preferences are stored per browser origin. The report
also has compact/comfortable table spacing; themes and spacing survive query form
submission and can be preserved with `theme=` and `density=` URL parameters.

Use `--pos F` (or C/LW/RW/D) to filter positions. F includes centers and both
wings; NHL L/R codes match LW/RW. Use `--roster all` (default), `current`, or
`former` to include everyone who appeared, only players on the captured current
roster, or only players outside it. In the web form these are **All players who
appeared**, **Current roster only**, and **Not on current roster**. Membership
is as of the separately displayed roster capture time, not the historical
window's end date. Outside-current-roster players include departures and
assignments; this is not a contract-ownership or organization-wide filter.
Refresh captures roster evidence when a membership filter is selected. Cached
membership queries require that evidence and fail visibly if it is absent.
JSON exports include its URL, digest and timestamp. Position and membership
filters apply after historical team/date aggregation and recalculate ranks
among the selected players; they never add games from another team.

P/G and EVP/G divide each player's points and EV points by that player's GP
within the selected window. The table/CSV show three decimals; JSON and sorts
retain full precision. Use `--sort ppg` or `--sort ev-ppg` for rate ordering. Every table header is
sortable; click again to toggle ascending/descending order. Sorting preserves
all displayed statistics, filters, highlight, theme and spacing. Use
`--order asc` or `--order desc` (default) in the CLI; the web/API use `order=`.
All thirteen displayed columns can be sort keys, including names, positions,
games, goals, assists, special-teams points and the two count ranks.

```powershell
icelines query leaders --team BOS --date-from 2026-03-01 --date-to 2026-11-11 --pos F --roster current --sort ev-ppg --refresh --json
```

EV uses the NHL's `evPoints` field, including 3-on-3 and 4-on-4; it is not
5-on-5-only scoring. Each row must satisfy P = G + A = EV P + PP P + SH P.
The acquisition paginates official per-game summary records, checks unique
player/game pairs, and reconciles the game set against official completed
season schedules. Missing reports or unfinished games fail the refresh and
retain the prior capture. A completed-game check cannot guarantee the NHL will
never revise a player's official scoring afterward.

Captures live under the configured snapshot root's `date-ranges/` directory.
They retain exact source payloads and SHA-256 digests, acquisition time and
query scope; a later refresh replaces only the same exact window. Copy the
capture alongside an exported result to preserve dated evidence. Refresh is
explicit, observes `--no-live` and the shared fetch lock, and has a three-minute
deadline (plus a 90-second roster deadline when selected). Windows are limited to three years; the end date must precede today
in UTC. An exact saved window is required for offline queries.

`icelines query` supports team date-window scoring within **leaders** (ranked leaderboards),
**player** (deep profiles), and **compare** (head-to-head or similarity).

---

## query leaders

Ranked leaderboard with flexible filtering and 30+ sort metrics.

### Basic usage

```bash
icelines query leaders                              # top 20 by pts/82
icelines query leaders --top 50                    # top 50
icelines query leaders --pos C --top 15            # centers only
icelines query leaders --team SEA                  # Kraken only
```

### Sort metrics

```bash
--sort pts-pace      # pts per 82 (default)
--sort ppg           # pts per game
--sort g-pace        # goals per 82
--sort pp-pts-pace   # PP pts per 82 (power play specialists)
--sort sh-pct        # shooting percentage
--sort plus-minus    # +/- (best defenders)
--sort toi           # ice time per game
--sort hits-pace     # hits per 82 (physical players)
--sort improvement   # Y/Y PPG delta (who's breaking out)
--sort xgf-pct       # Expected Goals For % at 5v5 (requires fetch money-puck)
```

See `src/data/sort-metrics.md` for the complete list.

### Demographic filters

```bash
# Young Finnish forwards
icelines query leaders --nationality FIN --pos F --age-max 26 --sort ppg

# 2022 draft class
icelines query leaders --draft-year 2022 --sort pts-pace

# Ontario/Quebec players
icelines query leaders --birth-province ON,QC --sort pts-pace --top 10

# Undrafted gems
icelines query leaders --undrafted --ppg-min 0.60 --sort ppg

# This season's rookies
icelines query leaders --rookie --sort ppg --top 15
```

### Statistical thresholds

```bash
# Minimum 40 GP, at least 0.80 PPG
icelines query leaders --gp-min 40 --ppg-min 0.80

# Top-pair defensemen (18.5+ min/game)
icelines query leaders --pos D --toi-min 18.5 --sort pts-pace

# Best positive +/- players
icelines query leaders --sort plus-minus --gp-min 50 --top 15
```

### Multi-season aggregation

Aggregate stats across N bundled seasons (requires `icelines data install --seasons N`):

```bash
# 3-year aggregate — most consistent C producers
icelines query leaders --seasons 3 --pos C --sort pts-pace --top 10

# 5-year aggregate — franchise-level production
icelines query leaders --seasons 5 --sort pts-pace --top 15
```

Column header shows `"Pts/82 (3yr)"` so it's always clear you're viewing aggregates.

### League percentiles

```bash
icelines query leaders --pos D --sort pts-pace --top 10 --percentiles
# Adds Pctl column: "100th", "99th", "98th" etc.
```

### Export

```bash
icelines query leaders --top 50 --json     # JSON array
icelines query leaders --top 50 --csv      # CSV with header row
icelines query leaders --top 50 --json > leaders.json
```

---

## query player

Deep profile on a single player: current stats, PP/SH breakdowns, career arc, league rank.

```bash
icelines query player "Connor McDavid"
icelines query player "McDavid" --percentiles        # shows league rank
icelines query player "Celebrini" --percentiles      # #1 RW, 98th percentile
```

Output includes:
- Current season: G, A, Pts, PPG, PP goals/pts, GWG, shots, SH%, +/-, TOI, FO%
- Contract status (if `fetch contracts` has been run)
- League rank among position peers
- Career arc: 5 seasons of season-by-season stats (up to 38 with data install)
- Peak season, career PPG

```bash
# Situational breakdown (Phase 5C — requires shift data)
icelines query player "McDavid" --breakdown situation
```

---

## query compare

Two modes: **head-to-head** or **similarity search**.

### Head-to-head

```bash
icelines query compare "McDavid" "MacKinnon"
icelines query compare "Bouchard" "Makar"
```

Shows side-by-side: position, age, draft, GP, PPG, Pts/82, Goals/82,
PP points, PP goals, GWG, shots, SH%, +/-, TOI, contract info.

### Similarity search

Find the N most similar players using Z-score Euclidean distance.
Matches on: PPG, goals-per-game, draft pedigree — within age ±2 cohort.

```bash
icelines query compare "Matty Beniers" --similar 8
icelines query compare "Lane Hutson" --similar 5
```

Output shows similarity % (100% = identical profile):

```
SIMILAR PLAYERS TO Matty Beniers (SEA · C · Age 24 · 2021 R1#2)
──────────────────────────────────────────────────────────────────
Rank  Player            Team  Age  Draft      PPG    Similarity
   1  Mason McTavish    ANA   23   21 R1#3   0.547      77%
   2  Marco Rossi       VAN   25   20 R1#9   0.700      73%
   3  Dawson Mercer     NJD   25   20 R1#18  0.512      70%
```

---

## Breakout leaders

Show players most improved year-over-year (requires 2 bundled seasons):

```bash
icelines query leaders --sort improvement --pos F --gp-min 40 --top 15
```

Shows a special table with `Curr | Prior | Δ PPG` columns.
Only includes players who appeared in both seasons with ≥10 GP — no rookie inflation.

---

## Contract filters (requires `icelines fetch contracts`)

```bash
icelines query leaders --ufa --sort pts-pace --top 20  # UFA free agents
icelines query leaders --rfa --pos C --top 15          # RFA centers
icelines query leaders --elc --sort ppg --top 10       # Entry-level players
icelines query leaders --expiry-year 2026 --sort pts-pace
```

If no contract data has been fetched, a hint message guides you.

---

## Examples gallery

```bash
# Who are the best U23 centers right now?
icelines query leaders --pos C --age-max 23 --sort ppg --top 15

# Which 2022 draft picks delivered?
icelines query leaders --draft-year 2022 --sort pts-pace --top 20

# Power play specialists (top PP pts/82, 40+ GP)
icelines query leaders --sort pp-pts-pace --gp-min 40 --top 15

# Best Finnish players in the NHL
icelines query leaders --nationality FIN --sort ppg --top 10

# Toughest players (most hits per 82)
icelines query leaders --sort hits-pace --gp-min 50 --top 15

# Who plays most like Beniers?
icelines query compare "Beniers" --similar 8

# 3-year consistent producers among Swedish D
icelines query leaders --nationality SWE --pos D --seasons 3 --sort pts-pace
```


Date bounds extend the existing `query leaders` command and `/leaders` report;
they do not select a separate UI. Season and date-window adapters render the
same shared query shell, controls, applied-filter strip, table and export tools.
The applied-filter strip remains above the table when printing, including the
franchise name, position group, inclusive dates, roster scope and ordered sorts.
It describes the completed query, not unsubmitted form edits.

For numbered multi-column sorting, click a primary heading, then Shift-click
additional headings in priority order. Shift-click an active heading reverses
its direction without changing its priority; an ordinary click returns to one
sort column. Shift+Enter provides the same interaction from the keyboard.
Bookmarks, filters and exports preserve the ordered chain. The CLI uses
`--then <column>:<asc|desc>` (repeatable or comma-separated), for example:

```powershell
icelines query leaders --team BOS --date-from 2026-03-01 --date-to 2026-11-11 --pos F --roster current --sort ppg --order desc --then ev-ppg:desc --json-envelope
```

`--json` keeps the existing bare-array contract; `--json-envelope` includes
source and query context. Row limits apply after filtering/sorting, while
point/EV ranks describe the whole matching population. Legacy date-range
bookmarks redirect to `/leaders`; the old command is a hidden compatibility
alias. Date-window evidence currently supports team, position, roster and
scoring sorts; unsupported season-only demographic/stat-expression constraints
fail explicitly instead of being silently ignored.
