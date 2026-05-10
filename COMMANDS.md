# IceLines — Command Reference

## Test slices

Use the slice runner while developing so you do not have to run the full
workspace suite after every small change.

```powershell
pwsh scripts/test-slice.ps1 list             # show available slices
pwsh scripts/test-slice.ps1 quick            # workspace compile + ViewModel tests
pwsh scripts/test-slice.ps1 viewmodel        # Campbell ViewModel tests
pwsh scripts/test-slice.ps1 cli-matrix       # Foster capability matrix
pwsh scripts/test-slice.ps1 workspace-check  # compile all crates
pwsh scripts/test-slice.ps1 full             # long gate: workspace --no-fail-fast
```

Add `-NoCapture` when you need test stdout:

```powershell
pwsh scripts/test-slice.ps1 viewmodel -NoCapture
```

Recommended rhythm:

- focused code change: run the nearest slice;
- cross-crate type changes: run `quick` or `workspace-check`;
- before commit/push: run `full` when the change affects shared contracts or
  multiple surfaces.

---

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

## Filter grammar — Phase Art Ross (v0.20.0)

Every catalog stat is filterable through `--filter`. Multiple `--filter` flags are ANDed at the top level. **A single `--filter` value can also use AND / OR / NOT / parens** for richer expressions.

### Grammar

```
--filter "<expr>"

<expr>     := <or-expr>
<or-expr>  := <and-expr> ( OR <and-expr> )*
<and-expr> := <unary>    ( AND <unary>   )*
<unary>    := NOT <unary> | <primary>
<primary>  := '(' <expr> ')' | <atom> [EVER] [AT <age-clause>]
<atom>     := <key> <op> <value>
            | <key> [NOT] IN '(' <value> (',' <value>)* ')'
            | <key> BETWEEN <number> AND <number>
            | <key> [NOT] LIKE <pattern>
            | <key> ('~' | '!~') <pattern>
```

Precedence: `NOT > AND > OR`. Keywords (AND, OR, NOT, IN, BETWEEN, LIKE, EVER, AT) are case-insensitive at word boundaries.

### Operators

| Op | Meaning | Notes |
|---|---|---|
| `>=` `<=` `==` `=` | Standard comparators | `g>=10` |
| `<` `>` | Strict comparators | `age<25` |
| `!=` | Not equal | `g!=0` (`<>` typo hint suggests `!=`) |
| `IN (a,b,c)` | Set membership | `country IN (CAN, USA, SWE)` |
| `NOT IN (a,b,c)` | Negated set | `team NOT IN (BOS, NYR)` |
| `BETWEEN x AND y` | Inclusive range | `age BETWEEN 22 AND 28` |
| `LIKE "pat"` | Glob match | `country LIKE "CA*"` (NFD-normalized) |
| `NOT LIKE "pat"` | Negated glob | |
| `~ pat` / `!~ pat` | Substring sugar | |

### Atom keys

**Catalog stats** (108 cli_keys): `goals`, `assists`, `points`, `pim`, `shots`, `hits`, `blocked-shots`, `takeaways`, `giveaways`, plus aliases (`g`, `a`, `p`, `ppg`, `blk`, `tk`, `gv`, `pen`, `+/-`, `sv%`).

**Bio fields:**
- `age` (HR Feb-1 convention via `compute_age`)
- `country` / `nationality` (distinct — country matches birth_country OR nationality_code; nationality matches only nationality_code)
- `shoots` / `hand` / `catches`
- `pos` / `position` (roster primary)
- `team` (current stint), `team.any` (any stint this season)
- `draft-year` / `draft`, `draft-round`, `draft-overall`
- `height` / `ht`, `weight` / `wt`
- `birth-state`, `birth-city`
- `rookie-season`

**Sliding-window atoms** (Phase Art Ross A.2): `<stat>.last<N><u>` where `u` is `g` (games-played), `d` (days), `w` (weeks), `m` (months). N is 1..=255 for g/w/m, 1..=65535 for d.
- `g.last10g>=5` — last 10 games played, current team stint, contiguous (default scope)
- `g.last10g.allteams>=5` — last 10 GP across all stints this season
- `g.last10g.career>=5` — last 10 GP crossing season boundaries
- `g.last30d>=10` — last 30 calendar days
- `g.last3w>=8`, `g.last3m>=20`

**Career aggregator atoms** (Phase Art Ross A.3): `<stat>.<aggregator>`
- `p.career>=500` — lifetime sum across all eligible seasons
- `p.streak>=15` — longest run of consecutive games with non-zero stat
- `g.any10g>=5 EVER` — any 10-GP intra-season window across career (axis-typed; lockout 2004-05 skipped; short-circuits on first hit)
- `g.seasons-with>=5` — count of seasons matching predicate

**AT-age modifier** (Phase Art Ross A.3): slice the season set BEFORE aggregation
- `g.any10g>=5 EVER AT age<=25`
- `p.career>=500 AT age BETWEEN 20 AND 25`

**Cross-league career atoms** (Phase Art Ross A.4):
- `league=OHL` — ever played in this league
- `league IN (OHL, WHL, QMJHL)` / `league NOT IN (NHL)`
- `league.tier=Junior` (Pro / Junior / College / International / Other — Phase Calder canonical classification)
- `p.career.junior>=200` — junior-tier career sum
- `p.career.nhl>=500` — NHL-only career sum
- `p.career.ohl>=300` — specific league career sum

### `--explain` (Phase Art Ross A.5)

Print the parsed query plan + data requirements without running the query:

```bash
icelines query leaders --filter "g.last10g>=5 AND age<=25" --explain
icelines query leaders --filter "g.any10g>=5 EVER AT age<=25" --explain --json
```

The JSON envelope follows `explain.v1` shape (frozen v1; additive only; breaking changes ship as `explain.v2`):
```json
{
  "schema_version": "explain.v1",
  "route": "leaders.explain",
  "data": { "plans": [{ "filter_input": "...", "plan_tree_text": "...",
                         "needs_provider": true, "requirements": {...} }] },
  "meta": { "note": "..." }
}
```

### Vision queries

```bash
# "5 goals over 10 games, age <= 25" — current-season streak
icelines query leaders --filter "g.last10g>=5 AND age<=25"

# Same, historical — any 10-GP window across all 38 bundled seasons
icelines query leaders --filter "g.any10g>=5 EVER AT age<=25"

# Junior elite cohorts
icelines query leaders --filter "league.tier=Junior AND p.career.junior>=200"

# Mac* family with elite NHL careers, ever
icelines query leaders --filter 'name LIKE "Mac*" AND p.career.nhl>=500'
```

(Some queries require `icelines fetch boxscore` and/or `icelines fetch career` to populate local caches before they return non-empty results.)

### Errors

The parser emits typed errors with span info. Multi-error reporting is preserved — a 5-atom filter with 3 errors surfaces all 3 in one round-trip.

| Variant | Trigger |
|---|---|
| `EmptyInput` | `--filter ""` |
| `MissingOp` | atom has no op |
| `MultipleOps` | `g>=>=5`, `g===5` |
| `OpTypoHint` | `g=>5` → suggests `>=`; `g=<5` → `<=`; `g<>5` → `!=` |
| `UnknownStat` | unknown cli_key |
| `BadNumber` / `NotFinite` | `g>=many`, `g>=NaN` |
| `EmptySet` | `country IN ()` |
| `IncompatiblePredicate` | `LIKE 5`, `g IN (10, 20, 30)` (use BETWEEN), string field with `>=` |
| `UnknownWindowUnit` | `g.last10z>=5` (suggests g/d/w/m) |
| `ZeroWindowSize` | `g.last0g>=5` |
| `WindowSizeOutOfRange` | `g.last1000g>=5` (max 255 for g/w/m) |
| `FeatureNotYet` | atom routes to a sub-phase that hasn't shipped (e.g. `team.career=` was here pre-A.4) |
| `UnclosedParen` / `UnexpectedRParen` / `UnexpectedEnd` | grammar |

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

### Bio keys inside `--filter` (QueryB)

Bio fields aren't in the StatId catalog (per CLAUDE.md they're per-player facts, not stats), but you can mix them into the top-level AND chain of any `--filter` expression on `query leaders / player / compare / goalies`:

| Key | Operators | Example |
|-----|-----------|---------|
| `age`, `draft` (also `draft-year`), `height` (also `ht`), `weight` (also `wt`) | `>=`, `<=`, `=` | `--filter "age<=24 AND p>=80"` |
| `country` (also `nation`/`nationality`), `shoots` (also `hand`/`catches`) | `=` only | `--filter "country=SWE AND height>=72 AND p>=40"` |

Bio keys go through the shared `icelines-query` engine — same engine the web `/leaders` page uses, so the URL `?filter=age<=24 AND p>=80` returns the same set as the CLI.

Limitation: bio keys must be on the **top-level AND chain** of a single `--filter`. Mixing bio terms inside `OR` / `NOT` / parens isn't extracted (the catalog parser will reject the bio key with a "did you mean a stat?" hint). For OR with bio terms, run the query twice and union the results, or fall back to the discrete `--age-min` / `--age-max` flags.

### Common filter recipes

```bash
# Young power forward (bio mixed into the filter grammar)
icelines query leaders --filter "age<=24 AND hits>=200 AND p>=40"

# Or with discrete --age-max flag
icelines query leaders --age-max 24 --filter "hits>=200" --filter "p>=40"

# Tall Swedish forwards with 40+ pts
icelines query leaders --filter "country=SWE AND height>=72 AND p>=40"

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

### `query career` — cross-league leaderboard (Phase Calder)

Walks the local career-history store (`~/.icelines/career_history.json`)
populated by `icelines fetch career --bundled-seasons 5`, and lists the
top scorers from a non-NHL league + season. Useful for "OHL leaders
2014-15", "AHL goal-scorers 2024-25" — questions `query leaders`
(NHL-only) can't answer.

```bash
icelines query career --league OHL --season 20142015           # McDavid era
icelines query career --league AHL --season 20242025 --top 30
icelines query career --league NCAA --season 20132014 --sort goals
icelines query career --league WHL --json | jq '.data[0]'

# Phase Art Ross — narrow the cohort with bio filters
icelines query career --league OHL --season 20142015 --filter "country=CAN"
icelines query career --league OHL --season 20142015 --filter "pos=C AND age<=18"
icelines query career --league WHL --filter "draft-round<=2"
```

Cohort scope: only players who appeared on an NHL roster in the last
5 bundled seasons (the `fetch career` target). Career-only players
who never reached the NHL aren't in scope. `--season` defaults to the
most-recent season for the chosen league.

`--filter` accepts the same Phase Art Ross grammar as `query leaders`.
Bio atoms (`country`, `pos`, `age`, `draft-*`) work as expected — the
`age` atom anchors on the cohort year, so `age<=18` on a 2014-15
cohort uses each player's age as of Feb-1-2015. Stat atoms (`g>=10`,
sliding-window, career-aggregate) evaluate against the player's NHL
career, not their non-NHL league stats — useful for "OHL leaders who
later hit 30 NHL goals" but not for narrowing on the OHL stat line
itself (use `--sort` for that).

Run `icelines fetch career --bundled-seasons 5` once (≈100 s, hits the
NHL landing endpoint for ~1,650 players) before this command; the
data isn't bundled into the binary.

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
icelines fetch career --bundled-seasons 5   # multi-league career history (Calder)
                                            # ~100s; populates ~/.icelines/career_history.json
                                            # for ~1,650 players. Lights up the pre-NHL section
                                            # on player cards + `query career` leaderboards.

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
| `?` | Help overlay (keybind cheatsheet) |
| `M` (Shift+m) | **Manual / docs overlay** — full COMMANDS.md content scrollable inside the TUI. Lowercase `m` is reserved for the Goalies min-GP cycle. (LP.4) |
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
| `w` | Toggle selected Poach candidate in the local `Watchlist` group |
| `s` / `l` | Save / load query (Queries tab) |
| `f` | **Free-form filter overlay** (Queries tab) — type any Phase Art Ross filter (`country IN (CAN, USA) AND age<25`, `g.last10g>=5`, `p.career>=500`, etc.); Enter to apply, Esc to cancel. Inside the overlay: `↑/↓` walk recent-filter history, `?` toggle grammar cheatsheet, live "→ N of M match" count for bio + season-stat filters |

The Reports overlay (`R`) persists toggles to `~/.icelines/config.toml`. Disabled reports drop their columns from career tables, sort pickers, and query results.

### TUI surfaces — per-experience launchers (Phase Lady Byng)

Boot directly on a specific surface instead of launching League and pressing a digit.

```bash
# Nav-tab launchers
icelines tui league                  # default — 32-team rankings
icelines tui depth                   # cross-team depth chart
icelines tui stats                   # interactive query builder
icelines tui goalies                 # goalie leaderboard
icelines tui poach                   # fantasy poacher board
icelines tui watchlist               # local fantasy poacher watchlist group
icelines tui scores                  # tonight's games + boxscores
icelines tui schedule                # weekly + season schedule
icelines tui transactions            # league-wide moves feed
icelines tui playoffs                # bracket + series detail

# Drill-down launchers — open a specific card cold
icelines tui player Bedard           # name (substring match)
icelines tui player 8478402          # explicit pid (bypasses name lookup)
icelines tui team EDM                # team depth chart
icelines tui goalie "Connor Hellebuyck"
icelines tui comps McDavid           # similarity comps screen

# Phase Masterton.3 — focused single-screen mode (standalone)
icelines tui goalies --standalone        # locked to Goalies, no tab cycling
icelines tui scores --standalone         # focused live-scores TUI
icelines tui transactions --standalone   # focused transactions feed
# When --standalone is set: Tab/Shift+Tab no-op, tab strip is hidden, the
# screen's chrome (header title + footer keybinds) is the only navigation
# UI. Overlays (?, F, y, R) and per-screen keybinds work as usual.

# Phase Jack Adams — multi-pane MDI dashboard
icelines tui --mdi                       # Scores ribbon top + Favorites left
                                          # + swappable Workspace middle +
                                          # Schedule right + cmdbar bottom
icelines tui --mdi goalies               # MDI launching with goalies workspace
# In MDI mode: press `:` or `/` to focus the cmdbar; type a verb (e.g.
# `stats`, `goalies`, `team EDM`, `query g >= 30`, `/fav add Bedard`,
# `/hide schedule`); Enter to submit. `?` shows the full command
# reference. Ctrl+H toggles Favorites pane, Ctrl+L toggles Schedule pane.
# Mutually exclusive with --standalone.

# Equivalent flag form (for scripts)
icelines tui --start goalies
icelines tui --start poach
icelines tui --start watchlist
icelines tui --start "player:Bedard"
icelines tui --start "team:EDM"
```

### MDI dashboard cmdbar reference (Phase Jack Adams)

When `--mdi` is set, the TUI gains a chat-CLI command bar at the bottom.
Press `:` to focus the bar with empty input, or `/` to focus with `/`
already typed (for slash commands). Enter submits; Esc cancels.

| Verb | Effect | Example |
|---|---|---|
| `stats` / `goalies` / `poach` / `watchlist` / `transactions` / `playoffs` / `depth` / `scores` / `schedule` / `favorites` | Swap workspace | `:poach` |
| `player <name>` | Open player card | `:player Bedard` |
| `team <ABBR>` | Team depth chart | `:team EDM` |
| `team <ABBR> season` | Team's full schedule | `:team EDM season` |
| `compare <a>` / `compare <a> <b>` | Similarity peers / head-to-head | `:compare McDavid` |
| `box <game-id>` | Boxscore detail | `:box 2025020001` |
| `query <filter>` | Apply Phase Art Ross filter, swap to Stats | `:query g >= 30 AND age <= 25` |
| `/fav add <name>` | Add to Favorites | `/fav add Bedard` |
| `/fav remove <name>` | Remove from Favorites | `/fav remove Bedard` |
| `/hide favorites` / `/hide schedule` | Hide a side pane | `/hide schedule` |
| `/show favorites` / `/show schedule` | Restore a side pane | `/show schedule` |
| `/help` (alias `/h`, `/?`) | Full command reference overlay | `/help` |
| `/quit` (alias `q`, `quit`) | Exit | `:q` |

Global hotkeys (work without entering the bar): `q` quits, `?` opens
help overlay, `Ctrl+H` toggles Favorites pane, `Ctrl+L` toggles
Schedule pane.

Adaptive layout — the dashboard auto-drops side panes on narrow
terminals: ≥160 cols full / 120-159 drops Schedule / 100-119 drops
Favorites too / <100 falls back to single-document SDI render for
that frame.

### MDI cmdbar AI fallback (Phase Jack Adams.6 / .7, v0.23.1+)

Off by default. When enabled, an input that the deterministic parser
rejects (`show me young scorers`) is delegated to a configured LLM
provider for natural-language → command interpretation. The returned
string is re-parsed through `parse_command` exactly like user input,
so AI output is never trusted blindly.

Configure in `~/.icelines/config.toml`:

```toml
[ai]
enabled = true                      # default false
provider = "claude-cli"             # or "anthropic-api"
model = "claude-haiku-4-5"
timeout_secs = 15
```

**Provider: `claude-cli`** — shells out to `claude -p "<prompt>"`.
Requires Claude Code installed and authenticated locally. No API key
in icelines config.

**Provider: `anthropic-api`** — direct HTTP POST to the Anthropic
Messages API. Reads `$ANTHROPIC_API_KEY` from the environment at
launch time. Faster startup than the subprocess path.

Behavior:

- Parser rejects → bar shows ` ! asking claude-cli… (Esc to cancel) `
- Provider succeeds → response goes through `parse_command`; cmdbar
  applies it like any other input. History gets an `ai:` prefix.
- Provider fails (timeout / parse error / unsupported) → flash carries
  the original parser error PLUS the provider's diagnostic; input is
  preserved so you can edit manually.
- Esc at any time during the ask aborts the in-flight request.

The system prompt that providers receive is hand-written and
versioned (`crate::ai::SYSTEM_PROMPT_VERSION`). It documents the full
cmdbar grammar plus the Phase Art Ross filter syntax (sliding-window
atoms, EVER queries, cross-league career filters). Models that can't
express a request in the grammar are told to return the literal token
`UNSUPPORTED`, which surfaces as a clear flash rather than a fake
command.

**Slug aliases** (case-insensitive, accepted on `--start` only — sugar subcommands stick to canonical names):

| Canonical | Aliases |
|-----------|---------|
| `stats` | `queries` |
| `scores` | `tonight` |
| `transactions` | `moves` |

**Drill-down ambiguity**: `tui player Smith` lists every Smith candidate (pid + team + season + role) so you can re-run with the unambiguous pid. No silent picks.

**Resolution failures** (unknown slug, unknown player, ambiguous match, bad team abbrev) print to normal stderr and exit non-zero — the alt-screen never opens, so error messages don't get eaten.

### `icelines menu` — looping launcher

Friendly entry point for users who don't want to memorize subcommand names.

```bash
icelines menu
```

Prints a numbered menu, reads a choice, dispatches to the matching surface, then re-prints the menu when the surface quits. `Q` is the only way out.

```
  1-8   Nav-tab surfaces
  P/T/G/C   Drill-downs (sub-prompts for name/abbrev)
  W   Web dashboard (port 8000)
  D   Print COMMANDS.md
  Q   Quit
```

`icelines menu < /dev/null` (non-TTY) exits 0 with a redirect message — for scripted use, call `icelines tui --start <slug>` directly. Ctrl-C inside the prompt currently exits 130 (Unix) / 1 (Windows); a clean-exit `ctrlc` handler is a follow-up.

---

## Live-data commands (Phase Lester Patrick — full CLI parity)

```bash
# Tonight's games
icelines tonight
icelines tonight --team EDM

# Upcoming schedule (LP.1 — gained --json/--csv + comfy-table)
icelines schedule                       # next 7 days
icelines schedule --team EDM --days 14
icelines schedule --json > games.json   # scripted export
icelines schedule --csv > games.csv     # Excel-friendly

# Playoff bracket (LP.2 — new)
icelines playoffs                       # most recent completed bracket
icelines playoffs --season 19931994     # 1993-94 — NYR ended 54-yr drought
icelines playoffs --round 4             # only the Cup Final
icelines playoffs --json                # JSON for scripting

# League-wide transactions feed (Phase Selke; Lester Patrick verified parity)
icelines transactions                   # current season, default 7-day window
icelines transactions --team EDM
icelines transactions --kind trade --since 2026-01-01
icelines transactions --player McDavid
icelines transactions --json > moves.json
```

The schedule, playoffs, and transactions commands all share the same data sources as the TUI tabs (Schedule / Playoffs / Transactions) and the web routes (`/schedule`, `/playoffs`, `/transactions`) — guaranteed-consistent across all three surfaces per the IceLines.md "Feature × surface portfolio" doctrine.

---

## Other commands

```bash

# Player groups (SQLite-backed watchlists)
icelines group create "Watchlist"
icelines group add "Watchlist" "McDavid"
icelines group show "Watchlist"
# Poach `w` also stores the current score/explanation as a watch reason.
icelines watch list
icelines watch note "Matthew Knies" "PP1 promotion and strong hits fit"
icelines watch rules
icelines watch player "Matthew Knies" --when pp1 --save
icelines watch deployment --team TOR --line-change --save
icelines watch disable player-matthew-knies
icelines watch enable player-matthew-knies
icelines watch fire player-matthew-knies --player "Matthew Knies" "PP1 usage crossed threshold"
icelines watch history

# Personal-attendance tracker
icelines games add ...                 # log games attended
icelines games list

# Snapshots (data versioning)
icelines snapshot list
icelines snapshot verify

# Trade simulator (depth-chart impact, not fantasy)
icelines trade "Bouchard" for "Fox" --team EDM

# Web dashboard (Phase King Clancy King.1.5)
icelines serve                         # boot localhost:8000, auto-open browser
icelines serve --port 9000             # custom port
icelines serve --no-open               # print URL, don't auto-open
icelines serve --bind 0.0.0.0          # LAN-accessible (warning prints)

# Selke fantasy poacher web/API surfaces:
# /poach, /reports/poach, /reports/weekly, /watchlist, /api/v1/poach,
# /api/v1/watchlist, /api/v1/watch-rules

# Removed 2026-05-04 — the mkdocs static-site frontend (`build`,
# `site`, `deploy`, `--site-dir`, `/site/*` mount) is gone. The new
# web dashboard at `icelines serve` is the single web frontend.
# The `icelines-site` crate still exists for markdown generation
# but has no CLI entry point.
```

---

## Phase Foster — favorites, time-travel, sync

### `icelines favorites` — your favorited players + teams

```bash
# Today's favorites view (text table)
icelines favorites

# Time-travel — past or future date
icelines favorites --date 2014-10-08
icelines favorites --date 2026-01-15 --range week
icelines favorites --date 2026-01-15 --range month

# Read from a different group
icelines favorites --group "Watchlist"

# JSON envelope (heterogeneous data per WIRE B1: players + teams + events)
icelines favorites --json
```

The empty-state card surfaces the two `group add` examples for new
users. Per-night stat lines + boxscore-driven content land
incrementally as Foster.3+ wires the data orchestration; the surface
itself is in place today.

### `icelines setup` — first-run wizard

```bash
# Interactive — three questions (transactions / boxscores / sync policy)
icelines setup

# Headless / scripted: write the spec defaults non-interactively
icelines setup --accept-defaults

# Preview without touching ~/.icelines/config.toml
icelines setup --accept-defaults --dry-run

# Re-run even if config.toml exists
icelines setup --reset
```

Top-level `--no-setup` flag skips the auto-prompt for callers that
expect to run headless.

### `icelines config` — sync + capability matrix

```bash
icelines config get sync.policy
icelines config get sync.capabilities.transactions
icelines config set sync.capabilities.transactions league
icelines config list
icelines config reset sync.capabilities
```

Capability matrix (6 capabilities × 3 modes = `off | favorites | league`):

| Key                                  | Default     | Notes |
|--------------------------------------|-------------|-------|
| `sync.capabilities.stats`            | `league`    | Base — always on |
| `sync.capabilities.scores_schedule`  | `league`    | Default ON for everyone |
| `sync.capabilities.transactions`     | `favorites` | Opt-in to `league` |
| `sync.capabilities.boxscores`        | `favorites` | Deeper stats for favorites |
| `sync.capabilities.shifts`           | `off`       | **Locked** — only `off` valid (per-shift parsing not implemented) |
| `sync.capabilities.career_history`   | `favorites` | Adds slowly |

`sync.policy` ∈ `eager | lazy | off`; `sync.banner` ∈ `summary |
silent | verbose`; `sync.season_transition` ∈ `prompt | auto |
ignore`.

### `icelines fetch boxscore` — score events for a date

```bash
# Today's slate — write a score event per game to the EventStream
icelines fetch boxscore

# Specific date
icelines fetch boxscore --date 2026-01-15

# Only games involving favorited teams
icelines fetch boxscore --for-favorites

# Preview
icelines fetch boxscore --date 2026-01-15 --dry-run
```

### `icelines fetch sync` — refresh stale entries

```bash
# Walk the manifest, refresh anything past TTL
icelines fetch sync

# List what would be refreshed without fetching
icelines fetch sync --dry-run

# Override Static TTL (DataInstall pin) — re-fetch everything
icelines fetch sync --force
```

Non-blocking variant runs automatically on TUI launch when
`sync.policy = eager`. Set `ICELINES_TEST_MODE=1` to suppress the
spawn entirely (intended for L3 golden tests / CI determinism).

### `icelines data-status` — inspect the on-disk manifest

```bash
# Pretty-print every shard, its files, and freshness
icelines data-status

# Filter to one DataKind (case-insensitive)
icelines data-status --shard boxscore

# Show only entries `fetch sync` would refresh
icelines data-status --stale-only
```

Recognized `--shard` values: `bios`, `stats`, `goalie_stats`,
`transactions`, `boxscore`, `career_history`, `schedule`, `score`,
`playoff_bracket`. Source labels: Bundle / Setup / Live /
DataInstall / Manual.

### Date axis on existing commands

```bash
# tonight + schedule accept --date YYYY-MM-DD (verified ≥2014)
icelines tonight --date 2014-10-08
icelines schedule --date 2014-10-08

# Deprecated alias (will be removed in v0.15)
icelines schedule --start 2014-10-08
```

### Windowed filter atoms

```bash
# Bare atom (no .window) — defaults to season totals (existing behavior)
icelines query leaders --filter "g>=10"

# Explicit window — always-week / always-month / always-season
icelines query leaders --filter "g.week>=10"
icelines query leaders --filter "p.month>=20"
icelines query leaders --filter "g.season>=50"
```

`query career --week` / `--month` is intentionally rejected (junior
seasons aren't aligned with NHL week boundaries) — use `--season`.

### TUI keybinds added in Foster

- `Shift+D` — open the date picker overlay on Tonight (jumps to a
  past date) / Schedule (snaps to that week's Monday) / Playoffs
  (opens the season picker since playoffs is season-anchored).

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
