# IceLines — Command Reference

Single-page reference for the `icelines` CLI. Every subcommand is listed with a one-line description and 2-3 real examples. Designed so you can read it once and be productive.

If you only have the binary, run `icelines docs` to print this content from inside the binary itself — no internet required.

---

## Quick start

```bash
icelines query leaders --top 10                       # top scorers, current season
icelines query leaders --age-max 24 --filter "hits>=200" --filter "p>=40"
icelines query player "Connor McDavid" --seasons 38   # full bundled career
icelines query compare "Wayne Gretzky" "Mario Lemieux" --seasons 38
icelines query goalies --filter "save-pct>=0.92" --top 10
icelines tui                                          # interactive dashboard
```

**38 seasons (1987-88 → 2025-26)** are bundled into the binary. No internet, no fetch, no setup needed.

---

## Filter grammar

Every catalog stat is filterable through `--filter`. Multiple `--filter` flags are ANDed at the top level. **A single `--filter` value can also use AND / OR / NOT / parens** for richer expressions.

```
--filter "<expr>"

<expr>     := <or-expr>
<or-expr>  := <and-expr> ( OR <and-expr> )*
<and-expr> := <unary>    ( AND <unary>   )*
<unary>    := NOT <unary> | <primary>
<primary>  := '(' <expr> ')' | <atom>
<atom>     := <stat> <op> <value>

<stat>   any cli_key from the catalog (108 stats) or a short alias
<op>     >=  <=  >  <  ==
<value>  number; locale-comma `,` rejected — use `.`
```

Precedence: `NOT > AND > OR`. Standard left-associativity. Keywords `AND`/`OR`/`NOT` are case-insensitive and must be at word boundaries (won't collide with stat keys).

```bash
# Plain atom — backward compatible
icelines query leaders --filter "g>=50"

# OR — either side qualifies
icelines query leaders --filter "g>=50 OR a>=80" --season 19921993

# AND — both must hold (also expressible as multiple --filter args)
icelines query leaders --filter "g>=30 AND a>=30"
icelines query leaders --filter "g>=30" --filter "a>=30"   # equivalent

# NOT — invert
icelines query leaders --filter "NOT pim>=100" --filter "p>=70"

# Parens — group / override precedence
icelines query leaders --filter "(g>=30 AND a>=30) OR p>=80"

# Multiple --filter still ANDed at top level — combine freely
icelines query leaders --filter "g>=20 OR a>=40" --filter "gp>=70"
```

### Short aliases

| Short | Canonical | Short | Canonical |
|---|---|---|---|
| `g` | `goals` | `gp` | `games` |
| `a` | `assists` | `ppg` | `points-per-game` |
| `p`, `pts` | `points` | `gpg` | `goals-per-game` |
| `s`, `sog` | `shots` | `apg` | `assists-per-game` |
| `pen` | `pim` | `pace` | `pace-82` |
| `+/-` | `plus-minus` | `sv%`, `sv` | `save-pct`, `saves` |
| `blk`, `blocks` | `blocked-shots` | `w`, `l`, `so`, `ot` | `wins`, `losses`, `shutouts`, `ot-losses` |
| `tk` / `gv` | `takeaways` / `giveaways` | `ga` / `sa` | `goals-against` / `shots-against` |
| `mis` | `missed-shots` | `fow%`, `fow-pct` | `faceoff-win-pct` |
| `shootingpct`, `sh%` | `shooting-pct` | | |

Filter keys are case-insensitive: `--filter "HITS>=200"` resolves correctly.

`age` is **not** a stat — use the `--age-min N` / `--age-max N` flags on `query leaders` instead.

### Common filter recipes

```bash
# Young power forward
icelines query leaders --age-max 24 --filter "hits>=200" --filter "p>=40"

# Clean scorer (high points, low penalties)
icelines query leaders --filter "p>=50" --filter "pim<=30"

# 30-30 club
icelines query leaders --filter "g>=30" --filter "a>=30"

# Defensive forward profile
icelines query leaders --filter "hits>=150" --filter "blk>=50" --filter "tk>=40"

# Vezina-shortlist goalie
icelines query goalies --filter "gp>=30" --filter "save-pct>=0.92"

# 3-season aggregate of any pattern
icelines query leaders --seasons 3 --age-max 25 --filter "hits>=600" --filter "p>=120"
```

### `--seasons N` — multi-season aggregate

Aggregates stats across the last N bundled seasons (1-38). Available on `query leaders`, `query player`, `query compare`. On player/compare, it controls how many seasons of career-arc rows print after the head-to-head table.

```bash
icelines query leaders --seasons 5 --filter "g>=200" --top 20
icelines query player "Wayne Gretzky" --seasons 38         # full career arc
icelines query compare "Connor McDavid" "Sidney Crosby" --seasons 10
```

---

## `query` — the main analytics surface

### `query leaders` — leaderboard

Top-N players by any of 30+ sort metrics, filtered by every dimension on `PlayerFilter` plus the `--filter` catalog grammar.

```bash
icelines query leaders --top 20
icelines query leaders --pos C --sort ppg --top 15
icelines query leaders --season 19921993 --filter "g>=50"   # Lemieux era 50-goal club
icelines query leaders --seasons 3 --filter "p>=300" --top 10
icelines query leaders --json | jq '.[] | .full_name'
```

**Sort metrics** (use either canonical key or short alias):
- Points: `pts-pace` (default), `ppg`, `pts`/`p`, `goals`/`g`, `assists`/`a`, `gp`
- Power play: `pp-pts-pace`, `pp-g-pace`, `pp-pts`, `pp-g`
- Shorthanded: `sh-g-pace`, `sh-g`
- Other: `gwg`, `shots`/`s`, `sh-pct`, `plus-minus`/`+/-`, `toi`, `fo-pct`
- Physical: `hits`, `blocks`/`blk`, `takeaways`/`tk`, `giveaways`/`gv`, `pim`/`pen`
- Advanced: `xg`, `xg-per-60`, `cf-pct`, `ff-pct`, `xgf-pct` (requires `fetch money-puck`)
- Trend: `improvement` — Y/Y PPG delta vs prior season

**Flag-based filters**: `--pos`, `--team`, `--age-min`/`--age-max`, `--nationality`, `--draft-year`, `--draft-round`, `--draft-pick-max`, `--undrafted`, `--rookie`, `--handedness`, `--gp-min`/`--gp-max`, `--toi-min`, `--ppg-min`, `--plus-minus-min`, `--shots-pg-min`, `--birth-province`.

**Output**: `--json` / `--csv` to a stream, or `--out PATH` to a file.

### `query player NAME` — player profile

Career arc, percentile rank, full stats. Searches both skater AND goalie bios. Historical players resolve without `--season` via cross-bundled name lookup.

```bash
icelines query player "Connor McDavid"                       # current season
icelines query player "Wayne Gretzky" --seasons 38           # full career arc
icelines query player "Patrick Roy" --season 19951996        # historical goalie
icelines query player "McDavid" --percentiles --rank-by g    # rank by goals not pts
icelines query player "McDavid" --filter "gp>=60"            # narrow peer pool
```

### `query compare PLAYER1 PLAYER2` — head-to-head

Side-by-side stats. With `--seasons N`, prints each player's career arc afterward.

```bash
icelines query compare "Connor McDavid" "Sidney Crosby"
icelines query compare "Wayne Gretzky" "Mario Lemieux" --seasons 38
icelines query compare "Matty Beniers" --similar 8           # similarity search
icelines query compare "McDavid" --similar 5 --filter "gp>=20"   # narrowed cohort
```

### `query goalies` — goalie leaderboard

Same filter grammar as `query leaders`, with goalie-context rewriting: `gp` → `goalie-games`, `starts` → `goalie-starts`.

```bash
icelines query goalies --top 10
icelines query goalies --filter "gp>=30" --filter "save-pct>=0.92"   # Vezina shortlist
icelines query goalies --filter "wins>=30" --filter "so>=4"
icelines query goalies --season 19981999 --top 10                    # Hasek era
icelines query goalies --json
```

---

## Top-level analytics commands

```bash
icelines rank --top 10 [--pos C|LW|RW|D]    # league pace-score ranking
icelines rank --json
icelines team EDM                            # team depth chart with cross-team fit
icelines players --top 20                    # full PlayerFilter surface
icelines history "Connor McDavid"            # season-by-season log
icelines project "Celebrini" --mode pace     # rest-of-season projection
icelines scouting "Evan Bouchard"            # 8-section scouting report
icelines peers "Lane Hutson" --size 8        # draft-class statistical peers
icelines class 2022 --top 15                 # full draft class ranked
icelines compare "McDavid" "MacKinnon"       # alias for `query compare`
icelines mates "Beniers" --top 5             # linemates (requires fetch shifts)
```

## `x` — quick CSV/JSON export

One-shot export of any report shape to stdout (default CSV) or a file. Excel-friendly.

```bash
icelines x leaders --top 20                     # CSV to stdout
icelines x leaders --top 20 --json --out top.json
icelines x goalies --top 10
icelines x history --player "McDavid"           # uses --player flag
icelines x peers --player "Hutson"
icelines x class --year 2015                    # uses --year flag
icelines x compare --player "McDavid" --opponent "Crosby"
```

Shapes: `rank`, `leaders`, `goalies`, `players`, `class`, `history`, `peers`, `compare`, `transactions`.

## `export md` — markdown tables

Deterministic markdown output for documentation / reports.

```bash
icelines export md leaders --top 10 --out leaders.md
icelines export md goalies --top 5 --out goalies.md
icelines export md leaders --columns "g,a,p,blk" --out custom.md
```

---

## Fantasy league

```bash
# Setup
icelines fantasy league-create --name "My League" --scheme yahoo-default-points
icelines fantasy team-create --name "My Team" --owner "Gio"
icelines fantasy team-add --team "My Team" --player "McDavid"

# Manage
icelines fantasy team-show --team "My Team"
icelines fantasy standings
icelines fantasy league-list
icelines fantasy league-switch --name "My League"

# Trades
icelines fantasy trade --from "My Team" --to "Other" --send "Bouchard" --receive "Werenski"
icelines fantasy trade ... --execute     # commit instead of simulate

# Web dashboard
icelines fantasy serve --port 8080
# GET /                    HTML standings
# GET /api/standings       JSON
# GET /api/team/<name>     team JSON
# POST /api/trade          simulation JSON
```

## `scheme` — fantasy scoring schemes

```bash
icelines scheme list
icelines scheme show yahoo-default-points
icelines scheme from-csv path/to/yahoo.csv      # detect platform, build template
```

Built-in schemes: `yahoo-default-points`, `yahoo-default-rotisserie`, `head-to-head-9cat`, `simple-points`.

---

## Data and history

```bash
# Fetch fresh data (optional — bundles work immediately)
icelines fetch all                    # rosters + stats (~5 min)
icelines fetch realtime               # hits, blocks, giveaways, takeaways
icelines fetch money-puck             # xG, CF%, FF%, xGF% (free)
icelines fetch contracts              # UFA/RFA/ELC

# Historical data is bundled — every season since 1987-88. No install required.
# `data install` is for keeping a local mirror up to date if you fetch fresh data.
icelines data list                    # show installed seasons
icelines data install --season 19881989
icelines data remove 19921993
```

The bundled-data cap is 38 seasons because `BUNDLED_SEASONS` is the canonical source. The 2004-05 lockout has no data and never will.

---

## TUI (`icelines tui` or `icelines dashboard`)

Interactive dashboard. Six tabs (League, Depth, Stats, Goalies, Scores, Schedule) plus Playoffs and Transactions overlays. Player cards lazy-load every player's full historical career across all 38 bundled seasons on first open.

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle tabs forward / backward |
| `↑↓` / `←→` | Navigate within a tab |
| `Enter` | Drill into selection (team / player / game) |
| `Esc` / `q` | Back / quit |
| `?` | Help overlay |
| `R` | **Reports overlay** — toggle Tier-1 reports loaded into columns |
| `y` | **Season picker** — jump to any of the 38 bundled seasons |
| `Shift+P` | Toggle Regular ↔ Playoff for the active season |
| `o` | Toggle the current section on Stats (Queries) |
| `[` / `]` | Cycle career-table column presets on a player card |
| `/` | Open the sort picker (search-as-you-type 108 stats) |
| `r` | Refresh current view |
| `d` | Jump to depth chart (or jump-to-date on Scores) |
| `F` | Toggle admin overlay |
| `g` | Add to group from a player card / team roster |
| `s` / `l` | Save / load query (Queries tab) |

The Reports overlay (`R`) persists toggles to `~/.icelines/config.toml`. Disabled reports drop their columns from career tables, sort pickers, and query results.

---

## Other commands

```bash
# Live data
icelines tonight                       # tonight's NHL games
icelines tonight --team EDM
icelines schedule --days 7             # upcoming schedule
icelines transactions --season 20242025

# Player groups (SQLite-backed watchlists)
icelines group create "Watchlist"
icelines group add "Watchlist" "McDavid"
icelines group show "Watchlist"

# Personal-attendance tracker
icelines games add ...                 # log games attended
icelines games list

# Snapshots (data versioning)
icelines snapshot list
icelines snapshot verify

# Trade simulator (depth-chart impact, not fantasy)
icelines trade "Bouchard" for "Fox" --team EDM

# Site generation
icelines build                         # generate mkdocs site
icelines serve                         # serve site locally
icelines deploy                        # deploy to GitHub Pages
```

---

## Global flags

These work on every subcommand:

```bash
--no-live              # disable all live NHL API fetches (deterministic CI mode)
--no-dashboards        # disable the TUI dashboard side panel
ICELINES_NO_LIVE=1     # env var equivalent of --no-live
ICELINES_DASHBOARDS=0  # env var equivalent of --no-dashboards
```

Live-feed precedence: CLI flag > env var > config file > default (live ON).

---

## Output formats

| Flag | Where | Example |
|---|---|---|
| `--json` | `query leaders/goalies/player/compare`, `rank`, `x` | `--json` to stdout |
| `--csv` | `query leaders/goalies`, `rank`, `x` (default) | `--csv` to stdout |
| `--out PATH` | `query`, `rank`, `x`, `export md` | Writes to file |
| `--columns "k1,k2,..."` | `export md`, `query leaders` | Comma-separated cli_keys / aliases |

`--json` and `--csv` together: `query goalies` errors on conflict; `query leaders` silently picks JSON. Avoid passing both.

---

## Where to look next

- `README.md` — installation + tutorial primer
- `design/specs/stat-catalog.md` — the StatId catalog (108 stats, categories, units, report sources)
- `CLAUDE.md` — AI / contributor context (crate ownership, key constants, architecture rules)
- `cargo run --release -- --help` — clap-generated per-command help with all flags
- `icelines tui` then `?` — interactive help overlay covering every keybind

If you find a gap between this document and the binary's actual behavior, that's a bug — please open an issue or PR.
