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
pwsh scripts/test-slice.ps1 ci-audit         # cargo-audit vulnerability gate
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

Prince visual-system focused gates:

```powershell
cargo test -p icelines-cli prince_tui
cargo test -p icelines-cli --test prince_cli_visual
cargo test -p icelines-web l1_static_css_contains_prince_route_layout_classes
```

Release smoke:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1 -SkipBuild
```

Full release checklist: `design/release-checklist.md`.
`ci-audit` installs `cargo-audit --locked` when missing; RustSec vulnerability
advisories block, while warning-class advisories remain visible in the release
checklist ledger.

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
The newest 5 bundled seasons carry modern Tier-1 depth; older rows are
historical/skeleton season totals. Missing modern fields render unavailable,
not zero.

VTRACE baseline note: `docs/vtrace/` is the authoritative mission, requirement,
design, verification, validation, and work-package baseline. This command
reference is an operator guide and must not claim a feature beyond that baseline
or `design/specs/surface-parity.md`.

FLETCH integration gates are migration/dependency-seam commands, not normal
quick-start steps:

```bash
icelines fetch fletch-sources --gate
icelines fetch fletch-partitions --gate
icelines fetch fletch-quivers --gate
```

`fletch-sources` inventories source-byte acquisition. `fletch-partitions`
projects ICELINES query surfaces into FLETCH partition/rollup IDs while keeping
activation on ICELINES sealed snapshots and active pointers. `fletch-quivers`
groups those partition rows into query bootstrap and enrichment bundle
candidates without exporting bytes or activating data. VTRACE `WP-007` keeps the
standalone/no-cross-repo-dependency target open until these seams are removed or
explicitly replaced, refused, or rolled back.

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
When the rendered leaders window or career arc extends beyond the newest 5
bundled seasons, the CLI prints a data-depth disclosure: older seasons are
historical/skeleton season totals and missing modern fields render unavailable,
not zero.

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
- Advanced: `xg`, `xg-per-60`, `cf-pct`, `ff-pct`, `on-ice-xg-for`,
  `on-ice-xg-against`, `xgf-pct` (requires `fetch money-puck`)
- Trend: `improvement` — Y/Y PPG delta vs prior season

**Flag-based filters**: `--pos`, `--team`, `--age-min`/`--age-max`, `--nationality`, `--draft-year`, `--draft-round`, `--draft-pick-max`, `--undrafted`, `--rookie`, `--handedness`, `--gp-min`/`--gp-max`, `--toi-min`, `--ppg-min`, `--plus-minus-min`, `--shots-pg-min`, `--birth-province`.

**Output**: `--json` / `--csv` to a stream, or `--out PATH` to a file.

### `query player NAME` — player profile

Career arc, percentile rank, full stats. Searches both skater AND goalie bios. Historical players resolve without `--season` via cross-bundled name lookup.
When multiple career seasons are shown, the text career arc includes a compact
oldest-to-newest Pts/82 and G/82 sparkline. Windows beyond the newest five
modern bundled seasons still print the data-depth disclosure.
Web `/player/:id` renders an inline Pts/82 bundled regular-season career trend
when the loaded player card has at least two bundled career rows; `/api/v1/player/:id`
keeps the tabular JSON contract unchanged.

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
Default text, JSON, and CSV output include the shared goalie workload/quality
fields `QS%`/`quality_start_pct` and `SA/60`/`shots_against_per_60` when goalie
advanced data is loaded. Web `/goalies` and `/api/v1/goalies` expose the same
fields for rows backed by goalie advanced data. GSAx is not surfaced until a
verified goalie xGA source exists.

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

Cohort scope: only players fetched by the career loader, usually the newest
5-season NHL roster cohort (`fetch career --bundled-seasons 5`). Career-only
players who never reached the NHL aren't in scope. `--season` defaults to the
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

TUI note: cross-league cohorts do not have a dedicated in-dashboard board.
Use the command bar handoff (`:career league=OHL season=20142015 top=8`)
to open the canonical CLI command or web `/career` target. Player cards still
show bundled NHL career arcs and local-store pre-NHL rows when available.

---

## Top-level analytics commands

```bash
icelines rank --top 10 [--pos C|LW|RW|D]    # league pace-score ranking
icelines rank --json
icelines team EDM                            # team depth chart with cross-team fit
icelines team-season EDM                     # season record, splits, form, remaining games
icelines team-season EDM --json              # shared TeamSeasonView JSON
icelines players --top 20                    # full PlayerFilter surface
icelines history "Connor McDavid"            # season-by-season log
icelines project "Celebrini" --mode pace     # rest-of-season projection
icelines scouting "Evan Bouchard"            # 8-section scouting report
icelines peers "Lane Hutson" --size 8        # draft-class statistical peers
icelines class 2022 --top 15                 # full draft class ranked
icelines compare "McDavid" "MacKinnon"       # alias for `query compare`
icelines mates "Beniers" --top 5             # roster fallback; shift bundles parked
```

`icelines project` preserves its legacy projected-points fields and also emits
`PlayerScoringPaceView`-backed pace outlook ranges for goals, points, and shots.
Text output prints a `Pace outlook ranges` line; JSON/CSV add `pace_outlook_*`
rows with projected finish and nullable low/high bands.

## `report` — report surface map and durable decision reports

Use `icelines report list` when you are not sure which command generates the
report you want. It lists the canonical report families, output formats, and the
screen/web surface that owns each one.

```bash
icelines report list
icelines report list --json
icelines report poach --category shots --top 10 --out poach.md
icelines report weekly --league default --category hits,blocks --out weekly.md
```

Surface rule of thumb:

| Need | Use |
|---|---|
| Ask a filter/query question | `icelines query ...` |
| Quick CSV/JSON for Excel/scripts | `icelines x <shape>` |
| Durable markdown packet | `icelines export md <shape>` |
| Fantasy decision report | `icelines report poach` / `icelines report weekly` |
| See every available/planned report family | `icelines report list` |

Web `/reports/poach` and `/reports/weekly` HTML append an inline SVG bar chart
for positive returned poach scores. The chart is descriptive report context;
`/api/v1/poach` remains the board JSON contract.

Records reports live under the canonical `records` surface and also appear in
`report list`: player/team symmetric facts such as NHL teams a player has scored
against, goalies scored against, fight opponents, and head-to-head counts.

## `awards` - player NHL Trophy Case

Official NHL awards and trophy seasons come from the NHL player landing
endpoint's `awards[]` array. They are not inferred from leaderboard finishes.

```bash
icelines awards "Connor McDavid"
icelines awards "Connor McDavid" --json
icelines awards "Connor McDavid" --csv --out mcdavid-awards.csv
```

Web routes: `/player/:id/awards` and `/api/v1/player/:id/awards`. In the TUI,
press `a` from a player card to open the cached Trophy Case; run the CLI command
once to populate `~/.icelines/player_awards.json`.

## `streaks` - player scoring and shot streaks

Goal/assist/point streaks are computed from cached per-game boxscore skater
rows. Shot-on-goal and shot-attempt streaks are computed from cached official
play-by-play rows, with loaded zero-attempt games breaking the streak. Populate
local inputs with `icelines fetch boxscore --date YYYY-MM-DD` and
`icelines fetch play-by-play --date YYYY-MM-DD`, then run:

```bash
icelines streaks "Connor McDavid"
icelines streaks "Connor McDavid" --json
icelines streaks "Connor McDavid" --csv --out mcdavid-streaks.csv
```

Web routes: `/player/:id/streaks` and `/api/v1/player/:id/streaks`. In the TUI,
press `s` from a player card, or run `:streaks player <name>` in the command bar.

Player cards are now the hub for player-specific surfaces: records, awards,
streaks, scouting, compare, groups/favorites, and fantasy watch handoffs are
linked from the TUI card, web `/player/:id`, or the command bar.

## `signals NAME` — descriptive derived metrics

IceLines Signals are descriptive derived metrics built from existing stat inputs
(Phase Hurricane / WP-010). They are **not** predictions, betting edges, injury
signals, deployment recommendations, or autonomous coaching decisions. Missing or
partial evidence renders as `unavailable` (text) / `null` (JSON) with an evidence
tier and the missing inputs — never as a `0.0` player value.

```bash
icelines signals "Connor McDavid"                      # text table
icelines signals "McDavid" --json                      # frozen signals.v1 envelope
icelines export md signals --player "McDavid" --out -  # markdown report packet
icelines signals "Wayne Gretzky" --season 19881989     # historical (skeleton) season
icelines signals "Cale Makar" --type playoff           # playoff window
```

Current signals (all per-60):

| Signal | Key | Polarity | Formula |
|---|---|---|---|
| Physical Engagement Rate | `physical-engagement-rate` | = neutral | `(hits + blocked shots)` per 60 |
| Puck Management Differential | `puck-management-differential` | ↑ higher better | `(takeaways − giveaways)` per 60 |
| Penalty Drag Rate | `penalty-drag-rate` | ↓ lower better | `penalty minutes` per 60 |

Legend: `↑` higher is better · `↓` lower is better · `=` neutral. Signals are
available through the CLI, the player-card TUI block, Web HTML
(`/player/:id/signals`), Web JSON (`/api/v1/player/:id/signals`), and
`export md signals --player <name>`. They are intentionally **not** in the
`--filter` catalog, leaderboards, `StatId`, or analytics cache. Phase Capitals
reviewed the promotion gate and kept those deferrals until accepted cache metric
keys, invalidation/source-state rules, and bounded catalog/leaderboard copy exist
(see
[`design/specs/icelines-signals.md`](design/specs/icelines-signals.md)).

### `signals-roster` — team-scoped Signals discovery

```bash
icelines signals-roster --team NYR
icelines signals-roster --team NYR --json
```

`signals-roster` renders a roster matrix over the existing `PlayerSignalsView`
rows for one team. It is an inspection aid, not a Signal leaderboard, `StatId`
promotion, filter key, analytics-cache metric family, prediction, betting edge,
injury signal, deployment recommendation, player-quality grade, or autonomous
coaching decision. Missing Signals render as `unavailable`, never as zero-filled
player values. JSON uses the additive `signals-roster.v1` envelope. Phase
Capitals keeps this surface uncached and team-scoped.

## `records` — player/team individual records

The first records slice uses persisted boxscore goal rows. Populate local
boxscores with `icelines fetch boxscore --date YYYY-MM-DD`, then run:

```bash
icelines records player "Andre Burakovsky" --metric teams-scored-against
icelines records player "Andre Burakovsky" --metric goalies-scored-against
icelines records player "Andre Burakovsky" --metric fight-opponents
icelines records player "Andre Burakovsky" --metric teams-scored-against --json
icelines records team EDM --metric players-scored-against-team --csv
icelines records team EDM --metric goalies-beaten-by-team
icelines records team EDM --metric fight-opponents-by-team
```

Available now: `teams-scored-against` for players and
`goalies-scored-against` and `fight-opponents` for players, plus
`players-scored-against-team`, `goalies-beaten-by-team`, and
`fight-opponents-by-team` for teams. `icelines fetch play-by-play --date
YYYY-MM-DD` installs the event participant source for goalie and fight metrics.
Fight records use explicit fighting-major participants, not aggregate PIM.
Default web records pages live at
`/records/player/:id`, `/records/team/:abbrev`,
`/api/v1/records/player/:id`, and `/api/v1/records/team/:abbrev`.
Web records HTML pages render an inline count SVG chart when record rows have
positive counts; the JSON records routes remain unchanged.
In the TUI, open a player card and press `r` to see the player records screen
with teams scored against, goalies scored against, and fight opponents. In the
MDI command bar, `:records player <name>` opens the same TUI records screen;
`:records team <ABBR>` still flashes the canonical CLI/web target.

## Web scoring reports

Rocket Richard scoring reports use cached official NHL play-by-play scoring
events: goals, shots on goal, missed shots, and blocked shots. Populate inputs
with `icelines fetch play-by-play --date YYYY-MM-DD` or use the web Admin game
cache loader's "Scoring events / play-by-play" artifact.

Web routes: `/game/:id/scoring`, `/team/:abbrev/scoring`,
`/player/:id/scoring`, `/team/:abbrev/outlook`, `/player/:id/outlook`,
`/tonight/intel`, `/api/v1/game/:id/scoring`, `/api/v1/team/:abbrev/scoring`,
`/api/v1/player/:id/scoring`, `/api/v1/team/:abbrev/outlook`,
`/api/v1/player/:id/outlook`, and `/api/v1/tonight/intel`.

Scoring outlook pages are descriptive pace surfaces, not betting forecasts.
Player outlook rows show goals, points, and shots with 82-game pace and nullable
projected finish/range below the sample floor or when remaining schedule data is
not loaded. The player range is a descriptive confidence band around current
pace, not a betting forecast. Team outlook rows show goals for/against pace and
recent pressure from cached regular-season schedule scores only; GET routes do
not fetch live NHL data.
Web `/player/:id/outlook` and `/team/:abbrev/outlook` render an inline 82-game
pace SVG chart when outlook rows have finite positive pace values; the JSON
routes are unchanged.

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
icelines export md roster --pos G --out goalies.md
icelines export md team-season --team EDM --out team-season-EDM.md
icelines export md leaders --columns "g,a,p,blk" --out custom.md
icelines export md leaders --season 20242025 --filter "country=CAN" --top 5 --out leaders-can.md
icelines export md compare --p1 McDavid --p2 MacKinnon --out compare.md
icelines export md signals --player McDavid --out mcdavid-signals.md
```

Shapes: `leaders`, `team`, `team-season`, `depth`, `fantasy`, `compare`, `signals`, `series`, `roster`.

`export md leaders` accepts the same repeatable free-form `--filter` strings as
`query leaders`, plus explicit `--season` and `--type regular|playoff` controls
for reproducible report evidence windows.
`export md leaders` includes an inline SVG bar chart for the top returned
skaters by current-window Pts/82 when the rendered result has finite positive
Pts/82 values.
Web `/leaders` renders the same descriptive current-window Pts/82 bar chart for
non-empty skater results; `/api/v1/leaders` is unchanged.
`export md team-season` includes an inline SVG quality-ledger bar chart when
quality ledger counters are positive. The chart is descriptive context over the
rendered quality ledger table.
`export md depth` includes an inline SVG team-strength bar chart when rendered
team totals are positive. The chart is descriptive context over the team-strength
table.
`export md fantasy` includes an inline SVG poach-score bar chart when report
candidates have positive scores. The chart is descriptive context over the
fantasy poacher report tables.
`export md roster` includes an inline SVG Pts/82 bar chart when rendered skater
rows have positive rates. The chart is descriptive context over the roster table.
`export md team` includes an inline SVG Pts/82 bar chart when rendered target-team
skater rows have positive rates. The chart is descriptive context over the team
roster table.
`export md series` includes an inline SVG game-margin bar chart when rendered
playoff games have nonzero goal margins. The chart is descriptive context over
the series game log.
`export md compare` includes an inline SVG Pts/82 career trend when both players
have at least two bundled career seasons; the chart is descriptive context over
bundled regular-season rows, not an era-adjusted player valuation.
`export md signals` renders the same `PlayerSignalsView` rows as the CLI/Web
Signals surfaces with disclosure, non-claim copy, methodology, limitations,
evidence tiers, and missing-input labels before the table; unavailable evidence
prints as `unavailable`, never a zero-filled signal value.
The Web `/compare?a=ID&b=ID` page renders the same bundled regular-season Pts/82
career trend after the side-by-side table when both compared players have enough
bundled career rows.
Web `/player/:id` renders the same descriptive single-player Pts/82 career trend
below the career table when the loaded player card has enough bundled rows.
Web `/team/:abbrev` renders an inline active-roster skater Pts/82 bar chart when
the team has finite positive skater rates; `/api/v1/team/:abbrev` is unchanged.
Web `/goalies` renders an inline SV% bar chart for returned goalies with finite
save percentages; `/api/v1/goalies` is unchanged.
Web scoring outlook pages render an inline 82-game pace bar chart for finite
positive outlook rows; their `/api/v1/.../outlook` routes are unchanged.
Web records pages render an inline count bar chart for positive record rows;
their `/api/v1/records/...` routes are unchanged.
TUI playoff series detail adds a compact game-margin sparkline when bundled
series game logs have nonzero margins; live/no-game playoff detail keeps the
existing played-count fallback.
TUI team-season detail adds a compact goal-differential sparkline for completed
non-tied schedule rows; upcoming/live rows stay out of the visual.
TUI schedule matchup detail adds a compact margin sparkline for completed
non-tied head-to-head rows; no-games and upcoming-only matchups remain textual.
TUI game detail adds compact skater-activity bars under each team's boxscore
leader block when skater game stats are loaded.
TUI player-records detail adds compact ASCII count bars beside each opponent
row while keeping the numeric count as the controlling value.
TUI goalie leaderboard rows add compact ASCII SV% quality bars beside the
printed save percentage while keeping SV% as the controlling value.
TUI Stats leaders rows add compact ASCII primary-metric bars beside the printed
leader metric while preserving the numeric/text metric as the controlling value.
TUI team roster rows add compact ASCII Pts/82 bars beside the printed rate while
keeping Pts/82 as the controlling value.

---

## Fantasy league

```bash
# Setup
icelines fantasy league-create "My League" --scheme yahoo-standard
icelines fantasy team-create "My Team" --owner "Gio"
icelines fantasy team-add "My Team" "McDavid"
icelines fantasy import-yahoo --file rosters.csv --league "My League" --dry-run
icelines fantasy import-yahoo --file rosters.csv --league "My League" --my-team "My Team"
icelines fantasy roster-shape
icelines fantasy roster-shape-set yahoo-standard --league "My League"
icelines fantasy roster-shape-validate --team "My Team" --json

# Manage
icelines fantasy team-show "My Team"
icelines fantasy standings
icelines fantasy daily --date 2026-01-15 --json
icelines fantasy matchup-set --week 2026-01-15 --home "My Team" --away "Rival"
icelines fantasy matchup --date 2026-01-15 --json
icelines fantasy league-list
icelines fantasy league-switch "My League"

# Trades
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski"
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --execute

# Web dashboard
icelines fantasy serve --port 8080
# GET /                    HTML standings
# GET /api/standings       JSON
# GET /api/team/<name>     team JSON
# POST /api/trade          simulation JSON

# Main web dashboard also exposes:
# GET /api/v1/fantasy/daily?date=YYYY-MM-DD   FantasyDailyDeltaView JSON
# GET /api/v1/fantasy/matchup?date=YYYY-MM-DD FantasyMatchupWeekView JSON
# GET /api/v1/fantasy/roster-shape?team=<name> RosterShapeValidationView JSON
```

`fantasy import-yahoo` accepts Yahoo roster CSV exports with a player column
(`Player`, `Name`, `Player Name`, or `First Name` + `Last Name`) and a fantasy
team column (`Fantasy Team`, `Team Name`, `Rostered By`, `Owner Team`, or
`Manager Team`). Optional `Owner`, `NHL Team`, and `Eligible Positions` columns
are diagnostic context only. Use `--dry-run` first to preview created/updated
teams, imported/skipped players, unresolved names, duplicate ownership, and
header problems; rerun without `--dry-run` to apply local FantasyDb membership.
Yahoo stats are ignored and never become player/stat/photo truth.

`fantasy roster-shape` lists the active league shape and available built-ins.
`fantasy roster-shape-set <shape>` persists the per-league setup rule, and
`fantasy roster-shape-validate [--team <name>] [--json]` validates persisted
FantasyDb rosters against canonical NHL/bundled player positions. Shape mutation
stays CLI-backed; TUI and web dashboard commands hand off or defer so GET
navigation never mutates roster state.

## `scheme` — fantasy scoring schemes

```bash
icelines scheme list
icelines scheme show yahoo-standard
icelines scheme from-csv path/to/yahoo.csv      # detect platform, build template
```

Built-in schemes: `yahoo-standard`, `espn-standard`, `simple-pts`.

---

## Data and history

```bash
# Fetch fresh data (optional — bundles work immediately)
icelines fetch all                    # rosters + stats (~5 min)
icelines fetch rosters                # roster source bytes via FLETCH, ICELINES snapshot seal
icelines fetch realtime               # hits, blocks, giveaways, takeaways
icelines fetch money-puck             # MoneyPuck CSV via FLETCH, ICELINES parses xG/CF/FF/xGF/xGA
icelines fetch money-puck --seasons 5 # current season plus 4 prior regular seasons
icelines fetch fletch-sources --gate  # source handoff inventory + migration gate
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
icelines data verify --all            # verify installed data manifests
```

The web Admin page also exposes scoped data install/remove forms. Web install
writes only embedded bundled seasons to `~/.icelines/seasons/<season>/bundle-<season>`
after exact `INSTALL <season>` confirmation; it does not fetch live source data.
Web remove deletes only `~/.icelines/seasons/<season>` after exact
`REMOVE <season>` confirmation.

The bundled-data cap is 38 seasons because `BUNDLED_SEASONS` is the canonical source. The 2004-05 lockout has no data and never will.

---

## TUI (`icelines tui` or `icelines dashboard`)

Interactive dashboard. By default `icelines tui` opens the shared composable
workbench: an activity/catalog rail, scores ribbon, swappable left/right context
panes, central workspace, bound experience presets, active field summaries, and a
command bar. Use `--classic` for the older tabbed single-document UI. Player
cards lazy-load every player's full historical career across all 38 bundled
seasons on first open. When the card trend extends beyond the newest 5 bundled
seasons, the TUI labels it as a bundled trend and shows a compact data-depth
line: newest 5 modern, older seasons skeleton, missing modern fields
unavailable.

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Default MDI: move focus across workbench rail / panes / workspace. `--classic`: cycle tabs forward / backward. `--standalone`: no-op. |
| `↑↓` | Navigate within the focused zone or screen |
| `←→` | Side pane focused: cycle pane composition. Workspace focused: screen-local navigation. |
| `Enter` | Rail focused: open selected workbench entry. Workspace focused: drill into selection (team / player / game). |
| `Esc` / `q` | Back / quit |
| `?` | Help overlay (keybind cheatsheet) |
| `M` (Shift+m) | **Manual / docs overlay** — full COMMANDS.md content scrollable inside the TUI. Lowercase `m` is reserved for the Goalies min-GP cycle. (LP.4) |
| `R` | **Reports overlay** — toggle Tier-1 reports loaded into columns |
| `y` | **Season picker** — jump to any of the 38 bundled seasons |
| `Shift+P` | Toggle Regular ↔ Playoff for the active season |
| `o` | Toggle the current section on Stats (Queries) |
| `[` / `]` | Cycle career-table column presets on a player card |
| `/` | Open the sort picker (search-as-you-type 108 stats) |
| `r` | Open player records from a player card; refresh current view elsewhere |
| `d` | Jump to depth chart (or jump-to-date on Scores) |
| `F` | Toggle admin overlay |
| `g` | Add to group from a player card / team roster |
| `w` | Toggle selected Poach candidate in the local `Watchlist` group |
| `s` / `l` | Save / load query on Queries; `s` opens player streaks from a player card |
| `f` | **Free-form filter overlay** (Queries tab) — type any Phase Art Ross filter (`country IN (CAN, USA) AND age<25`, `g.last10g>=5`, `p.career>=500`, etc.); Enter to apply, Esc to cancel. Inside the overlay: `↑/↓` walk recent-filter history, `?` toggle grammar cheatsheet, live "→ N of M match" count for bio + season-stat filters |

### TUI filter/sort matrix (Phase Messier)

Player-list screens use the same filter vocabulary. Keybinds mutate the
current screen state; in MDI mode, `f` pre-fills the command bar with the
matching verb so wider filters can be entered as `key=value` pairs.

| Screen | `s` | `p` | `n` | `h` | `m` | `f` / cmdbar |
|---|---|---|---|---|---|---|
| Team | Sort | Position | Nationality | Hits column | Min GP | `:team EDM sort=hits pos=F nationality=CAN` |
| Goalies | Sort | Role class | Nationality | Saves column | Min GP | `:goalies sort=gaa min-gp=20 nationality=CAN saves=on` |
| Stats | Save query | Query builder | `nationality=` shortcut | — | — | Free-form Art Ross filter overlay |
| Depth | Scoring mode | Position | Nationality | — | — | `:depth pos=F nationality=SWE` |
| Favorites | Sort | Position | Nationality | — | — | `:favorites sort=name min-gp=20` |

Supported roster KV keys are `sort`, `pos`/`position`,
`country`/`nationality`, `min-gp`, `hits`, and `saves`. Duplicate keys and
unknown values are rejected before any screen state is mutated.
Goalie starter/backup role is cycled with `p`; the current KV grammar does not
accept `pos=Starters`.

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

# Phase Jack Adams / Call the Changes — workbench MDI dashboard (default)
icelines tui                             # Activity rail + Scores ribbon top
                                          # + Favorites left + swappable
                                          # Workspace middle + Schedule right
                                          # + cmdbar bottom
icelines tui stats                       # dashboard launch with Scoring room preset
icelines tui goalies                     # dashboard launch with goalies workspace
icelines tui --mdi                       # explicit dashboard mode (same default)
icelines tui --classic                   # older tabbed single-document UI
# In MDI mode: Tab/Shift+Tab move focus between the activity rail, side
# panes, and center workspace. With the rail focused, ↑/↓ selects a shared
# catalog entry and Enter opens it when it has a no-argument TUI screen.
# With a side pane focused, ←/→ cycles the shared pane binding. TUI-safe
# bindings render either the native pane or a compact field/command summary,
# including web-derived player/game/record inspector panes.
# Press `:` or `/` to focus the cmdbar; type a verb (e.g. `stats`, `goalies`,
# `team EDM`, `query g >= 30`, `/fav add Bedard`, `/fav add EDM`, `/hide schedule`); Enter to
# submit. `?` shows the full command reference. Ctrl+H toggles Favorites pane,
# Ctrl+L toggles Schedule pane.
# --mdi is mutually exclusive with --standalone and --classic.

# Equivalent flag form (for scripts)
icelines tui --start goalies
icelines tui --start poach
icelines tui --start watchlist
icelines tui --start "player:Bedard"
icelines tui --start "team:EDM"
```

### MDI dashboard cmdbar reference (Phase Jack Adams)

The default TUI dashboard includes a chat-CLI command bar at the bottom.
Press `:` to focus the bar with empty input, or `/` to focus with `/`
already typed (for slash commands). Enter submits and keeps the bar ready
for the next command; pressing `:` again at the empty prompt is harmless.
Use `Tab` or `Esc` to leave command mode. Outside command mode, Tab moves
between workbench zones instead of cycling legacy tabs.

Bound MDI experiences are available from the activity rail for Tonight bench,
Scoring room, Team room, Fantasy room, and Admin room. Each preset swaps the
workspace plus left/right context panes together, using the same shared
workbench IDs as the web dashboard. Starting the dashboard on a bound workspace
(`icelines tui stats`, `icelines tui --start scores`, `icelines tui fantasy`)
applies the matching room before the first frame and positions the rail on that
workspace. Command-bar workspace swaps keep the rail selection aligned too.
The rail keeps the selected room visible on shorter terminals. Cycling a side
pane also exposes the shared inspector catalog; panes without native TUI bodies
render compact field and command summaries instead of dead placeholders. Admin,
Docs, and Groups workbench destinations use named chrome labels instead of the
generic screen fallback. Hiding the focused side pane moves focus back to the
central workspace so keyboard input never lands on an invisible pane.

| Verb | Effect | Example |
|---|---|---|
| `stats` / `goalies` / `poach` / `watchlist` / `transactions` / `playoffs` / `depth` / `scores` / `schedule` / `favorites` | Swap workspace | `:poach` |
| `goalies <kv...>` | Goalies filters/sort | `:goalies sort=gaa min-gp=20 nationality=CAN` |
| `depth <kv...>` | Depth filters | `:depth pos=F country=SWE` |
| `favorites <kv...>` | Favorites filters/sort | `:favorites sort=name min-gp=20` |
| `gaps <kv...>` / `fantasy gaps <kv...>` | Fantasy roster-gap filters | `:gaps cats=hits,blocks,shots top=8` |
| `poach <kv...>` / `fantasy poach <kv...>` | Fantasy poacher filters | `:poach rw cats=hits,blocks free top=12` |
| `simulate <kv...>` / `fantasy simulate <kv...>` | Fantasy add/drop scenario projection | `:simulate add=Connor_McDavid drop=Bench_Forward weeks=3` |
| `daily date=YYYY-MM-DD` / `fantasy daily date=YYYY-MM-DD` | Fantasy daily-delta read-surface handoff | `:fantasy daily date=2026-01-15` |
| `fantasy roster-shape show|validate ...` | Fantasy roster-shape CLI/API handoff | `:fantasy roster-shape validate team My_Team` |
| `fantasy roster-shape set <shape>` | Fantasy roster-shape setup CLI handoff | `:fantasy roster-shape set yahoo-standard` |
| `fantasy import file=... league ... [dry-run]` | Fantasy roster CSV import CLI handoff | `:fantasy import file=rosters.csv league My_League dry-run` |
| `simulate clear` | Clear the active fantasy simulation scenario | `:simulate clear` |
| `report poach` / `report weekly` | Show exact report CLI/web target | `:report weekly cats=shots,hits top=12` |
| `awards player <name>` | Open the cached TUI Trophy Case | `:awards player Connor McDavid` |
| `streaks player <name>` | Open the TUI player streaks screen | `:streaks player Connor McDavid` |
| `records player <name>` | Open the TUI player records screen | `:records player Andre Burakovsky` |
| `records team <ABBR>` | Show exact records CLI/web target | `:records team SEA` |
| `scouting player <name>` | Show exact scouting CLI/web target | `:scouting player Connor McDavid` |
| `mates player <name>` | Show exact linemate/deployment CLI target | `:mates player Connor McDavid` |
| `watch <player>` | Show exact watch-note/rule target | `:watch Connor McDavid` |
| `admin` | Open the operational admin overlay | `:admin` |
| `data ...` / `snapshot ...` / `config ...` | Show exact admin CLI/web target | `:data status`, `:snapshot list`, `:config list` |
| `roster` / `fantasy roster` | Open active fantasy roster-gap view | `:roster` |
| `player <name>` | Open player card | `:player Bedard` |
| `team <ABBR> <kv...>` | Team depth chart with optional filters | `:team EDM pos=LW country=CAN` |
| `team <ABBR> season` | Team season-performance view | `:team EDM season` |
| `team <ABBR> schedule` | Team's full schedule list | `:team EDM schedule` |
| `class <year>` | Apply draft-year query, swap to Queries | `:class 2024` |
| `career <kv...>` | Show exact Career cohort CLI/web target | `:career league=OHL season=20142015 top=8` |
| `compare <a>` / `compare <a> vs <b>` | Similarity peers / head-to-head handoff | `:compare McDavid`, `:compare McDavid vs Crosby` |
| `box <game-id>` / `box <AWAY@HOME>` | Boxscore detail from id or loaded slate | `:box 2025020001`, `:box EDM@BOS` |
| `query <filter>` | Apply Phase Art Ross filter, swap to Stats | `:query g >= 30 AND age <= 25` |
| `/fav add <name-or-team>` | Add to Favorites | `/fav add Bedard`, `/fav add EDM` |
| `/fav remove <name-or-team>` | Remove from Favorites | `/fav remove Bedard`, `/fav remove EDM` |
| `/hide favorites` / `/hide schedule` | Hide a side pane | `/hide schedule` |
| `/show favorites` / `/show schedule` | Restore a side pane | `/show schedule` |
| `/help` (alias `/h`, `/?`) | Full command reference overlay | `/help` |
| `/quit` (alias `q`, `quit`) | Exit | `:q` |

Global hotkeys (work without entering the bar): `q` quits, `?` opens
help overlay, `Tab`/`Shift+Tab` moves workbench focus, `←`/`→` cycles a focused
side pane, `Ctrl+H` toggles Favorites pane, `Ctrl+L` toggles Schedule pane.

### Web dashboard command contract (Phase Jack Adams Web)

Run `icelines serve --port 8000` and open `/dashboard` for the browser version
of the shared workbench: grouped activity/catalog rail, bound experience tabs,
scores ribbon, left/right pane selectors, central workspace, active
pane-model/field affordances, and a command bar. Canonical route pages still
work directly; the dashboard shell wraps them as workspace panels through
`/dashboard?workspace=<route>`.

Pane composition is read-only URL state and allowlisted by shared workbench IDs:

```text
/dashboard?workspace=/scores&left=favorites-left&right=schedule-right&experience=tonight-bench
/dashboard?workspace=/leaders&left=saved-queries-left&right=stat-filter-right
```

Bound experience tabs carry coherent workspace + pane presets. Left/right pane
selector chips swap context panes without mutating favorites, watch rules,
caches, snapshots, or config. Pane visibility remains local browser state.

The browser dashboard uses the same deterministic command vocabulary as the
TUI where the web route exists. Read commands resolve to internal workspace
URLs for `/dashboard?workspace=...`; write commands resolve to POST-backed
mutation intents rather than GET links.

Examples:

```text
stats                                      -> /leaders
goalies                                    -> /goalies
poach                                      -> /poach
roster                                     -> /fantasy
poach rw cats=hits,blocks free top=12      -> /poach?pos=RW&category=hits%2Cblocks&availability=available&top=12
gaps cats=hits,blocks top=8                -> /fantasy?category=hits%2Cblocks&top=8
fantasy poach top=8 availability=available -> /poach?top=8&availability=available
fantasy poach top=8 available              -> /poach?top=8&availability=available
simulate add=Connor_McDavid drop=Bench_Forward weeks=3
                                           -> /fantasy?add_player=Connor_McDavid&drop_player=Bench_Forward&weeks=3
fantasy simulate add Connor_McDavid drop Bench_Forward
                                           -> /fantasy?add_player=Connor_McDavid&drop_player=Bench_Forward
fantasy daily date=2026-01-15              -> /api/v1/fantasy/daily?date=2026-01-15
fantasy roster-shape validate team="My Team"
                                           -> /api/v1/fantasy/roster-shape?team=My+Team
fantasy roster-shape set yahoo-standard    -> deferred; use `icelines fantasy roster-shape-set`
fantasy import file=rosters.csv league=Office
                                           -> deferred; use `icelines fantasy import-yahoo --dry-run`
report weekly cats=shots,hits top=12       -> /reports/weekly?category=shots%2Chits&top=12
report poach availability=imported-available
                                           -> /reports/poach?availability=imported-available
records team EDM                           -> /records/team/EDM
records player 8478402                     -> /records/player/8478402
favorites group=Prospects                  -> /favorites?group=Prospects
group show Prospects                        -> /favorites?group=Prospects
group create Prospects                      -> not GET-backed; use `/favorites` POST forms or `icelines group`
team EDM                                   -> /team/EDM
team EDM season                            -> /team/EDM/season
team EDM schedule                          -> /schedule?team=EDM
career                                     -> /career?league=OHL&sort=points
class 2015                                 -> /career?season=2015&sort=points
career league=OHL season=20142015 top=8    -> /career?league=OHL&season=20142015&top=8
player Connor McDavid                      -> /leaders?filter=name%3DConnor+McDavid
compare Connor McDavid vs Sidney Crosby    -> /compare?left=Connor+McDavid&right=Sidney+Crosby
/fav add Connor McDavid                    -> POST /favorites/add
watch Connor McDavid                       -> POST /watch-rules/create
watch player Connor McDavid when=available -> save player watch rule
watch enable player-connor-mcdavid         -> toggle persisted watch rule on
watch disable player-connor-mcdavid        -> toggle persisted watch rule off
watch deployment TOR                       -> deferred; use CLI preview, not dashboard mutation
```

`/favorites` can select any SQLite group with `?group=<name>`. The page exposes
POST-backed create, rename, delete, and selected-group member add/remove forms;
dashboard command text still opens read views instead of turning group edits into
GET mutations.

Fantasy screen shortcuts prefill the same command bar grammar: `g` on Fantasy
Gaps starts `gaps `, `p` on Poach starts `poach `, `w` on Poach toggles the
selected player in the local Watchlist, and `a` on Fantasy Sim starts
`simulate add=`.

Browser adaptive layout: wide screens show the catalog rail + experience tabs +
scores + left context pane + Workspace + right context pane. Tablet/mobile keeps
Workspace primary, collapses Schedule/right context first when no user
preference exists, lets both side panes reopen via visible Show/Hide handles,
and keeps the command bar as a sticky reach target. Command history is
session-local; side-pane visibility is local browser state. Dashboard GET
navigation is read-only; favorite/watch writes continue to use POST-backed
endpoints.

Visual capture gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 web-captures
```

This starts `icelines serve --no-open`, uses installed Edge/Chrome headless, and
writes desktop/mobile dashboard screenshots under `dist/web-dashboard-captures/`.

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

# Team season performance (Presidents Trophy)
icelines team-season EDM                # record, home/away splits, form, remaining schedule
icelines team-season EDM --json         # shared TeamSeasonView JSON

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

# Browser group editor
/favorites                              # create groups and edit Favorites
/favorites?group=Watchlist              # rename/delete selected group; add/remove members
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

# Web admin operations are at /admin while the server is running.
# Safe POST-backed mutations: runtime web config set/reset, data verify,
# bundled data install, scoped data remove, inactive snapshot delete,
# sealed snapshot activate, and game-cache warmers.
# Runtime web config controls only the running server's active season context.
# Persistent report toggles are intentionally deferred on web admin; use
# `icelines tui` then press R to persist them to ~/.icelines/config.toml.
# Web game-cache forms may fetch official game rows for records/streaks/scoring,
# but they do not install release bundles or remove local data. Web data install
# writes only embedded bundled seasons after exact `INSTALL <season>`
# confirmation; web data remove deletes only ~/.icelines/seasons/<season> after
# exact `REMOVE <season>` confirmation.

# Selke fantasy poacher web/API surfaces:
# /poach, /reports/poach, /reports/weekly, /watchlist, /api/v1/poach,
# /api/v1/watchlist, /api/v1/watch-rules

# Removed 2026-05-04 — the mkdocs static-site frontend (`build`,
# `site`, `deploy`, `--site-dir`, `/site/*` mount) is gone. The new
# web dashboard at `icelines serve` is the single web frontend.
# The `icelines-site` crate still exists for markdown generation
# but has no CLI entry point.
```

The web/TUI watch-rule editors intentionally support player-rule create,
enable/disable, and web delete only. Arbitrary team/deployment editing remains a
CLI preview/save shortcut until the shared mutation intent carries validated
team/deployment dimensions.

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

### `icelines fetch play-by-play` — event participants and scoring events

```bash
# Today's slate — persist raw play-by-play JSON under the data manifest
icelines fetch play-by-play

# Specific date
icelines fetch play-by-play --date 2026-01-15

# Only games involving favorited teams
icelines fetch play-by-play --for-favorites

# Preview
icelines fetch play-by-play --date 2026-01-15 --dry-run
```

This is the source for event-backed records such as goalies a player has scored
against and fight opponents, and for Rocket Richard scoring reports built from
goal, shot-on-goal, missed-shot, and blocked-shot events. Empty-net goals remain
no-goalie rows; the records layer must not infer a goalie from the boxscore.

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
`transactions`, `boxscore`, `play_by_play`, `career_history`, `schedule`,
`score`, `playoff_bracket`. Source labels: Bundle / Setup / Live /
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
