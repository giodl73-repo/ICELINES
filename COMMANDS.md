# IceLines — Command Reference

Visible report headings use The Rink product language—such as **The Insider —
Morning Skate** and **The Crease — Who Gets the Net?**—without renaming the
stable commands documented below. Branded commands such as `icecast` are
additive rather than replacements; `icereplay` remains planned.

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
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-release-artifact.ps1 -ArtifactPath dist\release\icelines-windows-x86_64.zip
```

`package-release.ps1` writes `dist\release\icelines-windows-x86_64.zip` plus a
`.sha256` sidecar; `ICELINES-PACKAGE.txt` inside the archive records the binary
version, source commit, build timestamp, and binary SHA-256.
Tagged GitHub releases publish the same `.sha256` sidecar pattern for every
Linux, macOS, and Windows archive, and each release archive includes the same
`ICELINES-PACKAGE.txt` metadata file. `verify-release-artifact.ps1` verifies the
sidecar hash and required archive members for downloaded `.zip` or `.tar.gz`
artifacts.

Full release checklist: `design/release-checklist.md`.
`ci-audit` installs `cargo-audit --locked` when missing; RustSec vulnerability
advisories block, while warning-class advisories remain visible in the release
checklist ledger.

---

## Stathead starter packs

`icelines stathead` prints curated editorial/stathead query packs. These are
recipes over existing IceLines commands, not new metric semantics. JSON and
Markdown outputs include the source/data requirement and command-effect notes
for each recipe; `--commands` emits only runnable command lines, and
`--commands --read-only` omits file-writing recipes. Use
`--commands --writes-only` to inspect only recipes that write files.

```bash
icelines stathead                         # list available packs
icelines stathead young-stars             # show one pack
icelines stathead era-leaders --json      # machine-readable pack metadata
icelines stathead --markdown --out stathead-packs.md
icelines stathead goalie-notebook --markdown --out goalie-notebook.md
icelines stathead young-stars --commands  # runnable commands only
icelines stathead fantasy-prep --commands --read-only
icelines stathead --commands --writes-only
```

Current packs:

| Pack | Focus |
|---|---|
| `era-leaders` | Bundled-history and multi-season scoring leaderboards |
| `young-stars` | Age-gated scorer and category-filter starter queries |
| `playoff-runs` | Playoff scoring and series-context entry points |
| `goalie-notebook` | Goalie rate and workload notebook starters |
| `records-notebook` | Cached event-data player/team record starters |
| `fantasy-prep` | Fantasy poach and weekly-prep starter recipes |
| `draft-scouting` | Draft class, peer cohort, and scouting-report starters |

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

The current lean/dependency-seam posture is auditable without promoting lean
support:

```powershell
pwsh scripts/rangers-lean-audit.ps1
pwsh scripts/rangers-lean-audit.ps1 -Json
```

The audit is expected to report `target-not-met` while `fletch-core`,
`slice-core`, the FLETCH command surfaces, the SLICE selector surface, and the
missing `cli` feature remain present.

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
verified goalie xGA source exists. The Web/API goalie surface now includes a
`goalie_xga_source` gate that names the blocked GSAx metric family and records
that QS%/SA/60 and skater on-ice xGA are not goalie xGA substitutes.
The reserved catalog keys `goalie-xg-against`, `goalie-xg-against-per-60`,
`goals-saved-above-expected`, and `gsax-per-60` intentionally return no values
until that gate is satisfied.
The same Web/API metadata exposes a `goalie_high_danger_source` gate for
high-danger SV% candidates. It blocks high-danger shots against, saves, and
save percentage until a verified goalie danger source exists; raw SV%, SA/60,
and skater on-ice xGA are not high-danger goalie substitutes.

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
icelines mates "Beniers" --top 5             # roster fallback; shifts locked off
```

`icelines mates` reads a legacy precomputed `ShiftProfile` only if one is
present. Otherwise it prints the `sync.capabilities.shifts=off` policy and
renders same-team forward roster fallback rows; there is no supported
`fetch shifts` recovery today.

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
icelines report team-ceiling
icelines report team-ceiling --team NYR
icelines report team-ceiling --json --out team-ceiling.json
icelines report team-lineup --team NYR
icelines report team-lineup --team SEA --json --out sea-lineup.json
icelines report team-card --team NYR --scenario-id nyr-development-variance
icelines report team-card --team SEA --scenario-id sea-development-variance --json --out sea-card.json
icelines report team-card --team NYR --scenario-id nyr-development-variance --scenario-comparison-key development-variance --trials 1000 --seed 20262027 --generated-at 2026-07-22T12:00:00Z --json
pwsh -NoProfile -File scripts/validate-card-document.ps1 -Path examples/team-prognosis-card-nyr-2026-27.json -Summary
pwsh -NoProfile -File scripts/render-card-document.ps1 -Path examples/team-prognosis-card-nyr-2026-27.json -OutDir dist/cards
pwsh -NoProfile -File scripts/render-card-document.ps1 -Path examples/team-prognosis-card-nyr-2026-27.json -OutDir dist/cards -ResolveAssets -Pdf
pwsh -NoProfile -File scripts/test-card-reference-renderer.ps1
icelines report cap-forecast --team NYR
icelines report cap-forecast --years 5 --growth-pct 5 --json --out cap-forecast.json
icelines report poach --category shots --top 10 --out poach.md
icelines report weekly --league default --category hits,blocks --out weekly.md
```

Surface rule of thumb:

| Need | Use |
|---|---|
| Ask a filter/query question | `icelines query ...` |
| Quick CSV/JSON for Excel/scripts | `icelines x <shape>` |
| Durable markdown packet | `icelines export md <shape>` |
| 2026-27 roster ceiling / prior-year delta | `icelines report team-ceiling` |
| Four lines, pairs, goalies, faces, and IceLines scores | `icelines report team-lineup --team NYR` |
| Two-page lineup and prognosis source document | `icelines report team-card --team NYR --scenario-id nyr-development-variance` |
| Five-year roster market cost / cap pressure | `icelines report cap-forecast` |
| Fantasy decision report | `icelines report poach` / `icelines report weekly` |
| See every available/planned report family | `icelines report list` |

`report team-ceiling` emits `team_ceiling.v1`. Current NHL roster membership
is rated from the completed 2025-26 NHL sample through four mechanisms:
points pace, goal scoring, fantasy/peripherals, and age-adjusted upside. Each
team uses the best 12 forwards, 6 defensemen, and 2 goalies for each lens.
Current and prior totals share the same 0-100 normalization, producing a true
within-report delta rather than comparing two unrelated scales. Missing-sample
prospects remain visible and reduce coverage instead of receiving a fabricated
zero. The playoff range is a transparent logistic roster-strength scenario
widened for missing coverage; it is not a trained forecast or betting line.

`report cap-forecast` emits the `cap_projection.v1` scenario contract. It uses
the official $104M 2026-27 upper limit, the announced $113.5M 2027-28
projection, and configurable growth after that. Team totals select a
deterministic 23-player scenario (14 forwards, 7 defensemen, 2 goalies) from
current-roster authority. Confirmed imported contracts remain distinct from
modeled low/mid/high role-market values. Pressure labels describe the cost of
retaining that current roster at modeled market rates; they are not committed
payroll or a prediction of trades, bonuses, term, clauses, or roster turnover.

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
Player streak JSON `meta.source_authorities` separates boxscore authority for
goal/assist/point streaks from play-by-play authority for shot and attempt
streaks; the web page prints the same authority labels.
Team streak JSON uses the same split for team streak leaders, with boxscore
authority for goal/assist/point leaders and play-by-play authority for shot and
attempt leaders.

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
available through the CLI, the player-card TUI block, Web player HTML
(`/player/:id/signals`), Web player JSON (`/api/v1/player/:id/signals`),
team-scoped Web roster HTML/JSON (`/team/:abbrev/signals` and
`/api/v1/team/:abbrev/signals`), and `export md signals --player <name>`.
They are intentionally **not** in the
`--filter` catalog, leaderboards, `StatId`, or analytics cache. Phase Capitals
reviewed the promotion gate and kept those deferrals until accepted cache metric
keys, invalidation/source-state rules, and bounded catalog/leaderboard copy exist
(see
[`design/specs/icelines-signals.md`](design/specs/icelines-signals.md)).
Single-player Signals JSON includes `meta.source_authority`; the TUI player-card
Signals block now prints compact source, coverage, covered-metric, and
blocked-claim authority details; Web HTML renders a Source authority section
with the same source, coverage state, covered inputs, covered metrics, blocked
claims, and limitations; Markdown export renders the same authority label and
details. The authority names covered inputs (season summary, realtime when
loaded, ice time when loaded, and minimum games), covered metrics, blocked
claims, and limitations while preserving unavailable values as missing evidence
instead of zero.

### `signals-roster` — team-scoped Signals discovery

```bash
icelines signals-roster --team NYR
icelines signals-roster --team NYR --evidence partial
icelines signals-roster --team EDM --evidence full --json
icelines signals-roster --team NYR --json
# Web twins:
/team/NYR/signals
/api/v1/team/NYR/signals?evidence=partial
```

`signals-roster` and `/team/:abbrev/signals` render the shared
`SignalsRosterView` matrix over the existing `PlayerSignalsView` rows for one
team. It is an inspection aid, not a Signal leaderboard, `StatId` promotion,
filter key, analytics-cache metric family, prediction, betting edge, injury
signal, deployment recommendation, player-quality grade, or autonomous coaching
decision. Missing Signals render as `unavailable`, never as zero-filled player
values. `--evidence all|full|partial|missing` and `?evidence=...` narrow the
team-scoped inspection rows by Signal evidence coverage while preserving
player-name sorting and the non-promotion boundary. Text, CLI JSON, Web HTML,
and Web JSON output report matched, total, and filtered-out row counts so filter
scope is auditable without implying rank. CLI JSON uses the additive
`signals-roster.v1` envelope; Web JSON uses the route envelope
`team-signals-roster`. Phase Capitals keeps this surface uncached and
team-scoped.
Web HTML also exposes explicit `all|full|partial|missing` filter handoff links
plus JSON twins for the same team-scoped roster; those links narrow inspection
only and do not rank players or promote Signals to cache, `StatId`, or
leaderboard surfaces.
`signals-roster.v1` also carries `meta.source_authority` and row-level
`source_authority` values copied from `PlayerSignalsView`, so the roster matrix
has the same covered inputs, blocked claims, and missing-evidence semantics as
single-player Signals.

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
Situation splits render event-owner strength labels such as
`even strength 5v5 (1551)`, `power play 5v4 (1541)`, and
`penalty kill 4v5 (1451)`, preserving the raw NHL `situationCode` for
auditability while making the first strength-state read surface human-readable.
The same reports also include aggregate By strength rows for even-strength,
power-play, and penalty-kill totals across raw NHL situation codes.
JSON scoring split rows also expose structured `situation_code`, `skater_state`,
and `owner_strength_state` fields for downstream consumers.
Scoring JSON `meta.source_authority` and the HTML report banner identify the
authority as the cached official NHL play-by-play source, with complete,
partial, stale, or unavailable state carried separately from the computed
scoring totals. `source_authority.coverage_state` gives consumers a compact
covered, partial, stale, or unavailable authority status.
`source_authority.covered_metrics` lists the exact scoring
metric family covered by that authority: goals, shots on goal, attempts,
unblocked attempts, missed shots, blocked shots, shot percentage, and strength
state. `source_authority.limitations` lists major non-covered domains,
including shift time, expected goals, live fetch status, and uncached games.
Tonight Intel uses the same source authority contract because it is built from
the same cached play-by-play scoring-event inputs.

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
not fetch live NHL data. Outlook JSON `meta.schedule_authority` and the HTML
banner identify the cached NHL schedule/final-score authority for remaining
games, projected-finish context, team goals for/against, and recent form.
Player outlook JSON also exposes `meta.season_stat_authority` for the loaded
season skater totals that drive games played, goals, points, shots, shot
percentage, per-game rates, and 82-game pace.
Web `/player/:id/outlook` and `/team/:abbrev/outlook` render an inline 82-game
pace SVG chart when outlook rows have finite positive pace values; the JSON
routes are unchanged.

## Analytics cache evidence routes

WP-009 analytics-cache routes read prepared cache records and render evidence
envelopes for selected future hockey decision surfaces. They do not compute live
analytics, fetch missing data, or claim prediction, betting, injury, line
chemistry causality, or autonomous coaching authority. Missing cache records
render an explicit unavailable state instead of synthesizing results.
Unavailable HTML pages now render the same Non-claims list as their JSON twins,
and unavailable JSON responses include a `non_claims[]` array that repeats the
route boundary for operators: no live analytics are computed, no prediction /
betting / injury / deployment / linemate meaning is inferred, and missing cache
records are not created or fetched.

Selected HTML / JSON twins:

```bash
/reports/analytics-cache?cache_key=coach_dashboard:20252026:regular&metrics=expected_goals_share
/api/v1/reports/analytics-cache?cache_key=coach_dashboard:20252026:regular&metrics=expected_goals_share
/coach/dashboard
/api/v1/coach/dashboard
/scout/opponent
/api/v1/scout/opponent
/player/evidence-card
/api/v1/player/evidence-card
```

Coach dashboard JSON is a prepared-cache evidence handoff only. It does not
issue coaching recommendations, deployment decisions, live analytics,
predictions, or cache fetches; its `consumer_boundary` field repeats that
contract for machine readers.

Opponent scout JSON follows the same prepared-cache boundary. It does not issue
scouting recommendations, line-matchup decisions, live analytics, predictions,
or cache fetches; its `consumer_boundary` field repeats that contract for
machine readers.

Player evidence card JSON is also prepared-cache evidence only. It does not
issue player grades, roster recommendations, deployment decisions, live
analytics, predictions, or cache fetches; its `consumer_boundary` field repeats
that contract for machine readers.

Line combination explorer JSON is prepared-cache evidence only. It does not
infer line chemistry, issue deployment recommendations, compute live analytics,
make predictions, or fetch cache records; its `consumer_boundary` field repeats
that contract for machine readers.

Goalie readiness JSON is prepared-cache evidence only. It does not issue
readiness recommendations, workload decisions, live analytics, predictions, or
cache fetches; its `consumer_boundary` field repeats that contract for machine
readers.

Practice focus JSON is prepared-cache evidence only. It does not issue practice
plans, coaching recommendations, deployment decisions, live analytics,
predictions, or cache fetches; its `consumer_boundary` field repeats that
contract for machine readers.

Postgame JSON is prepared-cache evidence only. It does not issue postgame
conclusions, adjustment plans, blame assignments, live analytics, predictions,
or cache fetches; its `consumer_boundary` field repeats that contract for
machine readers.

Agent evidence JSON is prepared-cache evidence only. It does not execute
recommendations, take autonomous actions, call agents, compute live analytics,
make predictions, or fetch cache records; its `consumer_boundary` field repeats
that contract for machine readers.

Additional selected cache-backed evidence routes include `/lines/explorer`,
`/goalies/readiness`, `/practice/focus`, `/postgame/review`,
`/postgame/adjustments`, and `/agents/evidence`, each with a matching
`/api/v1/...` JSON route. These routes are WP-009 evidence surfaces only; broader
practice, postgame, agent, and downstream product workflows remain partial until
`VAL-011` accepts product-copy and consumer evidence for that surface.

### Evidence map operator quick paths

The Web evidence maps are inspection-first navigation aids. They do not fetch
live data, make predictions, issue deployment advice, create cache claims, or
take autonomous coaching actions.

| Need | Web path | Focused gate |
|---|---|---|
| Home evidence map | `/` | `cargo test -p icelines-web --test l1_router l1_get_root_returns_200_html --quiet` |
| Team evidence map | `/team/EDM` | `cargo test -p icelines-web --test l1_router l1_team_html_includes_skater_pts82_svg_chart --quiet` |
| Player evidence map | `/player/8478402` | `cargo test -p icelines-web --test l1_router l1_player_html_links_signals_surface --quiet` |
| Evidence-map route inventory | n/a | `cargo test -p icelines-web --test ted_lindsay_route_inventory --quiet` |

The route-inventory gate keeps every evidence-map handoff backed by a mounted
route entry and by `design/specs/surface-parity.md` before the links become
operator-facing shortcuts.

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
non-empty skater results. Web `/leaders` and `/api/v1/leaders` also expose
MoneyPuck source authority for optional skater xG snapshots: covered metrics are
individual xG, ixG/60, on-ice xGF/xGA, xGF%, CF%, and FF%; blocked related
claims include goalie xGA/GSAx, goalie high-danger SV%, skater high-danger
chance %, zone entries, and deployment recommendations.
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
source-authority label/source/coverage/blocked-claim details, evidence tiers,
and missing-input labels before the table; unavailable evidence prints as
`unavailable`, never a zero-filled signal value.
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

## IceCast — The Goal Line

```powershell
icelines icecast camp --input examples/icecast-nyr-training-camp.json --json --out nyr-camp.json --lineup-set-out nyr-camp-lineups.json --blender-set-out nyr-camp-blenders.json --season-scenario-out nyr-camp-season.json --max-lineup-branches 5 --season-max-roster-branches 3000
icelines icecast camp-league --candidate-overlay examples/icecast-league-candidate-overlay-2026-27.json --authored-input examples/icecast-nyr-training-camp.json --authored-input examples/icecast-sea-training-camp.json --trials 1000 --json --out league-camp.json
icelines icecast bubble --input league-camp.json --top 10 --json --out league-bubble.json
icelines icecast bubble --input league-camp.json --transaction-context transaction-context.json --top 10 --json --out league-bubble.json
icelines icecast bubble --input examples/icecast-league-training-camp-2026-27.json --transaction-context examples/icecast-transaction-context-nyr-sea-2026-27.json --top 10 --json --out league-bubble-sourced.json
icelines icecast affiliate --input affiliate-scenario.json --json --out affiliate-lines.json
icelines icecast affiliate-identities --snapshot ahl-roster-stats.json --team "Hartford Wolf Pack" --candidates examples/icecast-league-candidate-overlay-2026-27.json --json --out hartford-identity-review.json
icelines icecast affiliate-identities --snapshot prior-ahl.json --team "Hartford Wolf Pack" --discover-official --json --out hartford-official-identity-review.json
icelines icecast affiliate-identities-league --snapshot ahl-season.json --discover-official --json --out ahl-league-identity-crosswalk.json
icelines icecast affiliate-review-draft --crosswalk hartford-official-identity-review.json --out hartford-review-decisions-draft.json
icelines icecast affiliate-review-draft-league --league-crosswalk ahl-league-exact-alias-reviewed.json --include-conflicts --out ahl-league-exception-drafts.json
icelines icecast affiliate-review-exact --crosswalk hartford-official-identity-review.json --reviewer "identity-pilot" --reviewed-at 2026-07-25T12:00:00Z --decisions-out hartford-exact-decisions.json --json --out hartford-exact-reviewed.json
icelines icecast affiliate-review-exact-league --league-crosswalk ahl-league-identity-crosswalk.json --reviewer "league-identity-pilot" --reviewed-at 2026-07-25T12:30:00Z --decisions-out ahl-league-exact-decisions.json --json --out ahl-league-exact-reviewed.json
icelines icecast affiliate-review-aliases --crosswalk hartford-exact-reviewed.json --reviewer "alias-pilot" --reviewed-at 2026-07-25T13:00:00Z --decisions-out hartford-alias-decisions.json --json --out hartford-alias-reviewed.json
icelines icecast affiliate-review-aliases-league --league-crosswalk ahl-league-exact-reviewed.json --reviewer "league-alias-pilot" --reviewed-at 2026-07-25T13:30:00Z --decisions-out ahl-league-alias-decisions.json --json --out ahl-league-alias-reviewed.json
icelines icecast affiliate-review-conflicts-league --league-crosswalk ahl-league-alias-reviewed.json --nhl-player-id 8482739 --evidence-url https://theahl.com/stats/player/9166 --evidence-url https://www.nhl.com/flyers/news/flyers-acquire-brett-harrison-jackson-edward-from-boston-in-exchange-for-alexis-gendron-massimo-rizzo --reviewer "league-conflict-pilot" --reviewed-at 2026-07-26T20:10:00Z --note "official NHL club transaction evidence controls the canonical NHL birth date while the AHL provider date remains retained" --decisions-out ahl-league-conflict-decisions.json --json --out ahl-league-conflict-reviewed.json
icelines icecast affiliate-review-birth-date-league --league-crosswalk ahl-league-conflict-reviewed.json --nhl-player-id 8484115 --canonical-birth-date 1999-04-17 --evidence-url https://www.iowawild.com/players/detail/zmolek-1 --evidence-url https://bsubeavers.com/sports/mens-ice-hockey/roster/will-zmolek/15025 --reviewer "league-date-pilot" --reviewed-at 2026-07-26T22:50:00Z --note "official AHL club and college records agree with the provider date" --decisions-out ahl-league-date-decisions.json --json --out ahl-league-date-reviewed.json
icelines icecast affiliate-review-collision-league --league-crosswalk ahl-league-conflict-reviewed.json --proposed-nhl-player-id 8475366 --canonical-nhl-player-id 8484302 --canonical-name "Matt Brown" --canonical-birth-date 1999-08-09 --evidence-url https://api-web.nhle.com/v1/player/8484302/landing --evidence-url https://www.phantomshockey.com/wp-content/uploads/2023/10/2023-Phantoms-Training-Camp-Roster.pdf --reviewer "league-collision-pilot" --reviewed-at 2026-07-26T21:10:00Z --note "official records identify the younger same-name player" --decisions-out ahl-league-collision-decisions.json --json --out ahl-league-collision-reviewed.json
icelines icecast affiliate-review-reject --crosswalk hartford-alias-reviewed.json --provider-player-id 8789 --evidence-url https://www.hartfordwolfpack.com/players/detail/ortiz --reviewer "exception-pilot" --reviewed-at 2026-07-25T14:00:00Z --note "official club evidence identifies an AHL-only player without a canonical NHL identity" --decisions-out hartford-reject-decisions.json --json --out hartford-exception-reviewed.json
icelines icecast affiliate-review-reject-league --league-crosswalk ahl-league-conflict-reviewed.json --provider-player-id 8789 --evidence-url https://www.hartfordwolfpack.com/players/detail/ortiz --reviewer "league-exception-pilot" --reviewed-at 2026-07-28T23:00:00Z --note "AHL player retained; no unique canonical NHL mapping" --decisions-out ahl-league-reject-decisions.json --json --out ahl-league-fully-reviewed.json
icelines icecast affiliate-review-league --crosswalk hartford-exception-reviewed.json --crosswalk coachella-reviewed.json --json --out ahl-league-identity-review.json
icelines icecast affiliate-review-league --league-crosswalk ahl-2023-reviewed.json --league-crosswalk ahl-2024-reviewed.json --league-crosswalk ahl-2025-reviewed.json --json --out ahl-three-season-identity-review.json
icelines icecast affiliate-review-board --review ahl-three-season-identity-review.json --json --out ahl-identity-exception-board.json
icelines icecast affiliate-review-draft --crosswalk hartford-official-identity-review.json --include-aliases --out hartford-review-with-aliases-draft.json
icelines icecast affiliate-review-draft --crosswalk hartford-official-identity-review.json --include-aliases --include-conflicts --out hartford-complete-proposals-draft.json
icelines icecast affiliate-review-show --crosswalk hartford-official-identity-review.json
icelines icecast affiliate-review-show --crosswalk hartford-official-identity-review.json --attention-only
icelines icecast affiliate-review-apply --crosswalk hartford-official-identity-review.json --decisions hartford-review-decisions.json --json --out hartford-reviewed-identities.json
icelines icecast affiliate-status-draft --prior-snapshot prior-ahl.json --crosswalk hartford-reviewed-identities.json --camp camp.json --nhl-team NYR --ahl-team "Hartford Wolf Pack" --out hartford-status-review-draft.json
icelines icecast affiliate-status-show --review hartford-status-review-draft.json
icelines icecast affiliate-status-apply --prior-snapshot prior-ahl.json --crosswalk hartford-reviewed-identities.json --camp camp.json --review hartford-status-review.json --config rollover-base.json --out rollover-config.json
icelines icecast affiliate-input --snapshot ahl-roster-stats.json --crosswalk hartford-identity-reviewed.json --facts hartford-projection-facts.json --nhl-team NYR --ahl-team "Hartford Wolf Pack" --out hartford-affiliate-input.json
icelines icecast affiliate-rollover --prior-snapshot prior-ahl.json --crosswalk prior-identities.json --camp camp.json --camp-forecast camp-forecast.json --config rollover-config.json --json --out rollover.json
icelines icecast affiliate-rollover-config-league --league-crosswalk ahl-league-fully-reviewed.json --camp-forecast league-camp.json --prior-affiliations examples/ahl-affiliations-2025-26.json --affiliations examples/ahl-affiliations-2026-27.json --as-of 2026-07-28 --source-url https://theahl.com/mediaguide --source-url https://theahl.com/nhl-affiliations --out league-rollover-config.json
icelines icecast affiliate-status-draft-league --prior-snapshot prior-ahl.json --league-crosswalk ahl-league-fully-reviewed.json --camp-forecast league-camp.json --config league-rollover-config.json --json --out league-status-review.json
icelines icecast affiliate-status-apply-league --prior-snapshot prior-ahl.json --league-crosswalk ahl-league-fully-reviewed.json --camp-forecast league-camp.json --review league-status-review-final.json --config league-rollover-config.json --out league-rollover-reviewed.json
icelines fetch career --league-crosswalk ahl-league-fully-reviewed.json
icelines icecast affiliate-professional-games --league-crosswalk ahl-league-fully-reviewed.json --career-history ~/.icelines/career_history.json --policy examples/ahl-professional-game-policy-2026-27.json --json --out professional-games.json
icelines icecast affiliate-professional-games-apply --crosswalk hartford-identity-reviewed.json --ledger professional-games-final.json --facts hartford-projection-facts.json --nhl-team NYR --ahl-team "Hartford Wolf Pack" --out hartford-projection-facts-reviewed.json
icelines icecast affiliate-rollover-league --prior-snapshot prior-ahl.json --league-crosswalk ahl-league-fully-reviewed.json --camp-forecast league-camp.json --config league-rollover-config.json --json --out league-rollover.json
icelines icecast affiliate-map --json --out ahl-affiliations.json
icelines icecast prospect-study --input examples/icecast-jagger-firkus-prospect-study.json
icelines icecast prospect-study --input examples/icecast-jagger-firkus-prospect-study.json --json --out firkus-study.json
icelines icecast prospect-context --snapshot ahl-2023-24.json --snapshot ahl-2024-25.json --snapshot ahl-2025-26.json --league-crosswalk reviewed-league-2023-24.json --league-crosswalk reviewed-league-2024-25.json --league-crosswalk reviewed-league-2025-26.json --affiliations ahl-affiliations-2025-26.json --as-of 2026-09-15 --max-age 24 --json --out prospect-context.json
icelines icecast prospect-league --snapshot ahl-2024-25.json --snapshot ahl-2025-26.json --crosswalk reviewed-2024-cv.json --crosswalk reviewed-2025-cv.json --context examples/icecast-prospect-league-context.json --json --out league-discovery.json
icelines icecast prospect-league --snapshot ahl-2023-24.json --snapshot ahl-2024-25.json --snapshot ahl-2025-26.json --crosswalk reviewed-league-2023-24.json --crosswalk reviewed-league-2024-25.json --crosswalk reviewed-league-2025-26.json --context prospect-context.json --json --out league-discovery.json
icelines icecast prospect-program --league-discovery league-discovery.json --json --out prospect-programs.json
icelines icecast prospect-program --league-discovery league-discovery.json --study college-prospect-study.json --prior-board prior-prospect-programs.json --out prospect-programs.txt
icelines icecast prospect-program-sensitivity --league-discovery league-discovery.json --thresholds 25,50,82 --json --out prospect-program-sensitivity.json
icelines icecast prospect-program-history --board prospect-programs-2024.json --board prospect-programs-2025.json --board prospect-programs-2026.json --json --out prospect-program-history.json
icelines icecast prospect-conversion --league-discovery frozen-2022-23-prospects.json --career-history ~/.icelines/career_history.json --baseline-season 20222023 --through-season 20252026 --performance-out nhl-performance.json --json --out prospect-conversion.json
icelines icecast prospect-conversion --league-discovery frozen-2022-23-prospects.json --career-history ~/.icelines/career_history.json --baseline-season 20222023 --through-season 20252026 --performance nhl-performance.json --json --out replayed-conversion.json
icelines icecast prospect-board --study firkus-study.json
icelines icecast prospect-board --study firkus-study.json --study another-study.json --json --out prospect-board.json
icelines icecast organization --input organization.json --json --out the-system.json
icelines icecast season --team NYR --scenario nyr-camp-season.json --trials 10000 --json --out nyr-camp-season-forecast.json
icelines icecast season --team NYR --all-games --game-forecast-out nyr-games.json
icelines icecast bench --forecast nyr-games.json --lineup examples/team-lineup-nyr-2026-27.json --profile nyr-decision-profile.json --style-evidence opponent-styles.json --scenario-out nyr-game-plans.json --json --out nyr-bench-schedule.json
icelines icecast blender --lineup examples/team-lineup-nyr-2026-27.json --scenario-out nyr-bench.json
icelines icecast blender --lineup examples/team-lineup-nyr-2026-27.json --review-games 6 --minimum-points-percentage 0.50 --max-changes 2 --max-choices 3 --json --out nyr-lines.json --scenario-out nyr-bench.json
icelines icecast blender --lineup examples/team-lineup-nyr-2026-27.json --shift-season 20252026 --shift-report-out nyr-shifts.json --json --out nyr-lines.json
icelines icecast blender --lineup examples/team-lineup-nyr-2026-27.json --shift-season 20252026 --allow-off-wing --json --out nyr-lines.json
icelines icecast season --team NYR --scenario nyr-bench.json --trials 10000 --json --out nyr-adaptive-lines.json
icelines icecast season                                # NYR + SEA summary by default
icelines icecast season --team NYR --team SEA --all-games
icelines icecast season --team NYR --trials 25000 --seed 20262027
icelines icecast season --scenario examples/icecast-scenario.json
icelines icecast scenario import --id nyr-development-variance --path examples/icecast-nyr-development-variance.json --season 20262027 --evidence estimated
icelines icecast scenario list
icelines icecast scenario show nyr-development-variance
icelines icecast season --scenario-id nyr-development-variance --team NYR
icelines icecast season --scenario-id nyr-development-variance --team NYR --isolated-impacts --json
icelines icecast season --auto-personnel --trials 10000
icelines icecast season --trade-mode plausible --trials 10000
icelines icecast season --team SEA --json
icelines icecast season --team NYR --json --out nyr-2026-27.json
icelines icecast season-card --input nyr-2026-27.json --team NYR --team-name "New York Rangers" --out nyr-season-card.json
icelines icecast season --refresh                      # refresh official schedule cache
icelines icecast season --season 20252026 --replay-mode rolling --all-games
icelines icecast season --season 20242025 --replay-mode rolling --through 2025-01-31 --trials 1000 --json
icelines icecast season --season 20242025 --replay-mode rolling --through 2025-01-31 --scenario historical-counterfactual.json --isolated-impacts --json
icelines icecast movement --earlier january.json --later february.json --team NYR --team SEA
icelines icecast movement --earlier january.json --later february.json --json --out movement.json
icelines icecast movement-card --input movement.json --team NYR --team-name "New York Rangers" --out nyr-movement-card.json
icelines icecast history --input january.json --input february.json --input march.json --team NYR --team SEA
icelines icecast history --input january.json --input february.json --json --out history.json
icelines icecast history-card --input history.json --team NYR --team-name "New York Rangers" --out nyr-history-card.json
icelines icecast window-build --season 20262027 --as-of 2026-07-27 --generated-at 2026-07-27T20:00:00-07:00 --prospect-program prospect-program.json --out window.json
icelines icecast season --season 20262027 --game-forecast-out games.json --json --out season.json
icelines icecast window-source-package --season 20262027 --as-of 2026-10-01 --team-season-forecast season.json --team-game-forecast games.json --cache-team-lineups --stats-season 20252026 --ahl-affiliate hartford.json --training-camp camp.json --cache-prospect-program --out window-sources.json
icelines icecast window-source-audit --input window-sources.json --generated-at 2026-10-01T12:00:00Z --out window-source-coverage.json
icelines icecast window-build --season 20262027 --as-of 2026-10-01 --generated-at 2026-10-01T12:00:00Z --source-package window-sources.json --require-ranked --out production-window.json
icelines icecast window --input window.json
icelines icecast window --input window.json --team NYR
icelines icecast window --input window.json --markdown --out window-report.md
icelines icecast window --input window.json --team NYR --markdown --out nyr-window-report.md
icelines icecast window-card --input window.json --team NYR --team-name "New York Rangers" --out nyr-window-card.json
icelines icecast window-movement --earlier october.json --later january.json --out window-movement.json
icelines icecast window-personnel-attribution --earlier october.json --later january.json --movement window-movement.json --input personnel-attribution.json --out attributed-movement.json
icelines icecast window-personnel-input-build --actual-forecast actual-february.json --counterfactual-board counterfactual-february-window.json --earlier-as-of 2025-01-31 --later-as-of 2025-02-28 --attribution-id january-february --scenario-id paired-replay --rationale "Paired rolling replay" --out personnel-attribution.json
icelines icecast window-personnel-summary --input attributed-movement.json --out personnel-evidence-summary.json
icelines icecast window-rebase --input october.json --target-manifest balanced-v2.json --bridge balanced-v1-to-v2-bridge.json --out october-rebased.json
icelines icecast window-movement --earlier october.json --later january-v2.json --bridge balanced-v1-to-v2-bridge.json --out bridged-movement.json
icelines icecast window-history --input october.json --input january.json --input march.json --out window-history.json
icelines icecast window-scenario --baseline baseline.json --scenario trade.json --scenario-id deadline-addition --out window-impact.json
icelines icecast window-scenario --baseline baseline.json --scenario trade.json --scenario-id deadline-addition --authority trade-authority.json --out attributed-window-impact.json
icelines icecast window-scenario --baseline baseline.json --scenario modeled.json --scenario-id sourced-scenario --team-season-authority season-scenario.json --training-camp-authority camp-scenario.json --out sourced-window-impact.json
icelines icecast window-scenario-distribute --baseline baseline.json --input scenario-distribution-input.json --out scenario-distribution.json
icelines icecast window-calibrate --target next-season-organization-value --origin 2023-origin.json --origin 2024-origin.json --origin 2025-origin.json --minimum-origins 3 --out rolling-calibration.json
icelines icecast window-evaluate --target next-season-organization-value --origin 2022-train.json --origin 2023-train.json --origin 2024-validation.json --origin 2025-retrospective-holdout.json --minimum-training-origins 2 --out split-evaluation.json
icelines icecast window-standings --target-season 20252026 --date 2026-04-17 --captured-at 2026-07-28T08:00:00Z --out standings-2025-26.json
icelines icecast window-origin-build --source-season 20242025 --target-season 20252026 --as-of 2025-06-30 --generated-at 2026-07-28T08:00:00Z --role retrospective_holdout --standings standings-2025-26.json --out origin-2025-26.json
powershell -ExecutionPolicy Bypass -File scripts/window-browser-review.ps1
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-release-artifact.ps1 -ArtifactPath dist/release/icelines-windows-x86_64.zip
icelines icecast backtest --input 2021-22.json --input 2022-23.json --input 2023-24.json
icelines icecast backtest --input 2021.json --input 2022.json --input 2023.json --json --out validation.json
icelines icecast calibrate-development --start-season 20052006 --end-season 20252026
icelines icecast calibrate-development --json --out development-calibration.json
icelines icecast import-opening-rosters --manifest opening-rosters-2024.json --dry-run
icelines icecast import-opening-rosters --manifest opening-rosters-2024.json
icelines icecast discover-opening-rosters --season 20242025 --out coverage.json --manifest-out import.json
icelines icecast discover-opening-rosters --season 20242025 --partial-manifest-out partial.json
icelines icecast discover-opening-rosters --season 20242025 --cache-only --partial-manifest-out partial.json
icelines icecast import-opening-rosters --manifest partial.json --allow-partial-evaluation
icelines icecast season --season 20212022 --stats-season 20202021 --replay-mode rolling --retrospective-opening-lineups
```

`icecast camp` selects the opening active roster before the dressed 12F/6D/2G
lineup. Text and JSON distinguish active-roster, dressed, healthy-scratch, and
waiver-exposure probabilities. Salary-cap enforcement is fail-closed when
configured; without complete sourced cap hits, the forecast reports a structured
`no_read` instead of treating missing salaries as zero.
Player make, dress, scratch, waiver, and displacement probabilities are
conditioned on valid constrained trials. Rejected cap or roster trials remain
visible through `incomplete_trials` and are not miscounted as player cuts.
`icecast camp-league` applies that same contract to every franchise. Authored
team inputs override the automatic concept pool; automatic pools use current
roster identities, merge optional explicitly sourced organizational candidates,
add explicitly labeled prior-season organizational fallback candidates toward
17F/9D/3G, and retain per-team authority warnings. Candidate overlays require a
checked date, unique NHL player IDs, valid positions, and absolute evidence URLs.
Opening-
roster authority is separate from competition-pool construction, so an optional
fallback invite does not imply the 23-man roster itself lacks authority.

`icecast bubble` converts that league camp forecast into the UI-neutral
`training_camp_exposure_board.v1`. It ranks each team's available-but-not-
selected pressure, healthy-scratch pressure, and disclosed prospect
displacement. Injury/unavailability is isolated from selection loss and cannot
create waiver exposure. Without a sourced transaction overlay, material
pressure is labeled `roster_decision_review`. `transaction_review` requires
sourced waiver status and trade-protection context and is still research, not
a transaction prediction: market demand and waiver-claim probability remain
unknown.

The optional `training_camp_transaction_context.v1` document is season-scoped
and keyed by NHL player ID. Each row can carry cap hit, expiry year/type,
trade protection, and `requires_waivers`, plus at least one absolute source
URL. Duplicate or unknown player IDs, season/schema mismatches, empty source
lists, relative URLs, label mismatches, and zero cap hits fail closed. A sourced
no-move clause produces `contract_protected`; it cannot fall through to an
ordinary transaction or waiver lane.

`icecast affiliate` builds `ahl_affiliate_projection.v1` for the associated
AHL club. It selects 12 forwards, six defensemen, and two goaltenders, then
emits four forward lines, three defense pairs, and the goalie tandem. Output
carries explicit roster-pool authority. Official snapshot adapters set
`official_snapshot`; sourced camp/prior-season pools use
`preseason_projection`; authored what-if pools use `authored_scenario`; older
inputs without the field remain `unspecified`/no-read. Preseason authority
requires a date, absolute sources, and a methodology note. The season-scoped
rule authority defaults to the AHL's official development rule:
at least 12 of 18 dressed skaters must have 260 or fewer professional
regular-season games as measured at the start of the season. Missing
professional-game totals fail closed; age, NHL waiver status, and AHL rookie
status are not substitutes for the development-rule calculation.

`icecast affiliate-identities` compares one official AHL roster with either an
`ahl_canonical_identity_catalog.v1` or the existing sourced league camp
candidate overlay. It emits `ahl_identity_crosswalk.v1`. Exact normalized-name
and birth-date matches remain `pending`; they are proposals, never automatic
approval. Ambiguity, missing candidates, and birth-date conflicts remain
structured review states, and provider-local AHL IDs are never copied into NHL
identity fields.

`--discover-official` expands each provider roster name through the official
NHL player-search service, retains exact normalized-name results, and
corroborates their player ID, display name, and birth date through the official
player landing endpoint. When exact-name search is empty, it also searches the
surname and retains only a unique surname-and-birth-date proposal as the
distinct `surname_and_birth_date` basis. Alias proposals remain outside the
automatic exact-match decision draft and require an explicit sourced remap.
The identity bridge treats hyphens as word boundaries and ignores apostrophes
and periods without changing the global player-search normalizer.
This comparison-only rule leaves established official query and FLETCH cache
keys unchanged. Curly-apostrophe names also receive a straight-apostrophe
search variant because the official search index can distinguish those forms.
Both source shapes are cached through FLETCH and can be merged with
`--candidates`. Discovery improves the review queue but never changes
`review_status`; even exact name-and-birth matches remain pending until
explicitly reviewed. `--refresh` forces both official discovery layers to be
reacquired; without it, league discovery reads verified cachelines first and
fetches only missing search or landing objects. Bounded batches commit manifest
checkpoints throughout league acquisition so interrupted runs are resumable.

`icecast affiliate-identities-league` applies the same discovery and evidence
rules to every team in a sealed season snapshot and emits
`ahl_identity_league_crosswalk.v1`. Search requests are deduplicated by
normalized roster name across clubs before cached acquisition, and landing
records are fetched once per canonical NHL ID. The envelope retains one
snapshot-bound child crosswalk per team, total roster appearances, and unique
AHL provider-player coverage. It proposes identities only; no row is approved.
Compatible repeated NHL candidates returned by distinct name searches merge by
NHL ID with all evidence retained; identity conflicts fail closed.

`icecast affiliate-review-league` accepts repeated team `--crosswalk` inputs,
repeated `--league-crosswalk` envelopes, or both. League envelopes are flattened
without changing their child team queues, enabling one multi-season coverage
and recurring-exception report without manual extraction.

`icecast affiliate-review-draft-league` creates one non-applicable decision
envelope across a league crosswalk. `--include-conflicts` drafts retained
birth-date-conflict proposals for human inspection; `--include-aliases` can add
still-pending surname remaps. Empty team batches are skipped and every pending
unmatched or ambiguous row is counted in `pending_without_proposal`. The command
never changes review state or supplies reviewer authority.

`icecast affiliate-review-draft` emits a separate
`ahl_identity_review_decisions.v1` document containing `accept_proposal`
entries only for pending exact-name-and-birth-date proposals. It is deliberately
written with `draft: true` and no reviewer/timestamp, so it cannot be applied.
A reviewer must inspect every retained source, remove decisions they do not
accept, set `draft: false`, and add their name plus an RFC3339 timestamp.
`--include-aliases` additionally copies fully sourced `surname_and_birth_date`
rows into the draft as explicit `set_identity` remaps. It never includes
conflicts or unmatched rows, and the resulting document remains non-applicable
until a reviewer inspects, edits, and finalizes it.
`--include-conflicts` adds exact-name birth-conflict rows as explicit
`accept_proposal` decisions whose notes preserve both dates. It remains opt-in,
never covers unmatched rows, and is inspection-only: final application rejects
birth conflicts unless they are converted to explicit sourced `set_identity`
decisions through the targeted conflict workflow.

`icecast affiliate-review-aliases` is the applicable counterpart for sourced
surname-and-equal-birth-date aliases. It revalidates the name distinction,
shared surname and date, canonical ID, and absolute evidence before recording
an explicit `set_identity` remap.
`icecast affiliate-review-conflicts-league` selects proposed NHL IDs but touches
only pending `birth_date_conflict` rows. It requires additional absolute
evidence, reviewer, timestamp, and rationale; emits explicit `set_identity`
decisions; unions retained and new evidence; and records both conflicting dates
in every decision note. Every requested NHL ID must be eligible or the atomic
league transformation fails.
`icecast affiliate-review-birth-date-league` handles the inverse authority
case: the NHL identity is correct, but independent official sources support the
AHL date rather than the displaced NHL landing date. It preserves the NHL ID,
requires an exact normalized name, requires the supplied canonical date to
equal the AHL date and differ from the NHL proposal, unions novel absolute
evidence, and records both dates in an atomic league audit.
`icecast affiliate-review-collision-league` is the separate correction lane for
the exception board's `investigate_identity_collision` action. It selects the
displaced proposal ID and an explicit canonical identity, then remaps every
eligible league appearance atomically. The displaced date must differ from the
AHL date by at least 1,460 days; the canonical date must equal the AHL date;
the surnames must agree; and new absolute evidence, reviewer, timestamp, and
rationale are mandatory. Its audit retains the displaced ID/date and evidence
alongside the canonical mapping. It changes only the NHL mapping and never
rejects or removes the AHL player.
`icecast affiliate-review-reject` closes only
selected pending NHL identity mappings and requires repeatable provider IDs,
reviewer, timestamp, and an evidence-backed rationale. It does not assert that
the underlying AHL person is invalid: AHL-only players and feed-classified
non-player personnel remain distinguishable in the retained note. Repeatable
`--evidence-url` values are validated as absolute URLs and retained as
structured row evidence instead of being buried only in prose.

`icecast affiliate-review-reject-league` applies the same mapping-only
rejection semantics atomically to a league envelope. A provider ID that appears
for multiple clubs after a trade is closed on every pending occurrence. Every
requested ID must have at least one pending occurrence or the operation returns
no updated envelope. The separate league decisions artifact records all
team-bound batches, skips, evidence, reviewer authority, and the explicit fact
that AHL player and season records remain retained for a future sourced remap.

`icecast affiliate-review-exact-league`,
`icecast affiliate-review-aliases-league`, and the targeted conflict command
apply narrow evidence rules atomically across an
`ahl_identity_league_crosswalk.v1` envelope. Teams
without eligible rows are recorded as skipped. The optional decisions output
is an `ahl_identity_league_review_decisions.v1` audit containing every original
team-bound batch; the updated league envelope and audit are separate artifacts.

`icecast affiliate-review-league` composes any number of independently
snapshot-bound team-season crosswalks into the UI-neutral
`ahl_identity_league_review.v1` coverage board. It reports reviewed, rejected,
pending, resolved, and canonical-identity coverage by team-season and overall.
Every pending or rejected appearance enters the attention queue; recurring
rows are grouped by canonical NHL ID when present, otherwise by normalized AHL
name plus birth date. This surface is read-only and never creates approval
authority.

`icecast affiliate-review-board` projects that league review into the
UI-neutral `ahl_identity_exception_board.v1` triage contract. It recommends a
review action, retains teams/seasons/conflicting date pairs and evidence, and
ranks recurring multi-season exceptions ahead of lower-leverage one-offs. Its
published score is deterministic and read-only; rank never grants review
authority. Conflict pairs expose their absolute day delta; a delta of at least
1,460 days recommends identity-collision investigation rather than a date
override.

Preseason rollover keeps prior evidence separate from target affiliation. In
`ahl_preseason_rollover.v1` config, `ahl_team` is the target-season affiliate;
optional `prior_ahl_team` names the club in the prior official snapshot. When
`prior_ahl_team` is absent it defaults to `ahl_team`, preserving existing
same-affiliate inputs. `affiliate-status-draft --ahl-team` continues to select
the prior-snapshot club. This distinction supports relocation and affiliation
changes without relabeling historical roster evidence.

`icecast affiliate-review-show` is the read-only text/JSON inspection surface
for an existing crosswalk. IceLines projects the authoritative crosswalk into
the UI-neutral `ahl_identity_review_inspection.v1` contract, which carries
declared and recomputed counts, a stale-count flag, total and attention counts,
scope, evidence, notes, and discovery disclosures without changing state.
`--attention-only` hides routine exact-name-and-birth proposals and retains
pending non-exact or rejected rows. It can be combined with `--json` because
the output is explicitly an inspection view rather than a partial authoritative
crosswalk. Attention rows include canonical NHL names, both provider birth
dates, and every evidence URL needed for the review decision.

`icecast affiliate-review-apply` binds that finalized batch to the exact
season/provider/team/roster-fetch crosswalk. It supports `accept_proposal`,
explicit sourced `set_identity` alias/remaps, and `reject`; rejects unknown or
duplicate provider IDs, duplicate resulting NHL IDs, invalid evidence URLs,
empty notes, stale bindings, draft documents, and missing reviewer authority.
Untouched rows retain their prior status. Every applied row records reviewer,
timestamp, action, and note. Birth-date conflicts reject `accept_proposal` and
require the targeted sourced `set_identity` workflow.

`icecast affiliate-status-draft` emits the second, non-applicable
`ahl_preseason_organization_review.v1` gate for every prior affiliate player.
It is bound to the historical roster fetch, current camp, and a SHA-256
fingerprint of the identity crosswalk. Pending identities remain explicit
blockers; reviewed players absent from camp require a sourced retained,
departed, or other-league decision.

`icecast affiliate-status-apply` requires complete reviewed identity coverage,
a finalized reviewer and RFC3339 timestamp, absolute evidence URLs, notes, and
exact row coverage. It rejects stale fingerprints and emits the sourced
rollover config consumed by `affiliate-rollover`; it never creates a roster.
`icecast affiliate-status-show` is the read-only text/JSON inspection surface
for both draft and finalized review artifacts; it never changes review state.

After every row is explicitly marked `reviewed`, `icecast affiliate-input`
joins that identity artifact to separately authored projection facts keyed by
AHL provider ID. The join rejects missing/extra identities, altered official
names or birth dates, duplicate NHL IDs, missing evidence URLs, stale snapshot
authority, an empty official roster, or any pending/rejected row. A zero-row
review artifact can document preseason source coverage but cannot certify a
projection pool. Its JSON output feeds `icecast affiliate`; identity review
does not establish player value, assignment,
prospect status, professional-game totals, waivers, or recall readiness.

`icecast affiliate-rollover` emits `ahl_preseason_rollover.v1` by reconciling a
prior official affiliate roster and its exact-coverage identity crosswalk with
a matching current camp input/forecast. Canonical NHL player ID is the only
automatic merge key. It reports projectable F/D/G coverage plus identity,
organization-status, and waiver review lanes. It never emits an affiliate
projection: readiness still requires downstream professional-game,
development-rule, contract, injury, assignment-rights, and player-value facts.

`icecast affiliate-rollover-config-league` composes one explicit team config
for every sealed camp team from separate season-dated prior and target
affiliation catalogs. It never derives a historical relationship from the
current catalog and emits no prior-player decisions. `icecast
affiliate-rollover-league` then applies the forecast-native rollover adapter to
the exact league cohort and emits `ahl_preseason_league_rollover.v1`. Missing
team forecasts or source bindings are typed failures; built-but-not-ready teams
retain their identity, organization-status, position-shape, and waiver queues.
The sealed forecast path is semantically identical to the original explicit
camp-input path for every field used by rollover.

`icecast affiliate-status-draft-league` creates the corresponding
`ahl_preseason_league_organization_review.v1` envelope directly from the sealed
league forecast. Each child is the existing crosswalk-fingerprint-bound review
contract; the league layer adds exact team coverage and recomputed aggregate
counts but no decisions. After every required child row has sourced status,
reviewer, timestamp, and note evidence, set the league and child `draft` fields
to false and use `affiliate-status-apply-league`. Application is atomic across
the cohort. Mapping-rejected identities remain projection blockers but do not
invalidate unrelated, otherwise-complete status decisions.

`fetch career --league-crosswalk` acquires official NHL landing career rows for
the unique canonical IDs in a reviewed AHL league envelope. The resulting
cache feeds `affiliate-professional-games`, which counts only prior regular
seasons under a versioned league-treatment policy. Missing histories and known
professional leagues without an explicit include/exclude decision withhold
that player's total. The ledger reports the 260-game threshold test but does
not infer age exemptions, contracts, assignments, waivers, recalls, or lineups.
Policies declare `draft`, `provisional`, or `final` authority. Draft and
provisional runs may calculate threshold, age, and youth-exemption facts for
review, but only a final policy can certify `development_rule_qualified`.
Affiliate projection input preserves that certified value separately from its
raw total; when present, it controls development/veteran classification.
`affiliate-professional-games-apply` is the fail-closed bridge: it accepts only
a final ledger, rejects identity/team/fact conflicts, changes only the two rule
facts, and emits a fingerprint-bound application that `affiliate-input` can
consume directly.

`icecast affiliate-map` emits the dated `ahl_affiliation_catalog.v1` authority
used to connect all 32 NHL organizations to their current AHL affiliates. For
the 2026-27 catalog, `icecast affiliate` rejects a mismatched free-form
affiliate label rather than silently projecting a player pool onto the wrong
club.

`icecast organization` builds The System as the UI-neutral
`organization_lineup_forecast.v1`. Its input combines a complete
`team_lineup_projection.v1` with the matching `ahl_affiliate_projection.v1`.
It emits four forward lines, three defense pairs, and two goalies at each level,
plus NHL extras, AHL depth outside the dressed lineup, and position-group recall
ladders. Team, season, current affiliation, development-rule compliance, unit
completeness, and cross-level player identity all fail closed. NHL special teams
are preserved; AHL PP/PK remains explicitly unavailable until affiliate role
evidence is supplied.

`icecast blender` reads `team_lineup_projection.v1`, ranks the submitted
lineup plus deterministic legal one-swap alternatives, and emits
`line_combination_forecast.v1`. `--scenario-out` writes a reusable
`team_season_scenario.v1`: The Bench opens with the submitted lineup, reviews
standings-points percentage after each configured team-game window, and
advances through ranked choices after a miss. Optional `--pair-evidence`
accepts explicitly labeled shift, coarse same-game, or simulated pair inputs;
absent evidence stays neutral and is disclosed rather than invented.

`icecast bench` is the sealed evidence bridge from the per-game IceCast
baseline to opponent-specific coaching plans. `icecast season
--game-forecast-out` writes the required `team_game_forecast.v1`; Bench joins
it to a UI-neutral lineup, one team decision profile, current-roster player
role evidence derived from `--stats-season`, and an explicit style-evidence row
for every scheduled opponent. Missing or no-read opponent evidence fails
closed. The output is `team_season_game_plan_schedule.v1`, while
`--scenario-out` writes its simulation-ready `team_season_scenario.v1`.

Web card routes served by `icelines serve`:

```text
/icecast/20262027/NYR/card?scenario=nyr-development-variance&page=depth-chart
/icecast/20262027/SEA/card?scenario=sea-development-variance&page=insider
/api/v1/cards/team-prognosis/20262027/NYR?scenario=nyr-development-variance
/icecast/20262027/NYR/simulation?page=scoreboard
/icecast/20262027/SEA/simulation?page=insider
/api/v1/cards/season-simulation/20262027/NYR
/icecast/20242025/NYR/simulation?page=insider
/api/v1/cards/season-simulation/20242025/NYR
/icecast/20242025/NYR/movement?page=shift
/icecast/20242025/SEA/movement?page=insider
/api/v1/cards/forecast-movement/20242025/NYR
/icecast/20242025/NYR/history?page=tape
/icecast/20242025/SEA/history?page=insider
/api/v1/cards/forecast-history/20242025/NYR
```

The TUI commands `season-card`, `season-card NYR`, and `season-card SEA` open
the same sealed season-simulation documents. Press `p` to switch between The
Scoreboard and The Insider, `t` to switch teams, or `c` for side-by-side NYR
and SEA projections from the same league run.

For the sealed completed-season replay, use `replay-card NYR` or
`replay-card SEA`. Its Insider page adds confirmed actual records and points,
league and focused-team pick accuracy, Brier score, calibration error,
coin-flip skill, and the best tested chronological Elo blend.

For the sealed Jan. 31 → Feb. 28, 2025 checkpoint comparison, use
`movement-card NYR` or `movement-card SEA`. Press `p` for The Shift/Insider,
`t` to switch teams, or `c` for side-by-side movement from the same two
league-run fingerprints.

For the multi-checkpoint projection, use `history-card NYR` or
`history-card SEA`. Press `p` for The Tape/Insider, `t` to switch teams, or
`c` for side-by-side history sourced from the same sealed league runs.
The sealed showcase includes Jan. 31, Feb. 28, and Mar. 31, 2025. After
building the CLI, `scripts/generate-icecast-history-showcase.ps1` regenerates
the history and cards; pass `-Season`, `-StatsSeason`, `-CheckpointDate`,
`-Trials`, and `-Seed` to use the same pipeline for another year.

After building the CLI, `scripts/generate-icecast-validation.ps1` generates
the chronological replay set and invokes `icecast backtest`. Its defaults cover
2021-22 through 2025-26 with deterministic season seeds and prior-season stats,
writing to `~/.icelines/reports/validation`. The runner validates schema,
season, and graded-game coverage before admitting a replay, but never upgrades
partial roster authority. `-PlanOnly` prints the complete plan without resolving
the executable or writing files. Valid replay artifacts are reused by default;
pass `-ForceReplay` after model changes. The quick
`scripts/test-icecast-validation-runner.ps1` fixture proves initial generation,
resume-only backtesting, and forced regeneration without running simulations.

`icecast season` emits `team_season_forecast.v1`, including its complete
`team_game_forecast.v1` game list. JSON always contains the full league run;
repeat `--team` to choose summary/text game tables. For the
current 2026–27 season the command refuses incomplete schedules unless it sees
exactly 1,344 games and 84 games (42 home/42 road) for every team.

`icecast calibrate-development` measures consecutive-season player-value
changes across completed seasons. Skaters must reach 20 games in the outcome
season and goalies 15; shortened lockout/pandemic seasons are excluded.
The v2 value model normalizes each lens within season and position: scoring,
ice time, shots, power-play production and plus/minus for skaters; save
percentage, inverse GAA, starts and shutout rate for goalies. Values are
credibility-shrunk toward 50, missing optional lenses are neutral, and extreme
feature z-scores are capped.
Position/age/experience/prior-value cohort rates are shrunk toward the global
rate, and JSON retains sample sizes, empirical rates, calibrated rates, median
deltas, a latest-season player lookup, examples, thresholds, and
leakage/selection disclosures. The bundled history does not yet provide
complete blocks, xG, possession, matchup-quality, or special-teams deployment.

The game baseline uses roster/depth strength plus home ice, rest,
back-to-backs, congestion, itinerary distance, and timezone displacement. A
seeded chronological league simulation produces consistent records, point
ranges, playoff/Presidents' Trophy odds, and longest-win-streak distributions.
Scenario injuries, goalie availability, form, deadline trades, playoff series,
and bounded hunt/spoiler state are supported. Live confirmations remain a
later IceCast milestone; point-in-time historical replay is available through
`icecast season --replay-mode rolling --through`.

`icecast scenario import` is the boundary between local authoring and reusable
scenarios. It stores immutable content under `~/.icelines/scenarios`, records a
stable ID, scope, evidence label, calendar fingerprint, and SHA-256 hash, and is
idempotent when the same content is imported again. Web, TUI, cards, and
reproducible comparisons use IDs; only the CLI accepts an ephemeral
`--scenario PATH`, and it cannot be combined with `--scenario-id`.

Scenario JSON accepts dated event kinds `injury`, `goalie`, `trade`, `return`,
`form`, and `custom`. Each event supplies `team`, `effective_date`, optional
`end_date`, signed `strength_delta`, and `occurrence_probability` from 0 to 1.
Occurrence uses an event-specific seeded stream, so adding an event does not
rewrite unrelated game luck. Trade events after `trade_deadline` are rejected.
When a 2026–27 scenario omits its deadline, the CLI attaches the user-provided
March 5, 2027 boundary. See `examples/icecast-scenario.json`.

`--auto-personnel` generates seeded player-aware injury and goalie-availability
events from each roster's highest-impact multi-lens player records. Age and
games played alter risk; rating and goalie role alter bounded team impact. The
generated events are serialized with the forecast for inspection and are
modeled risks rather than live status claims. Authored and automatic scenarios
can be combined.

`--trade-mode plausible` builds up to six named deadline hypotheses from team
outlook and roster records. Buyers target their weakest F/D/G bucket; sellers
contribute a high-impact player aged 33 or younger when available. Each paired
buyer/seller effect has one correlation key and occurrence probability, making
the movement atomic per trial. These are transparent hypothetical scenarios,
not rumors or claims about real negotiations.

Plausible trade mode automatically runs an otherwise-identical no-trade
counterfactual. `scenario_impacts` reports scenario-minus-baseline expected
points, playoff and Presidents' Trophy probability, and longest-win-streak
deltas for all teams. Comparisons refuse different seeds, trial counts,
schedules, seasons, or team sets.

Text labels the probability-weighted columns `Mkt` and the forced-occurrence
columns `Done`. JSON stores the corresponding full-league rows in
`scenario_impacts` and `conditional_scenario_impacts`. This prevents a 30%
trade proposal from being mistaken for the value of that trade if completed.

Each trial proceeds through the seeded divisional/wild-card playoff bracket and
best-of-seven 2-2-1-1-1 series. Team rows and JSON include
`second_round_probability`, `conference_final_probability`,
`stanley_cup_final_probability`, and `stanley_cup_probability`. Scenario impact
rows carry the matching round/Cup deltas.

`pivotal_games` identifies late-season schedule dates that become hunt or
spoiler games across trials. Conference ranks 7-10 define the hunt; ranks 13-16
can act as spoilers. Race motivation changes game probability by at most 0.4
points, while rolling five-game form is capped at 1.5 points. Text presents the
top five focused-team matchups under **The Bubble**.

**The Scoreboard** renders top-five Presidents' Trophy, Stanley Cup, and
longest-win-streak leader probabilities. **The Gauntlet** renders each focused
team's hardest and easiest consecutive five-game windows with average win
probability, expected wins, opponents, road count, back-to-backs, and travel.
The structured equivalents are `league_leaders` and `schedule_stretches`.

If the schedule contains completed games, **The Review** reports pick accuracy,
binary Brier score, binary winner log loss, and three-way regulation-home /
regulation-away / OT-SO log loss. Positive skill deltas mean the forecast beat
the corresponding 50/50 or equal-three-outcome baseline. Ten-point home-win
probability bins report observed rates and expected calibration error. The full
summary also reports logistic calibration intercept and slope once at least 20
games with both outcomes are available. Their ideal values are 0 and 1; they
are retrospective diagnostics and are not forecast inputs. The full
summary retains standard errors and approximate 95% Wald intervals for both
parameters. The full
JSON game ledger adds
`actual_away_score`, `actual_home_score`, `actual_winner`, `actual_ending`,
`pick_correct`, `brier_score`, `binary_log_loss`, and
`multiclass_log_loss`; the top-level `accuracy` summary and
`calibration_bins` are absent until at least one final can be graded. A
three-way score remains null when REG/OT/SO ending metadata is unavailable.
Final results are evaluation labels joined after probability computation, not
forecast features.

`accuracy.baselines` scores `home_only`, `rolling_standings`, and
`chronological_elo` over the exact same graded games. Home-only uses no team
information. Rolling standings uses only points earned before the game date,
regressed against a neutral 20-game prior. Elo begins at 1500 with a 22-point
home advantage and K=20; an OT/SO result is 0.75/0.25, and same-date ratings
are frozen until every game that day has been forecast. Per-game
`home_only_home_win_probability`, `standings_home_win_probability`, and
`elo_home_win_probability` make each comparison reproducible. Positive
`model_*_improvement` means IceLines beat that baseline; negative means the
baseline had lower loss. Non-rolling runs omit standings, label the Elo
comparison `frozen_equal_rating_elo`, and do not update it from in-season
results, preserving an equal-information comparison.

`accuracy.ablations` evaluates each factor by subtracting its frozen,
reconciled `home_win_probability_delta` from every graded game and rescoring
the same outcomes. Rows include `games_affected`,
`mean_absolute_probability_delta`, ablated pick/Brier/log-loss scores, and
signed model improvements. This is a local factor-removal audit, not a refit;
positive improvement means including the factor helped, while negative means
the ablated forecast scored better.
Rolling replay attributes earlier-result strength, verified opening-roster
priors, and later player changes separately as `strength`, `opening_roster`,
and `personnel`; their deltas still reconcile exactly to the published game
probability.
The verified opening-strength cohort is centered at neutral 50 before replay.
One shared normalization offset preserves its relative team ordering; a
one-team partial cohort is therefore neutral rather than receiving an
unverifiable absolute edge over uncovered teams.

Historical replay accepts `ARI` for pre-Utah schedules and retains `UTA` for
modern schedules. Both map to the Western Conference Central Division;
Arizona's historical arena coordinates and timezone are used for itinerary
features. A season schedule remains authoritative, so the aliases are never
merged into one season's team list.

In rolling replay, `accuracy.elo_blend_sweep` evaluates weights 0.0 through 1.0
in 0.1 steps using `p = IceLines * (1 - elo_weight) + Elo * elo_weight`.
`best_elo_blend_by_brier` retains the lowest-Brier row, breaking exact ties
toward less Elo. The sweep is empty for frozen forecasts and never changes the
primary game probabilities; it is calibration evidence for later model work.

`icecast backtest` reads at least three JSON files previously emitted by
`icecast season --json`. It emits `team_game_forecast_validation.v1` with a
game-weighted `pooled_sweep`, `pooled_best_by_brier`, and one row per held-out
season. Duplicate seasons, empty grading, and incompatible or non-finite blend
grids or calibration observations are rejected. A holdout row's
`selected_elo_weight` is learned only from
`training_seasons`; signed fields compare its untouched-season loss with
unblended IceLines and pure Elo. The report's `promotion_status` is backed by
named `promotion_checks`: five or more seasons, authoritative opening rosters
for all inputs, every holdout beating IceLines, at least 60% of holdouts beating
pure Elo, pooled blend improvement over pure Elo, and holdout-selected weight
span at most 0.20. Even a clean pass is only
`candidate_for_versioned_evaluation`; defaults are never changed by this
command. Missing roster authority produces
`evaluation_only_missing_roster_authority`.

The same artifact includes `calibration_holdouts`, beginning with the second
chronological input. Each row fits logistic calibration only on earlier
supplied seasons and then scores the untouched next season, reporting frozen
intercept/slope plus held-out Brier and binary-log-loss improvement. This is
the deployable-evidence path; same-season calibration remains diagnostic only.
`calibration_summary` pools those untouched holdouts by game count rather than
averaging seasons, and reports before/after loss, signed gains, and the number
of holdouts improved for both metrics. It also reports paired per-game standard
errors and normal-approximation 95% intervals. Those intervals do not model
parameter-selection uncertainty. A second interval uses a delete-one-holdout-
season jackknife to expose season-clustered variation; it remains conditional
on the fitted chronological sequence and is unstable with few holdouts. The
machine-readable evidence label remains `insufficient_holdouts` until four
holdout seasons exist, then becomes `positive`, `negative`, or `inconclusive`
according to whether the clustered interval lies above, below, or across zero.

The July 23, 2026 default runner execution produced four chronological
calibration holdouts over 5,248 games. Brier improved by 0.002531 and binary log
loss by 0.005168, but their season-clustered 95% intervals crossed zero, leaving
both evidence labels `inconclusive`. Recalibration improved 3/4 holdouts and
worsened the newest 2025-26 holdout by 0.001246 Brier and 0.002637 log loss.
The separate blend gates passed at a 90%
Elo pooled minimum, while opening-roster authority remained 0/5; the overall
status therefore stayed `evaluation_only_missing_roster_authority`.

An opening-roster archive manifest has this shape, repeated once for every
season member:

```json
{
  "schema": "icecast.opening_roster_archive.v1",
  "season": 20242025,
  "opening_date": "2024-10-04",
  "captures": [
    {
      "team": "NYR",
      "archive_url": "https://web.archive.org/web/20240930074603id_/https://api-web.nhle.com/v1/roster/NYR/20242025"
    }
  ]
}
```

The one-row example is illustrative and intentionally fails default coverage.
A promotion-authoritative manifest must contain exactly one matching immutable
official-API capture for all season teams. Either the season endpoint or its timestamped official
`current` endpoint is accepted inside the July 1-to-opening preseason window.
`--dry-run` validates schema, identity, timestamps, URLs, and
coverage without network downloads or snapshot writes. The apply command first
downloads and parses every roster, then creates an integrity-sealed snapshot;
partial downloads never become authority. Archive observation time and local
import time remain separate.
Each payload gets three bounded attempts. The importer recognizes gzip magic
bytes even when Wayback omits `Content-Encoding`, limits decompressed rosters to
4 MiB, and includes a short response signature when parsing still fails.

`--allow-partial-evaluation` accepts a non-empty manifest without full league
coverage and seals it with the same provenance checks. Rolling replay applies
player-value weights only to the manifest-verified teams and leaves every other
team neutral. Its authority status is `partial_evaluation`, so `icecast
backtest` never counts it toward the opening-roster promotion check.

`discover-opening-rosters` derives opening day and membership from the loaded
schedule, then selects the latest CDX capture strictly before that date for each
official season-roster endpoint. When that endpoint has no capture, discovery
also checks the official `current` roster archive inside the same preseason
window. Its report separates `missing_teams` from
`request_errors`, retains selected URLs, and embeds `import_manifest` only for
complete coverage. `--manifest-out` fails closed when coverage is incomplete.
`--partial-manifest-out` writes the non-empty verified capture set for an
explicit evaluation-only import while leaving `import_manifest` absent.
Discovery uses at most four concurrent archive requests.
Parsed CDX responses are atomically cached under the IceLines cache root by
season/team. A later transport failure may reuse that response and is listed in
`cache_fallback_teams`; unrecoverable failures remain in `request_errors`.
`--cache-only` skips all Internet Archive requests and revalidates only cached
CDX responses. An endpoint without a saved response remains an explicit
request error, preserving the distinction between “not captured” and “not
checked.”

`--retrospective-opening-lineups` requires rolling replay and is explicitly not
archive authority. For each team, it selects the first scheduled regular-season
game, loads the official NHL boxscore, requires 15–18 unique dressed skaters
and two goalies, and retains only player ID, abbreviated display name, and position.
That team's first-game date is its personnel evidence cutoff, so earlier
transactions are already reflected while later transactions may alter the
lineup. Boxscores are atomically cached under the IceLines cache root and
`--refresh` re-fetches them. This mode is always `retrospective_evaluation` and
never counts toward `opening_roster_authority` promotion.

Season simulation currently accepts 2021–22 and later alignment. A pre-2021–22
request fails with an explicit historical division/playoff-authority error
rather than applying the current bracket silently. Focus-team validation uses
the loaded schedule, so historical `--team ARI` is accepted and default SEA is
omitted for pre-expansion schedules.

`fetch rosters --season ...` is also season-aware: Coyotes seasons request
`ARI`, Utah seasons request `UTA`, and the pre-Seattle 2020–21 audit season has
31 teams. Fetch time remains the snapshot evidence time. Downloading a
historical endpoint today never fabricates a pre-opening capture and therefore
cannot satisfy The Crease gate by itself.

`--replay-mode rolling` switches to **The Film Room — IceReplay**. It uses a
neutral regressed opening prior, then updates each team from standings points
and goal differential in completed games strictly before the forecast date.
Same-date results are applied as one batch only after all that date's picks are
frozen. Text `Known` and JSON `away_evidence_games`, `home_evidence_games`, and
`evidence_cutoff_date` expose the cutoff. Replay refuses current-roster
substitution and simulated personnel or trade combinations. Player-value
effects require the dated opening-roster authority described below; without
it, personnel history remains auditable but strength-neutral.

Add `--through YYYY-MM-DD` to turn a completed rolling replay into an as-of
season forecast. Final games through that date seed every trial's standings,
form, and streak state; later games are simulated. IceLines removes all later
scores and dated personnel evidence before building rolling strengths, emits
the typed `as_of_date` plus `replay_checkpoint`, and fails if any required
earlier result is missing or any future result reaches the simulator. The text
report's **The Checkpoint** table and the card Scoreboard show actual GP,
W-L-OTL, points, and games remaining before the projected final distribution.
**The Rest of the Way** then shows model-expected remaining W-L-OTL and points;
those values are core-owned fields that reconcile observed plus expected
remainder to the projected final averages.
`--through` requires rolling mode and
can be combined with `--isolated-impacts`; its baseline, natural scenario,
single-event, and forced-ceiling runs all share the identical fixed-result
boundary, trials, and seed.

`--ignore-replay-personnel-after YYYY-MM-DD` is a paired-counterfactual tool
and requires rolling replay. It keeps all personnel evidence through the date
and omits only later events while leaving the game-results checkpoint intact.
Use it with the same `--through`, trials, seed, and other inputs as the actual
later checkpoint; it is not a general-purpose alternate-history switch.

`icecast movement --earlier ... --later ...` builds
`team_season_forecast_movement.v1` from two complete league artifacts. **The
Shift** reports later-minus-earlier changes in projected points, playoff and
Cup probability, newly completed games, observed standings points, and
expected remaining points. Both full artifacts are fingerprinted before the
text team filter is applied. Comparison requires identical season, schedule
size, teams, trials, and seed; two dated checkpoints must be chronological.

`icecast movement-card --input ... --team ...` projects one team from that
sealed movement artifact into `card_document.v1`. The two source fingerprints,
cutoffs, simulation identity, typed deltas, and disclosures remain core-owned;
generic terminal, web, SVG, and downstream renderers do not recalculate them.

`icecast history --input ... --input ...` builds
`team_season_forecast_history.v1` from two or more chronological `--through`
artifacts. **The Tape** reports each checkpoint's projected points, playoff
odds, observed games/points, and consecutive points/playoff/Cup movement.
It also reports first-to-last movement for each focused team and the league's
top-five projected-points risers and fallers. Each focused team carries its
league movement rank with deterministic team-code tie breaks, an
improving/declining/mixed/stable trajectory, and its largest signed checkpoint
swing. Checkpoints retain P10/P50/P90 points. Net movement materiality compares
the absolute first-to-last change with the average first/last P10-P90 width;
it is explicitly descriptive rather than a significance test. Every source fingerprint is retained. Inputs must have strictly increasing
dates and identical season, schedule, teams, trials, and seed. Text may focus
teams; JSON always retains the full league history.

The first-to-last movement bridge must reconcile net projected-points change
to confirmed standings points gained plus the change in expected remaining
points. Core stores and validates all three values before CLI, TUI, web, or SVG
rendering.

History also reports a pace-normalized attribution. It values games completed
between the first and last checkpoints at the first checkpoint's average
expected remaining points per game. Realized points versus that prior pace plus
revaluation of the still-unplayed outlook must reconcile to net movement. This
is a descriptive accounting view, not a causal schedule-strength decomposition.
Each checkpoint after the first also retains this split for its immediately
preceding interval; text output prints the interval attribution beneath that
checkpoint and UI-neutral cards expose the same typed metrics.

`icecast history-card --input ... --team ...` projects one team from that
history into `card_document.v1`. Every checkpoint fingerprint, absolute level,
consecutive delta, and disclosure remains core-owned; the TUI, web, SVG, and
downstream renderers only select and lay out sections.

**The Crease — Opening Roster Gate** reports the structured
`opening_roster_authority` decision. A roster snapshot qualifies only when it
is sealed, season-matched, captured before the first game date, integrity
valid, and non-empty for every scheduled team. Same-day snapshots are rejected
because game timestamps are not available to prove a pre-puck cutoff. Passing
the gate enables coverage-regressed `opening_strengths`: 55% from the top 12
forwards, 30% from six defensemen, and 15% from two goalies, using only the
preceding completed season. Missing histories are neutral rather than zero,
and current-season results progressively replace that opening prior.

`opening_strengths[].players` retains exact roster identity, position group,
prior/modeled value, and opening-slot selection. Only transactions strictly
after `personnel_events_effective_after` alter that baseline. The replay
recomputes active 12F/6D/2G groups after recalls, assignments, IR placements,
and activations; per-game signed changes appear as
`away_personnel_strength_delta` and `home_personnel_strength_delta`.
For newcomers absent from the snapshot, `resolved_players` includes
`prior_position_group`; a later recall or waiver claim joins the player to the
appropriate lineup pool only when both that group and `prior_value` are known.

Modern bundled transaction seasons also render **The Wire**. Every sourced
trade, recall, assignment, waiver, signing, and IR row is retained in
`personnel_evidence` and becomes known only after its date. Game rows expose
`away_known_personnel_events`, `home_known_personnel_events`, and conservative
active-IR signals. Only one-direction IR placement/activation prose changes
the signal; mixed rows remain neutral, and no generic transaction changes team
strength without player identity/value evidence.

`resolved_players` links exact full-name mentions to stable NHL player IDs.
Names with multiple identity candidates remain in `ambiguous_player_names`.
The identity catalog is used only for entity resolution; its full-season stats
do not become replay features.

Every resolved player also carries `action` and `membership_delta`. Mixed rows
are parsed per player, so recalls, assignments, and IR placements in one source
row do not share a guessed direction. Only recalls, waiver claims, and
assignments alter NHL active-roster evidence; trades, acquisitions, and
releases remain personnel evidence without asserting active status.

`paired_trades` links exactly one same-date `traded_away` row and one
`acquired` row for the same stable player ID on different teams. If the source
lineup is already known to contain that player, replay transfers membership,
player value, and active IR state atomically. Otherwise the row is retained as
`source_not_known_active` organizational evidence and does not change either
lineup. The Wire reports active-lineup and organizational-only counts.

`membership_intervals` opens and closes those sourced active-roster periods.
Removal-only periods are labeled `implied_preexisting`, while repeated
transitions appear in `membership_anomalies` instead of creating overlapping
intervals. Without dated opening-roster authority, interval values are audit
metadata and do not alter replay strength.
Repeated active-roster and IR transitions also leave cumulative game state
unchanged after the first valid player transition; raw source-event counts are
preserved separately.

`prior_season`, `prior_games_played`, and `prior_value` provide the only player
performance value admissible for a season-start replay. Values use the
immediately preceding completed season with small-sample regression; absent
history remains null rather than borrowing replay-year results.

## Fantasy league

### Schedule edge and draft-calendar fit

This report appears as **The Bench — The Gauntlet — Fantasy Schedule Edge**:
The Bench is the fantasy workspace, while The Gauntlet names schedule-density
and off-night analysis.

```bash
icelines fantasy schedule-edge --refresh
icelines fantasy schedule-edge --week 2026-10-05
icelines fantasy schedule-edge --teams NYR,COL,EDM
icelines fantasy schedule-edge --off-night-max-games 3 --classes 8
icelines fantasy schedule-edge --json --out schedule-edge.json
```

The report uses Monday-Sunday fantasy weeks. It ranks every team by games,
quiet-slate games (dates with at most four NHL games by default), and a scarcity
score that sums `1 / games on the NHL slate`. Eight exact-date overlap classes
group teams that frequently play together; the marked user roster adds its
highest-collision pairs and the lowest-overlap available team complements.
`--teams` overrides the marked roster for draft planning. The first successful
load persists the deduplicated season schedule locally; `--refresh` replaces it
from all 32 official club feeds.

### Full-season stress simulation

```bash
icelines fantasy season-sim --league "My League"
icelines fantasy season-sim --league "My League" --team "My Team"
icelines fantasy season-sim --trials 250 --seed 20262027 --json
icelines fantasy season-sim --injury-rate 0.003 --trade-probability 0.50
icelines fantasy season-sim --scenario-matrix
icelines fantasy season-sim --scenario-matrix --trials 120 --json
icelines fantasy season-sim --opponent-pickup-accuracy 0.70
icelines fantasy season-sim --manager-matrix --trials 120
icelines fantasy season-sim --pickup-reserve 1
icelines fantasy season-sim --reserve-matrix --trials 60
icelines fantasy season-sim --exceptional-reserve-min-value 6 --exceptional-reserve-min-games 3
icelines fantasy season-sim --strict-pickup-reserve
```

`season-sim` is a seeded, non-mutating Monte Carlo stress model. It creates a
synthetic league from completed-season player rates, uses exact 2026-27 game
dates and daily multi-position slot assignment, and simulates weekly pickups,
fair-value trades, scheduled-player-game injuries, IR/IR+ replacements,
recoveries, missed starts, and roster churn. Regular standings use Monday-Sunday
weeks and report average W-L-T records, average seed, and No. 1-seed
probability; six qualifiers then play a three-round head-to-head bracket across the
final three weeks, with first-round byes for seeds one and two. The command does
not write simulated transactions to FantasyDb or claim to forecast real injuries.
Results separate first-round, semifinal, and final exits from championships, so
a dominant regular season can still expose one-week playoff upset risk.
`--team` locks every resolved player from a partial or complete saved roster
before legally filling open spots; without it, the marked user team is used.
Use `team-add --stats-season 20252026` and `team-show --stats-season 20252026`
when reconstructing a historical roster from completed-season data.
Use `league-scheme-set dexters-dawgs --league "My League"` to apply the saved
Dexter's Dawgs weights without recreating the league.
`--scenario-matrix` holds the roster, seed, scoring, schedule, and trial count
constant while comparing clean, baseline, and high-chaos injury/trade settings.
The text view reports each environment's delta from baseline; JSON contains all
three full simulation views and their scenario labels.
`--opponent-pickup-accuracy` explicitly stress-tests transaction decision
quality. Team one retains the best projected weekly add; an opponent miss picks
the second- or third-ranked add. `1.0` is the neutral default, and the simulator
does not apply a hidden manager-skill points bonus.
Randomness is domain-separated: pickup, trade, injury, and performance rolls
remain reproducible without one scenario's extra decision consuming another
scenario's injury or scoring roll.
`--manager-matrix` compares parity (100%), moderate edge (85%), and strong edge
(70%) under the same baseline environment. It is mutually exclusive with
`--scenario-matrix`, and its point deltas use parity as the reference.
Pickup and trade events preserve each roster's ability to fill every configured
active slot. Multi-position eligibility participates in that matching; a move
is rejected when aggregate positional counts look acceptable but legal slot
assignment fails.
Injured players occupying simulated IR/IR+ are not eligible synthetic drops or
trade pieces. Replacement identity follows subsequent add/drop and trade swaps,
so the correct current substitute is released when the original player returns.
Complete locked rosters and every synthetic draft are position-validated before
trials. A complete imported roster is checked directly rather than first being
forced through an unrelated temporary synthetic draft.
On a recovery date, the returning player and substitute release are processed
before Monday's pickup/trade window, so transactions evaluate the actual current
roster rather than stale IR state.
The transaction window runs every simulated morning rather than only Monday.
Daily pickup priority rotates, and legal pickups plus later injury replacements
consume the same four-move Monday-Sunday counter.
Drops and released injury substitutes enter the saved waiver window rather than
returning immediately to free agency. A player dropped Monday is excluded until
Wednesday under the configured two-day rule.
Seven-day pickup gain is reduced by a three-game retention cost when the drop's
league-scored per-game rate exceeds the add's. This preserves schedule streaming
between comparable players without sacrificing a star solely for a quiet week.
Team one holds one acquisition back from proactive streaming through Friday by
default, then releases it Saturday if unused; injury replacements may use the
full weekly limit throughout. Set `--pickup-reserve 0` to stress an all-in
streaming policy or a larger value to model more caution.
The `IR blocked` result isolates long-injury replacements rejected because the
weekly acquisition budget was already exhausted.
`weekly-budget`, `weekly-pickups`, and `morning` expose both hard-limit remaining
moves and the smaller safe proactive budget. With three of four moves used
Monday-Friday, ordinary streams are withheld, an IR/IR+ replacement may use the
last move, and the reserve releases Saturday.
The morning surface may flag an exceptional reserve override only when the move
adds at least 6.0 projected net value and 3.0 usable starts and no roster status
requires a pregame refresh. Uncertain injury evidence tightens the policy.
`--reserve-matrix` holds all random domains constant while comparing no reserve,
a strict Friday reserve, and the adaptive threshold. It is mutually exclusive
with the scenario and manager matrices. The season model uses three extra
seven-day scheduled games as a stable proxy for two optimized usable starts.

### Draft and daily assistant rules

```bash
icelines fantasy assistant-rules
icelines fantasy assistant-rules --league "My League" --json
icelines fantasy assistant-setup
icelines fantasy assistant-setup --league "My League" --json
```

`assistant-rules` safely previews either the persisted league contract or the
configured 2026-27 default. `assistant-setup` persists the 2 C / 2 LW / 2 RW /
3 D / 1 skater UTIL / 2 G active shape, four unrestricted bench slots, two IR,
two IR+, four weekly acquisitions, two-day waivers, same-day free agents, and
daily lineup changes. An active league is required unless `--league` is given.

### Live draft board

This report appears as **The Bench — War Room — Draft Board**.

```powershell
Get-Clipboard | icelines fantasy draft-board --taken-file -
icelines fantasy draft-board --taken-file taken.txt --top 20
icelines fantasy draft-board --taken-file yahoo-draft.csv --json
icelines fantasy draft-board --eligibility-file yahoo-player-pool.csv
icelines fantasy draft-board --pick "Connor McDavid"
icelines fantasy draft-board --stats-season 20252026 --league "My League"
```

The draft board uses the active league scoring scheme and completed statistics,
then adjusts transparently for open starter slots, positional replacement
level, platform multi-position eligibility, incremental non-collision dates,
quiet slates, and exact-date roster collision. `--pick` previews the next board
without adding anyone to FantasyDb. Newline and common player-name CSV columns
are accepted; ambiguous and unresolved taken rows are reported and never
silently removed from the available pool. Injury and role deductions remain
explicitly disabled until the evidence/freshness phase is implemented.
Supplying `--eligibility-file` explicitly persists resolved platform positions
for the league; C/LW, C/RW, LW/RW, D, and G are supported, while duplicate,
ambiguous, unresolved, and invalid rows remain visible in the output.

### Weekly acquisition ledger

Weekly recommendations appear as **The Bench — Waiver Wire — Weekly Pickups**;
breakout searches appear as **The Bench — Call-Up Board — Sleepers**.

```powershell
icelines fantasy weekly-budget
icelines fantasy weekly-budget --at 2026-10-08T07:00:00-07:00 --json
icelines fantasy weekly-pickups --date 2026-10-08 --top 20
icelines fantasy weekly-pickups --candidates 75 --json
icelines fantasy sleepers --positions D --top 20
icelines fantasy sleepers --positions LW,RW --json
icelines fantasy acquisition-record --add "Darren Raddysh" --drop "Bench Defenseman"
icelines fantasy acquisition-record --add "Goalie Name" --kind waiver --json
```

The budget uses the league timezone and Monday-Sunday boundaries, including
Pacific DST transitions. A counted `acquisition-record` is rejected once four
moves have been used. Recording a drop creates a waiver window ending exactly
two days after the effective timestamp. These commands update only IceLines'
local ledger; they do not perform a move on Yahoo or another fantasy platform.

`weekly-pickups` simulates each remaining date through Sunday using the legal
active-slot assignment engine. Every candidate is tested against each legal
drop (and an open roster slot when available); rankings use incremental playable
starts, active league-scored value, dropped rest-of-week value, waiver
reacquisition cost, and pickup-budget cost. Raw scheduled games that would be
benched do not count as usable starts.
When the playoff calendar is configured, the top 15 available candidates are
also simulated against every legal drop across that window. The saved-calendar
retention value uses legal starts and active-lineup value, appears explicitly in
the recommendation reasons, and is capped at +6/-4. Candidates outside that
bounded playoff beam retain a neutral future-schedule component rather than a
fabricated zero-game claim.

### Sleeper discovery

`sleepers` excludes players rostered anywhere in the selected fantasy league
and compares 2025-26 rates with 2024-25 by default. Its typed
`fantasy_sleeper_board.v1` score separates active-league fantasy-rate growth,
shots/hits/blocks growth, power-play growth, quiet-slate value,
multi-position flexibility, newcomer opportunity, and small-sample risk.
Candidates need at least 10 games. Baseline source gaps are reported and never
receive fabricated growth or newcomer credit. This is a discovery board—not an
injury, lineup-role, or rest-of-season projection—and currently covers skaters.

### Status evidence and injury plan

```powershell
icelines fantasy status-record "Player Name" --status dtd --source "league app"
icelines fantasy status-record "Player Name" --status out --source "team report" --confidence confirmed --observed-at 2026-10-08T16:00:00-07:00
icelines fantasy status-show
icelines fantasy status-show "Player Name" --max-age-minutes 180 --json
icelines fantasy goalie-start-record "Igor Shesterkin" --date 2026-11-12 --state confirmed-starting --source "team reporter"
Get-Clipboard | icelines fantasy goalie-start-import --file - --source "daily goalie report"
icelines fantasy goalie-start-import --file examples/fantasy-goalie-starts.csv
icelines fantasy goalie-start-template --date 2026-11-12 --out goalie-news.csv
icelines fantasy goalie-start-show --week 2026-11-09 --max-age-minutes 180
icelines fantasy goalie-plan --week 2026-11-09 --strategy balanced
icelines fantasy goalie-plan --date 2026-11-12 --strategy floor --current-appearances 2 --json
icelines fantasy injury-plan --date 2026-10-08 --json
icelines fantasy morning --date 2026-10-08
icelines fantasy morning --date 2026-10-08 --at 2026-10-08T17:30:00-07:00
icelines fantasy morning --date 2026-10-08 --current-goalie-appearances 2
icelines fantasy morning --material-only --json
icelines fantasy morning-card --date 2026-10-08
icelines fantasy morning-card --date 2026-10-08 --current-goalie-appearances 2 --json
```

Supported statuses are healthy, DTD, GTD, out, IR, LTIR, suspended,
personal, and unknown. Every observation retains source, optional URL, observed
and fetched times, confidence, and detail. Stale, future-dated, or missing
evidence resolves to `Unknown` and requires a pregame refresh. `injury-plan`
places fresh IR/LTIR evidence into strict IR first and fresh DTD/GTD/out evidence
into IR+; it is advisory and never mutates the fantasy platform.

`morning` evaluates the requested day at 07:00 in the saved league timezone,
combines the injury/IR plan and goalie command center with the persisted weekly
acquisition budget and the top five legal remaining-week add/drop alternatives,
then emits ordered actions. Confirmed healthy skaters with a game receive start
actions; stale, missing, DTD, GTD, or otherwise uncertain evidence receives a
conditional refresh action instead. The top positive pickup becomes a concrete
conditional add/drop action; if none improves usable starts or projected value,
the briefing recommends no transaction. A decision-bearing fingerprint excludes
generation time and warning prose. With `--material-only`, a repeated unchanged
briefing prints only the no-change line; JSON sets `suppressed_unchanged` while
retaining the complete typed briefing. No external lineup or roster is changed.
Goalies use their own evidence gate: only a fresh confirmed starter receives a
firm goalie-start action. Reported, estimated, stale, or missing evidence emits
a same-day refresh with workload probability clearly labeled. When minimum risk
or meaningful coverage gain warrants it, the briefing includes the best legal
confirmed-before-add stream and a second fallback if the first player is claimed
or remains unconfirmed. `--current-goalie-appearances` supplies completed weekly
appearances so midweek minimum advice does not project from zero.
The top five sleeper rows are embedded separately. A leading sleeper matching
the best weekly pickup produces a supporting-evidence action; a different
leader produces a watch action. Sleeper evidence never silently changes the
optimizer's add/drop value.
Omitting `--at` evaluates the reproducible 07:00 local baseline. Supplying an
RFC3339 `--at` time reevaluates status freshness and waiver usability at that
pregame instant; its local date must match `--date` when both are supplied.
The text briefing includes one goalie checkpoint line with the next refresh,
number due now, and next lock. The v3 JSON contract separates the real
`generated_at` timestamp from the decision-bearing `evaluated_at` timestamp, so
replays never filter a valid stream using wall-clock time. Confirmed same-day
streams rank ahead of unconfirmed higher-volume options; the latter remains an
explicit confirmation-gated fallback.
Confirmed starters and confirmed backups still receive a final safety check 30
minutes before game lock. Inside that window, the briefing emits a verify-now
action while retaining the current start/bench recommendation until newer
evidence supersedes it.
`morning-card` evaluates that identical pipeline and seals it into the two-page
UI-neutral `fantasy_morning_card.v1` document. Its Morning Skate contains the
action queue and legal lineup; its Insider contains pickup budget, goalie
checkpoints and evidence, injury refreshes, weekly moves, warnings, and
methodology. Renderers do not recompute lineup, acquisition, or goalie choices.
The goalie stream and weekly pickup surfaces cannot silently spend the same
last move twice. With one proactive acquisition remaining, different candidates
are rendered as choose-one alternatives. An identical candidate is deduplicated
into the goalie action with the weekly optimizer's drop and value evidence.
Each primary and fallback stream independently searches all ranked weekly moves
for its legal drop/value pairing. Without one, the action is explicitly
capacity-gated and must not be executed unless an open roster spot is verified.

Goalie evidence is keyed by normalized player and NHL game date. Starter states
remain confirmed, reported, estimated, confirmed backup, reported backup, or
unknown; stale/future evidence resolves effectively to unknown. `goalie-plan`
uses the saved user roster, NHL schedule, active scoring scheme, daily goalie
slot count, and competition minimum. Expected appearances and the confirmed
floor are separate, and each row includes a poor-start points/SV%/GAA stress
case. Opponent offense is indexed from current-team skater goals/game relative
to the league average. Unsourced back-to-back starts receive a workload
discount, while sourced confirmation still wins. Free-agent goalies are ranked
by marginal usable appearances after daily slot collisions, waiver timing, and
the proactive move budget; the portfolio block compares keeping the current
group with the best conditional third-goalie add. Verified opponent shot
quality and richer multi-day goalie rest history remain follow-ons.

`goalie-start-import` atomically imports CSV evidence from a file or `--file -`.
Columns are `player,date,state,source,source_url,observed_at,detail`; source and
observation time may instead come from command fallbacks. Duplicate player/date
rows or any malformed row reject the entire paste. Goalie-plan rows carry NHL
game start, a 30-minute refresh deadline, minutes to lock, and check-later /
refresh-soon / refresh-now / locked urgency. Locked games contribute no
remaining appearance or stream value.

`goalie-start-template` emits the same CSV schema for rostered goalies playing
on the requested date and the top legal same-day stream candidates. Existing
reported state is retained so the file can be updated from morning news and fed
back to `goalie-start-import`. Plan JSON and text expose the next required
refresh, next game lock, number of checks due now, and unresolved rostered
goalies on the focus date. A newer observation always supersedes earlier
starter evidence, including a late confirmed-start to confirmed-backup reversal.

The fantasy command family keeps stable literal command names while its reports
use The Rink: draft and waiver work lives on **The Bench**, matchup plans meet in
the **Faceoff Circle** for a **Tale of the Tape**, and trades move to **The
Boards**. The **Trade Desk** evaluates an offer; the **Hot Stove** finds
plausible deals. Readiness, offers, and history retain those literal labels.

```bash
# Setup
icelines fantasy league-create "My League" --scheme yahoo-standard
icelines fantasy team-create "My Team" --owner "Gio"
icelines fantasy team-add "My Team" "McDavid"
icelines fantasy import-yahoo --file rosters.csv --league "My League" --dry-run
Get-Clipboard | icelines fantasy import-yahoo --file - --league "My League" --dry-run --replace
icelines fantasy import-yahoo --file rosters.csv --league "My League" --my-team "My Team"
icelines fantasy import-yahoo --file rosters.csv --league "My League" --dry-run --replace
icelines fantasy import-yahoo --file rosters.csv --league "My League" --replace
icelines fantasy roster-shape
icelines fantasy roster-shape-set yahoo-standard --league "My League"
icelines fantasy roster-shape-validate --team "My Team" --json
icelines fantasy assistant-rules
icelines fantasy assistant-setup --league "My League"
icelines fantasy draft-board --taken-file taken.txt

# Manage
icelines fantasy team-show "My Team"
icelines fantasy standings
icelines fantasy daily --date 2026-01-15 --json
icelines fantasy roster-card --date 2026-10-08
icelines fantasy roster-card --date 2026-10-08 --classes 8 --json
icelines fantasy draft-card --taken-file taken.txt
Get-Clipboard | icelines fantasy draft-card --taken-file - --top 8 --json
icelines fantasy matchup-set --week 2026-01-15 --home "My Team" --away "Rival"
icelines fantasy matchup --date 2026-01-15 --json
icelines fantasy matchup-plan --week 2026-10-05 --strategy balanced
icelines fantasy matchup-plan --week 2026-10-05 --team "My Team" --opponent "Rival" --strategy upside --json
icelines fantasy matchup-plan --week 2026-10-05 --through 2026-10-07 --user-current 42.5 --opponent-current 39 --current-source "Yahoo matchup page"
icelines fantasy competition-show --json
icelines fantasy competition-set --mode categories --category goals:higher:sum --category goals_against_average:lower:ratio:0.001 --minimum-goalie-appearances 3
icelines fantasy competition-set --mode points
icelines fantasy matchup-plan --week 2026-10-05 --category-snapshot examples/fantasy-category-snapshot.json
Get-Clipboard | icelines fantasy matchup-plan --week 2026-10-05 --category-snapshot -
icelines fantasy playoff-portfolio --rounds 3
icelines fantasy playoff-portfolio --start 2027-03-15 --rounds 3
icelines fantasy playoff-portfolio --team "Dexter's Dawgs" --season 20262027 --candidates 25 --top 10 --json
icelines fantasy playoff-calendar-set --start 2027-03-15 --rounds 3
icelines fantasy league-list
icelines fantasy league-switch "My League"

# Trades
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --stats-season 20252026
icelines fantasy trade-card "Bouchard" --to-team "Other" --for-player "Werenski"
icelines fantasy trade-card "McDavid,Bouchard" --to-team "Other" --for-player "MacKinnon,Werenski" --json
icelines fantasy trade "McDavid,Bouchard" --to-team "Other" --for-player "MacKinnon,Werenski" --stats-season 20252026 --json
icelines fantasy trade "McDavid,Bouchard" --to-team "Other" --for-player "MacKinnon,Werenski" --execute
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --execute
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --save-offer
icelines fantasy trade-offers --status pending
icelines fantasy trade-offers --status pending --actionable-only
icelines fantasy trade-offers --json
icelines fantasy trade-offer-close OFFER_ID --status accepted
icelines fantasy trade-history --limit 20
icelines fantasy trade-history --json
icelines fantasy trade-finder --team "Dexter's Dawgs" --stats-season 20252026 --top 20
icelines fantasy trade-finder --to-team "Other" --max-package 2 --fairness-percent 8 --json
icelines fantasy trade-finder --protect "McDavid,Kucherov" --top 20
icelines fantasy trade-finder --include-anchors --to-team "Other"
icelines fantasy trade-readiness --league "My League"
icelines fantasy trade-readiness --team "Dexter's Dawgs" --json
icelines fantasy trade-finder --require-complete --top 20
icelines tui team-card TRADE # sealed trade board; also `:trade-card`

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

`fantasy matchup-plan` emits `fantasy_matchup_strategy.v1`. It scores
completed-season per-game rates with the active league scheme, assigns both
saved rosters legally on every projected date, and reports expected/floor/upside
points, modeled win probability, usable starts, and value lost to bench
collisions. For an in-progress matchup, supply `--through`, `--user-current`,
and `--opponent-current` together. Those platform totals remain fixed and only
later dates are projected, so elapsed games are never counted twice.
`--current-source` labels their authority.

`fantasy playoff-portfolio` emits `fantasy_playoff_portfolio.v1`. It treats the
final requested Monday-Sunday weeks of the NHL regular-season schedule as the
fantasy playoff rounds, then runs the saved roster through the same legal daily
assignment engine used by matchup planning. Scheduled games, usable starts,
quiet-slate starts, bench collisions, and projected usable value remain
separate. A positive rank delta identifies a player who rises versus the
roster's completed-season per-game value order because the playoff calendar
fits the actual roster. The disclosed portfolio score adds 0.25 per quiet-slate
start and subtracts 0.50 per bench collision; it does not predict injuries,
starting goalies, or future role changes.
Pass `--start` with the league's first-round Monday to override the final-weeks
default. The candidate section evaluates the highest-value unrostered pool
(bounded by `--candidates`) against every one-for-one drop and ranks the best
whole-roster playoff deltas; `--top` controls the returned recommendations.
`fantasy playoff-calendar-set` persists the first-round Monday and one-to-four
round count inside the active league's assistant rules. Portfolio runs inherit
those values; command-line `--start` or `--rounds` values override them for a
single non-mutating report. `fantasy assistant-rules` displays the saved
calendar and includes it in JSON.
When that calendar is configured, `fantasy draft-board` reruns legal daily
assignments for the top 100 completed-season candidates over the exact playoff
dates. `playoff_fit_value` is exposed separately in each candidate's component
breakdown and capped at +12/-8 points, so schedule fit can break close calls but
cannot silently replace league-scored quality, starter gaps, or scarcity.

Fresh saved non-healthy status observations affect lineup eligibility during
the current matchup window. Missing, stale, or future-week status evidence is
not presented as confirmed health; the projection discloses its availability
assumption and asks for a pregame refresh. The best current legal one-move
pickup swing is included when the weekly optimizer can produce one. The 80%
bands use disclosed skater/goalie volatility proxies, and the probability is a
deterministic stress estimate—not betting odds.

`fantasy competition-set` persists the league's competition mode separately
from its points scheme. A category specification is
`KEY:DIRECTION:AGGREGATION[:TIE_EPSILON]`; direction is `higher` or `lower`,
and aggregation is `sum` or `ratio`. Supported skater keys are `goals`,
`assists`, `points`, `plus_minus`, `shots`, `hits`, `blocks`, `pp_goals`,
`pp_assists`, `sh_goals`, `sh_assists`, `gwg`, `ot_goals`, `takeaways`, and
`giveaways`. Supported goalie keys are `wins`, `losses`, `saves`,
`goals_against`, `shutouts`, `save_percentage`, and
`goals_against_average`. The last two require `ratio`; every other supported
key requires `sum`.
If the saved tie policy is `higher_seed_wins`, pass
`--user-higher-seed true` or `--user-higher-seed false` to `matchup-plan` so
the projected matchup result can apply the rule without guessing seed order.

When the saved league is in category mode, `fantasy matchup-plan` emits
`fantasy_category_matchup.v1`. It projects legal daily assignments, category
W-T-L, per-category win/tie/loss probabilities, safe/press/volatile/low-return
classification, and expected goalie appearances against the saved minimum.
Ratio categories sum their numerator and denominator before division.

For an in-progress category matchup, pass `--category-snapshot FILE` or pipe
pasted JSON with `--category-snapshot -`. The document uses
`fantasy_category_snapshot.v1`; see
`examples/fantasy-category-snapshot.json`. It must include the source,
`through_date`, both goalie-appearance totals, and exactly one row for every
configured category. Counting categories store the observed value in
`numerator` with a zero `denominator`. `save_percentage` uses saves and shots
against; `goals_against_average` uses goals against and goalie hours. IceLines
fixes those components as observed history and projects only later dates. JSON
and text output expose current + remaining = final values. Confirmed starting
goalies remain a later Wave 14 input and are not fabricated.

`fantasy import-yahoo` accepts Yahoo roster CSV exports with a player column
(`Player`, `Name`, `Player Name`, or `First Name` + `Last Name`) and a fantasy
team column (`Fantasy Team`, `Team Name`, `Rostered By`, `Owner Team`, or
`Manager Team`). Optional `Owner`, `NHL Team`, and `Eligible Positions` columns
are diagnostic context only. Use `--dry-run` first to preview created/updated
teams, imported/skipped players, unresolved names, duplicate ownership, and
header problems; rerun without `--dry-run` to apply local FantasyDb membership.
Yahoo stats are ignored and never become player/stat/photo truth.
Pass `--file -` to read the CSV from stdin; in PowerShell, `Get-Clipboard |`
provides a fast pre-draft or pre-trade synchronization path.
Imports are additive by default. For a complete current export, preview with
`--dry-run --replace`, fix every diagnostic, then apply with `--replace` to make
each included team's saved roster exactly match the CSV. Replacement is refused
when any row is skipped, unresolved, duplicated, or invalid; accepted changes
are committed atomically across the included rosters.

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

Built-in schemes: `yahoo-standard`, `espn-standard`, `simple-pts`,
`dexters-dawgs`.

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
icelines fetch ahl --season 20262027 --out data/ahl/roster-stats.json
icelines fetch ahl --season 20252026 --team HFD --team CV \
  --out data/ahl/nyr-sea-2025-26.json
icelines fetch contracts --source csv --input examples/contracts-young-stars-20262027.csv \
  --valuation-season 20262027 --cap-limit 104000000
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

`fetch ahl` resolves IceLines' eight-digit season against the official AHL
season catalog, discovers the provider's team IDs, and ingests the official
season-roster, skater-stat, and goalie-stat reports into
`ahl_roster_stats.v1`. With no `--team` filter it fetches every provider team;
repeat `--team` with an AHL code or exact name for a smaller snapshot. AHL
HockeyTech player IDs are serialized only as `provider_player_id`: they are
not NHL IDs and require an explicit identity crosswalk. The current season can
attach the dated NHL/AHL affiliation catalog; historical rows remain
league-neutral until a historical affiliation catalog is supplied.
Mixed other-team rows in a provider-filtered report and goalie scoring rows in
the skater report are excluded from typed team stats with reasons retained in
`source_warnings`; a report containing only other-team players fails closed.
Compatible duplicate roster rows that differ only by jersey history or forward
position collapse to one player with the ambiguous number omitted and forward
side generalized to `F`; those changes are audited and identity conflicts still
fail closed.
All source responses pass through verified FLETCH cachelines; the canonical
typed result is sealed at `<snapshot>/ahl/ahl-roster-stats.json`. `--out` is an
optional additional export, and `--refresh` forces source revalidation.
Filtered `--team` fetches use a team-code suffix in the snapshot name, so a
scoped side-fetch cannot overwrite the same-day full-league AHL snapshot.

Roster fetches seal `_official-roster-capture.json` with the observation time
and exact official NHL API URL for every season team. IceCast will not treat a
plain local roster snapshot as authoritative opening evidence, even when it is
sealed and contains all 32 teams.

Local contract overlays use this header:

```csv
nhl_id,player,team,season,cap_hit,aav,salary,expiry_year,expiry_type,source_url,checked_at
```

`nhl_id`, `player`, `team`, `season`, `source_url`, and `checked_at` are
required. `checked_at` must be RFC 3339 and `source_url` must be an absolute
HTTP(S) URL. At least one of `cap_hit`, `aav`, or `salary` must be present.
Rows for other valid seasons are ignored, while duplicate players, unknown NHL
IDs, malformed provenance, or an empty selected season fail before a snapshot
is created. The included young-stars example contains only confirmed values;
unsigned players are intentionally absent.

The web Admin page also exposes scoped data install/remove forms. Web install
writes only embedded bundled seasons to `~/.icelines/seasons/<season>/bundle-<season>`
after exact `INSTALL <season>` confirmation; it does not fetch live source data.
Web remove deletes only `~/.icelines/seasons/<season>` after exact
`REMOVE <season>` confirmation.

The bundled-data cap is 38 seasons because `BUNDLED_SEASONS` is the canonical source. The 2004-05 lockout has no data and never will.

---

## TUI (`icelines tui` or `icelines dashboard`)

```powershell
icelines tui team-card NYR  # sealed IceCast card; also `:team-card NYR` in the command bar
icelines tui team-card DEX  # sealed Dexter's Dawgs roster card; `p` switches to The Insider
icelines tui team-card DRAFT # sealed fantasy draft board; also `:draft-card`
icelines tui team-card MORNING # sealed Morning Skate; also `:morning-card`
```

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
Scoring room, Team room, The Bench, and Admin room. Each preset swaps the
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
| `team-card [NYR\|SEA\|DEX\|DRAFT\|MORNING\|TRADE]` / `draft-card` / `morning-card` / `trade-card` | Open a sealed UI-neutral card (`p` page; NHL cards: `t` team, `c` compare) | `:trade-card` |
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
| `mates player <name>` | Show roster-fallback linemate CLI target; shifts stay locked off | `:mates player Connor McDavid` |
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
fantasy roster-shape set yahoo-standard    -> not GET-backed; use `icelines fantasy roster-shape-set`
fantasy import file=rosters.csv league=Office
                                           -> not GET-backed; use `icelines fantasy import-yahoo --dry-run`
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
watch deployment TOR                       -> deferred; use CLI preview or `/watchlist` player rules
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
enable/disable, and web delete only. Arbitrary team/deployment editing is a CLI
preview/save path (`icelines watch deployment ... --save`) until the shared
mutation intent carries validated team/deployment dimensions.

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

If `~/.icelines/config.toml` already exists, `icelines setup` exits
without changing it unless `--reset` is passed. Reset rewrites the
`[sync]` settings only; other config keys are preserved. `--dry-run`
always previews without writing.

On the first interactive terminal run with no config file, `icelines`
opens this setup wizard before dispatching the requested command.
Top-level `--no-setup` skips that auto-prompt. Non-interactive stdin or
stdout never auto-prompts; scripted callers can run
`icelines setup --accept-defaults` explicitly.

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
| `sync.capabilities.shifts`           | `off`       | **Locked** — only `off` valid until a supported shift source, bundle, fetch, fixture, and join policy exists |
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

# Machine-readable freshness diagnostics
icelines data-status --json
```

Recognized `--shard` values: `bios`, `stats`, `goalie_stats`,
`transactions`, `boxscore`, `play_by_play`, `career_history`, `schedule`,
`score`, `playoff_bracket`. Source labels: Bundle / Setup / Live /
DataInstall / Manual.

`data-status --json`, `/admin`, and `/api/v1/admin/data-status` include shared
authority notes for optional advanced sources. The MoneyPuck skater snapshot
note names the same covered xG/CF/FF metrics as `/leaders`, records blocked
goalie/high-danger/zone-entry/deployment claims, and states that missing
snapshot values stay absent rather than zero.

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

## AHL preseason evidence workboard

```bash
# Compose the complete league rollover and professional-game evidence without
# inventing assignments, prospect labels, recall readiness, or waiver clearance.
icelines icecast affiliate-facts-board \
  --rollover league-rollover.json \
  --professional-games professional-games.json \
  --json --out affiliate-facts-board.json

# Create the exact candidate review envelope; edit sourced fields, reviewer,
# timestamp, and draft status before application.
icelines icecast affiliate-facts-draft \
  --workboard affiliate-facts-board.json \
  --out affiliate-facts-overlay-draft.json

# Apply only finalized facts bound to that exact workboard fingerprint.
icelines icecast affiliate-facts-apply \
  --workboard affiliate-facts-board.json \
  --overlay affiliate-facts-overlay-final.json \
  --json --out affiliate-facts-application.json

# Lower only complete teams through the canonical AHL lineup optimizer.
icelines icecast affiliate-inputs-league \
  --application affiliate-facts-application.json \
  --rule ahl-development-rule-final.json \
  --json --out affiliate-inputs-league.json
```

The JSON artifact is keyed by canonical NHL player ID where identity is
available, preserves exact eligible positions, and lists every remaining
authority blocker by player and team. Text output is a compact 32-team review
queue. A provisional professional-game policy can populate raw totals but
cannot certify final AHL development-rule qualification.

Overlay rows are partial by design. Omitted values stay blocked; `false` is an
explicit reviewed value for prospect status or assignment, not a synonym for
missing. Conflicts with sealed position/score facts, duplicate player rows,
non-HTTP evidence, invalid readiness, stale fingerprints, and draft overlays
fail before output.

League input lowering also requires `professional_game_policy_authority=final`,
a matching 260-game threshold, and explicit 18-dressed/12-development rule
authority. Every emitted team has already passed the canonical 12F/6D/2G and
development-rule projection builder. Incomplete teams remain named failures in
the league document.

`fetch career --league-crosswalk ...` stores official NHL landing birth dates
and primary positions beside career stints. The position is a fallback for a
generic AHL `F` row only; it is not fantasy eligibility or assignment evidence.

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
