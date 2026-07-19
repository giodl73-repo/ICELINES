# IceLines — NHL Analytics Platform

NHL depth charts, pace-adjusted rankings, query engine, fantasy league management, and 38 seasons of history — all from a single Rust CLI with **every season from 1987-88 to 2025-26 bundled in**, no fetch required.

IceLines is organized as **The Rink**: Center Ice for the league, the Red Line
for offense, the Blue Line for defense, the Crease for goalies, the Bench for
fantasy decisions, the Penalty Box for availability/constraints, and the Goal
Line for possible outcomes. The emerging product language reserves the Ice
family (`IceScout`, `IceBench`, `IceTrade`, `IceCast`, and `IceReplay`), while
**The Insider** is the evidence-aware voice across every area. See
[The Rink brand architecture](design/specs/brand-the-rink.md). Existing commands
and JSON contracts remain compatible as this language reaches the surfaces.

**[→ View the site](https://giodl73-repo.github.io/ICELINES/)**

**Review roles:** This repo uses
[ROLES](https://github.com/giodl73-repo/ROLES), the `.roles` convention for
repository-local review panels.

**Specification baseline:** `docs/vtrace/` is the governing project baseline for
mission, requirements, design, interfaces, verification, validation, work
packages, and change control. The older root and design docs are supporting
operator/developer references and should not override the VTRACE baseline.

---

## Download (no coding required)

**[→ Download the latest release](https://github.com/giodl73-repo/ICELINES/releases/latest)**

1. Click the link above and download the file for your platform:
   - Windows → `icelines-windows-x86_64.zip`
   - Mac (Apple Silicon) → `icelines-macos-arm64.tar.gz`
   - Mac (Intel) → `icelines-macos-x86_64.tar.gz`
   - Linux → `icelines-linux-x86_64.tar.gz`
2. Optional: download the matching `.sha256` file and verify the archive hash
   before extracting. Release archives also contain `ICELINES-PACKAGE.txt` with
   the source commit, build timestamp, and binary SHA-256.
3. Extract the archive — you get a single `icelines` (or `icelines.exe`) file
4. Open a terminal in that folder and run:

```bash
icelines fetch all        # download current NHL data (~5 seconds)
icelines tui              # launch the full interactive app
icelines menu             # don't know which surface you want? Pick from a menu.
icelines stathead         # browse curated query starter packs

# Or boot directly on a specific surface:
icelines tui scores               # tonight's games
icelines tui goalies              # goalie leaderboard
icelines tui poach                # fantasy poacher board
icelines tui watchlist            # fantasy poacher watchlist
icelines tui player Bedard        # Bedard's card cold
icelines tui team EDM             # Edmonton depth chart
```

Inside `icelines tui`, the default MDI mode is a shared composable workbench: an
activity catalog rail, a center workspace, swappable left/right context panes, a
scores ribbon, bound experience presets, active field summaries, and a bottom
command bar. Use `Tab` / `Shift+Tab` to move focus across workbench zones; when a
side pane is focused, `←` / `→` cycles native panes plus compact summaries for
the shared inspector pane catalog. Fantasy views
accept the same product grammar as the CLI: `gaps cats=hits,blocks,shots top=8`,
`poach rw cats=hits,blocks free top=12`, and
`simulate add=Connor_McDavid drop=Bench_Forward weeks=3`.

That's it. **38 seasons of NHL data** ship inside the binary — Gretzky's first LA year through this morning. No database setup, no accounts.

For the complete command reference, run `icelines docs` (or read [COMMANDS.md](COMMANDS.md)).

IceLines now uses a shared Prince of Wales visual system across the major
surfaces: TUI scan-rhythm contracts, web route layout classes, and 80-column
no-color CLI readability fences for representative outputs.

---

## Build from source

```bash
git clone https://github.com/giodl73-repo/ICELINES.git
cd ICELINES
cargo build --release
```

Works immediately — **all 38 seasons of NHL data** ship inside the binary:

```bash
icelines rank --top 10
icelines team EDM
icelines query leaders --pos C --age-max 23 --sort ppg --top 15
icelines query player "Connor McDavid" --percentiles
icelines tui
```

Release smoke for maintainers:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-release-artifact.ps1 -ArtifactPath dist\release\icelines-windows-x86_64.zip
```

Full release checklist: [design/release-checklist.md](design/release-checklist.md).

---

## Commands

### Query engine

```bash
# Leaderboard — 30+ sort metrics, all filters combinable
icelines query leaders --pos C --age-max 23 --sort ppg --top 15
icelines query leaders --draft-year 2022 --sort pts-pace --top 20
icelines query leaders --nationality FIN --sort ppg
icelines query leaders --sort pp-pts-pace --gp-min 40 --top 15   # PP specialists
icelines query leaders --sort improvement --pos F --gp-min 40    # Y/Y breakout leaders
icelines query leaders --sort hits-pace --top 15                 # Physical play
icelines query leaders --sort xgf-pct --top 15                   # Possession (MoneyPuck)
icelines query leaders --seasons 3 --pos C --sort pts-pace       # 3-season aggregate
icelines query leaders --undrafted --ppg-min 0.60                # Undrafted gems
icelines query leaders --rookie --sort ppg --top 15              # Rookie of Year race

# Player profile — career arc, percentile rank, all stats
icelines query player "Macklin Celebrini" --percentiles
icelines query player "McDavid" --breakdown career

# Comparison — head-to-head or similarity search
icelines query compare "McDavid" "MacKinnon"
icelines query compare "Matty Beniers" --similar 8   # finds historical comps
icelines query compare "Wayne Gretzky" "Mario Lemieux" --seasons 38   # full-history side-by-side

# Player profile — career arc + multi-season window
icelines query player "McDavid" --seasons 38 --percentiles   # full bundled history
icelines query player "Patrick Roy" --season 19951996        # historical goalies work too
icelines query player "Wayne Gretzky"                        # historical name resolves without --season

# Cross-league career cohorts — requires local career-history store
icelines fetch career --bundled-seasons 5
icelines query career --league OHL --season 20142015 --top 20
```

ICELINES keeps this hockey query UX and IR. SLICE examples are limited to simple
prepared row predicates and SQLite fold plans for ICELINES-owned joins; see
[`design/specs/slice-selectors.md`](design/specs/slice-selectors.md).

### Sort metrics

| Category | Metrics |
|----------|---------|
| Points | `pts-pace` (default), `ppg`, `pts`, `goals`, `assists`, `gp` |
| Goals | `g-pace`, `gpg` |
| Power play | `pp-pts-pace`, `pp-g-pace`, `pp-pts`, `pp-g` |
| Shorthanded | `sh-g-pace`, `sh-g` |
| Other scoring | `gwg-pace`, `gwg`, `shots-pace`, `shots` |
| Rates | `sh-pct`, `plus-minus`, `toi`, `fo-pct` |
| Physical | `hits-pace`, `hits`, `blocks-pace`, `blocks`, `takeaways`, `giveaways`, `pim` |
| Advanced | `xg`, `xg-per-60`, `cf-pct`, `ff-pct`, `xgf-pct` *(requires `fetch money-puck`)* |
| Trend | `improvement` — Y/Y PPG delta vs prior season |

### Filter flags (all combine with AND logic)

```bash
--pos C|LW|RW|D|F|G     # position (F = all forwards)
--team EDM               # team abbreviation
--age-min / --age-max    # age range (uses CURRENT age, not age-at-season)
--nationality FIN        # ISO-3166 alpha-3 (FIN, SWE, CAN, ...)
--birth-province ON,QC   # province/state codes, comma-separated
--draft-year 2022        # draft year
--draft-round 1          # draft round (1–7)
--draft-pick-max 30      # top-30 picks only
--undrafted              # only undrafted players
--rookie                 # only first NHL season
--handedness L|R         # shooting hand
--ppg-min 0.80           # minimum PPG (per game scale, e.g. 0.80)
--gp-min 40              # minimum games played
--gp-max 30              # maximum games played
--toi-min 18.5           # minimum TOI/game (minutes)
--plus-minus-min 5       # minimum +/-
--seasons N              # aggregate across last N bundled seasons (1–38)
--ufa / --rfa / --elc    # contract status (requires fetch contracts)
--expiry-year 2026       # contracts expiring this year
```

### Catalog filter grammar (`--filter` — boolean expressions over 108 stats)

Beyond the pre-baked flags above, **any of the 108 catalog stats** is filterable through the generic `--filter` grammar. Each `--filter` value is a full boolean expression with **AND / OR / NOT / parens**, and multiple `--filter` flags are ANDed at the top level.

```bash
# OR — either threshold qualifies
icelines query leaders --filter "g>=50 OR a>=80"

# Parens — group / override precedence (NOT > AND > OR)
icelines query leaders --filter "(g>=30 AND a>=30) OR p>=80"

# NOT — invert
icelines query leaders --filter "NOT pim>=100" --filter "p>=70"
```

```bash
# Young power forward — the canonical multi-filter pattern
icelines query leaders --age-max 24 --filter "hits>=200" --filter "points>=40"

# Clean scorer — high points, low penalties
icelines query leaders --filter "p>=50" --filter "pim<=30"

# Disciplined grinder — high hits, low PIM
icelines query leaders --filter "hits>=200" --filter "pim<=40"

# 3-season aggregate of the user pattern
icelines query leaders --seasons 3 --age-max 25 --filter "hits>=600" --filter "p>=120"

# Operators: >=, <=, >, <, ==
icelines query leaders --filter "g==50"            # exactly 50 goals
icelines query leaders --filter "shooting-pct>=0.18" --filter "shots>=200"
```

**Short aliases** — the filter parser accepts both the canonical `cli_key` and short forms users naturally type:

| Short | Canonical | Short | Canonical |
|---|---|---|---|
| `g` | `goals` | `gp` | `games` |
| `a` | `assists` | `ppg` | `points-per-game` |
| `p`, `pts` | `points` | `gpg` | `goals-per-game` |
| `s`, `sog` | `shots` | `apg` | `assists-per-game` |
| `pen` | `pim` | `pace` | `pace-82` |
| `+/-` | `plus-minus` | `sv%`, `sv` | `save-pct`, `saves` |
| `blk`, `blocks` | `blocked-shots` | `w`, `l`, `so` | `wins`, `losses`, `shutouts` |
| `tk` | `takeaways` | `ga`, `sa` | `goals-against`, `shots-against` |
| `gv` | `giveaways` | | |
| `mis` | `missed-shots` | | |

Filter keys are also case-insensitive: `--filter "HITS>=200"` resolves to Hits.

`age` is **not** a catalog stat — use the `--age-min` / `--age-max` flags above.

### Team depth charts

```bash
icelines team SEA        # Seattle Kraken — 4×3 forward grid, 3×2 defense pairs
icelines team EDM        # Edmonton Oilers
icelines team-season EDM # season record, standings context, SOS, quality ledger
icelines report team-ceiling                 # all-team 2026-27 ceiling + YoY delta
icelines report team-ceiling --team NYR      # Rangers lenses, changes, chance range
```

Players are color-coded by **cross-team fit** — how they'd rank on each of the other 31 teams:
- ★ **Elite** — true caliber for this slot on most rosters
- ~ **Solid** — fits their role
- ↑ **Buried** — underused, would play higher elsewhere
- ↓ **Stretch** — overextended in current role

`report team-ceiling` combines the current official roster with completed
2025-26 production. It publishes points-pace, goal-scoring,
fantasy/peripherals, and age-adjusted-upside lenses; a league-normalized
year-over-year delta; newcomer/departure lists; sample coverage; and a
heuristic playoff range. The range is an inspectable roster-strength scenario,
not a calibrated probability or betting line. JSON uses `team_ceiling.v1`.

### Player analysis

```bash
icelines history "Connor McDavid"          # season-by-season career stats
icelines project "Celebrini" --mode pace   # rest-of-season projection
icelines project "Bedard" --mode regressed # regression-weighted projection
icelines scouting "Evan Bouchard"          # full 8-section scouting report
icelines scouting "Bouchard" --format json # structured JSON output
icelines peers "Lane Hutson" --size 8      # draft class ± 1 year peers
icelines class 2022 --top 15              # full draft class ranked by production
icelines compare "McDavid" "MacKinnon"    # side-by-side stats comparison
icelines mates "Beniers" --top 5          # roster fallback; shift bundles parked
```

### Fantasy league

```bash
# Setup
icelines fantasy league-create "My League" --scheme yahoo-standard
icelines fantasy team-create "My Team" --owner "Gio"
icelines fantasy team-add "My Team" "McDavid"
icelines fantasy team-add "My Team" "Kucherov"
icelines fantasy import-yahoo --file rosters.csv --league "My League" --dry-run
Get-Clipboard | icelines fantasy import-yahoo --file - --league "My League" --dry-run --replace
icelines fantasy import-yahoo --file rosters.csv --league "My League" --my-team "My Team"
icelines fantasy import-yahoo --file rosters.csv --league "My League" --dry-run --replace # preview exact sync
icelines fantasy import-yahoo --file rosters.csv --league "My League" --replace # remove stale memberships
icelines fantasy roster-shape
icelines fantasy roster-shape-set yahoo-standard --league "My League"
icelines fantasy roster-shape-validate --team "My Team" --json
icelines fantasy assistant-rules                     # preview saved/default assistant contract
icelines fantasy assistant-setup                     # persist this league's 2026-27 rules
Get-Clipboard | icelines fantasy draft-board --taken-file - --top 12
icelines fantasy draft-board --taken-file taken.csv --json
icelines fantasy draft-board --eligibility-file yahoo-player-pool.csv
icelines fantasy draft-board --pick "Connor McDavid" # dry-run the following pick
icelines fantasy weekly-budget --json                 # Monday-Sunday add budget
icelines fantasy weekly-pickups --date 2026-10-08 --top 15
icelines fantasy sleepers --positions D --top 20
icelines fantasy acquisition-record --add "Player" --drop "Bench Player"
icelines fantasy status-record "Player" --status dtd --source "league app"
icelines fantasy status-show --json
icelines fantasy goalie-start-record "Igor Shesterkin" --date 2026-11-12 --state confirmed-starting --source "team reporter"
Get-Clipboard | icelines fantasy goalie-start-import --file - --source "daily goalie report"
icelines fantasy goalie-start-template --date 2026-11-12 --out goalie-news.csv
icelines fantasy goalie-start-show --week 2026-11-09
icelines fantasy goalie-plan --week 2026-11-09 --strategy balanced
icelines fantasy injury-plan --date 2026-10-08
icelines fantasy morning --date 2026-10-08
icelines fantasy morning --date 2026-10-08 --at 2026-10-08T17:30:00-07:00
icelines fantasy morning --date 2026-10-08 --current-goalie-appearances 2
icelines fantasy morning --material-only --json

# Manage
icelines fantasy team-show "My Team"       # roster with per-player fantasy scores
icelines fantasy standings                 # league standings
icelines fantasy league-switch "My League" # switch active league
icelines fantasy league-scheme-set dexters-dawgs --league "My League"
icelines fantasy team-use "My Team"        # mark your roster for gaps/poach
icelines fantasy gaps --category hits,blocks,shots
icelines fantasy simulate --weeks 4
icelines fantasy simulate --add "McDavid" --drop "Bouchard" --json
icelines fantasy season-sim --league "My League" --trials 100 # injuries, pickups, trades
icelines fantasy season-sim --league "My League" --team "My Team" # lock partial/full roster
icelines fantasy season-sim --injury-rate 0.003 --trade-probability 0.50 --json
icelines fantasy season-sim --scenario-matrix            # clean/baseline/high-chaos
icelines fantasy season-sim --opponent-pickup-accuracy 0.70 # model manager edge
icelines fantasy season-sim --manager-matrix             # parity/85%/70%
icelines fantasy season-sim --pickup-reserve 1           # save one add for injuries
icelines fantasy season-sim --reserve-matrix              # all-in/strict/adaptive
icelines fantasy schedule-edge --refresh              # load full official season schedule
icelines fantasy schedule-edge --week 2026-10-05      # Monday-Sunday volume/off-night leaders
icelines fantasy schedule-edge --teams NYR,COL,EDM    # draft-calendar fit before a roster exists
icelines fantasy schedule-edge --json --out schedule-edge.json
icelines fantasy daily --date 2026-01-15 --json # cached finalized boxscores only
icelines fantasy matchup-set --week 2026-01-15 --home "My Team" --away "Rival"
icelines fantasy matchup --date 2026-01-15 --json # weekly head-to-head, cache-backed
icelines fantasy matchup-plan --week 2026-10-05 --strategy balanced
icelines fantasy matchup-plan --week 2026-10-05 --team "My Team" --opponent "Rival" --strategy floor --json
icelines fantasy matchup-plan --week 2026-10-05 --through 2026-10-07 --user-current 42.5 --opponent-current 39 --current-source "Yahoo matchup page"
icelines fantasy competition-show --json
icelines fantasy competition-set --mode categories --category goals:higher:sum --category save_percentage:higher:ratio:0.0001 --minimum-goalie-appearances 3
icelines fantasy matchup-plan --week 2026-10-05 --category-snapshot examples/fantasy-category-snapshot.json

# Same fantasy workflow in the TUI command bar:
icelines tui
# :gaps cats=hits,blocks,shots top=8
# :poach rw cats=hits,blocks free top=12
# :simulate add=Connor_McDavid drop=Bench_Forward weeks=3
# :fantasy daily date=2026-01-15
# :fantasy matchup date=2026-01-15
# :fantasy import file=rosters.csv league My_League dry-run
# :fantasy roster-shape validate team My_Team

# Poacher
icelines poach --category hits,blocks --top 15
icelines poach --availability imported-available --category hits,blocks --top 15
icelines poach --team SEA --pos LW --json
icelines report list
icelines report cap-forecast --team NYR
icelines report cap-forecast --years 5 --growth-pct 5 --json
icelines report poach --category shots --top 10 --out poach.md
icelines report weekly --league default --category hits,blocks
icelines records player "Andre Burakovsky" --metric teams-scored-against
icelines records player "Andre Burakovsky" --metric goalies-scored-against
icelines records player "Andre Burakovsky" --metric fight-opponents
icelines awards "Connor McDavid"
icelines streaks "Connor McDavid"
icelines watch rules
icelines watch player "Matthew Knies" --when pp1 --save
icelines watch disable player-matthew-knies
icelines watch fire player-matthew-knies --player "Matthew Knies" "PP1 usage crossed threshold"
icelines watch history
icelines watch list
icelines watch note "Matthew Knies" "PP1 promotion and strong hits fit"
icelines tui poach                     # press w to watch with a score/reason note
icelines serve                         # web includes /fantasy, /poach, reports, watchlist, and JSON APIs

# Trades
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --stats-season 20252026 # simulate
icelines fantasy trade "McDavid,Bouchard" --to-team "Other" --for-player "MacKinnon,Werenski" --json # package
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --execute # commit legal 1-for-1
icelines fantasy trade "McDavid,Bouchard" --to-team "Other" --for-player "MacKinnon,Werenski" --execute # atomic package
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --save-offer # no roster mutation
icelines fantasy trade-offers --status pending
icelines fantasy trade-offers --status pending --actionable-only
icelines fantasy trade-offer-close OFFER_ID --status accepted # status only; sync or execute separately
icelines fantasy trade-history --limit 20 # audit locally executed trades
icelines fantasy trade-finder --team "Dexter's Dawgs" --top 20 # generate fair, legal offers league-wide
icelines fantasy trade-finder --to-team "Other" --fairness-percent 8 --json # target one manager
icelines fantasy trade-finder --protect "McDavid,Kucherov" # keep named anchors out of every offer
icelines fantasy trade-readiness --league "My League" # prove every roster is actionable
icelines fantasy trade-finder --require-complete --top 20 # refuse provisional offers

# Web dashboard
icelines serve --port 8000
# GET /dashboard               -> shared workbench catalog, pane selectors, presets, command bar
# GET /dashboard?workspace=/scores&left=favorites-left&right=schedule-right&experience=tonight-bench
# GET /favorites?group=Prospects -> read-only web group view; Favorites group keeps add/remove forms
# Try: poach rw cats=hits,blocks free top=12
# Try: fantasy simulate add Connor_McDavid drop Bench_Forward
# Try: fantasy daily date=2026-01-15
# Try: fantasy matchup date=2026-01-15
# Try: fantasy roster-shape validate team=My_Team
# GET /fantasy                 -> HTML gaps + simulation scenarios
# GET /api/v1/fantasy/gaps     -> FantasyRosterGapView JSON
# GET /api/v1/fantasy/simulate -> FantasySimulationView JSON
# GET /api/v1/fantasy/daily?date=YYYY-MM-DD -> FantasyDailyDeltaView JSON
# GET /api/v1/fantasy/matchup?date=YYYY-MM-DD -> FantasyMatchupWeekView JSON
# GET /api/v1/fantasy/roster-shape?team=<name> -> RosterShapeValidationView JSON
# GET /poach                   -> HTML poacher board
# GET /player/:id/outlook      -> descriptive scoring pace, nullable finish
# GET /team/:abbrev/outlook    -> cached GF/GA pace and recent pressure
```

`fantasy schedule-edge` deduplicates all 32 official club schedules into one
season calendar, caches it under `~/.icelines/data/seasons/<season>/schedule.json`,
and emits Monday-Sunday team volume, quiet-slate games, scarcity scores, and
exact-date schedule equivalence classes. By default it resolves the marked
FantasyDb user roster against the current roster snapshot. Drafting from
different classes—and consulting the low-overlap complements—reduces same-night
bench pressure. A quiet slate defaults to four or fewer NHL games and can be
changed with `--off-night-max-games`.

`fantasy season-sim` stress-tests a full synthetic league without modifying the
saved league. `--team` locks the selected partial or complete roster and fills
only its open spots; otherwise the marked user team is used. Seeded trials use the selected scoring scheme, configured roster
and IR rules, exact daily schedule/eligibility matching, four-add weekly limits,
injury replacements and recoveries, pickups, and fair-value trades. The output
reports weekly head-to-head records, average seed, first-place probability,
playoff/championship rates, first-round, semifinal, and final exit rates, plus
average adds, trades, injuries, missed starts, and roster churn; it is a
scenario model rather than a calibrated forecast.
`--scenario-matrix` reuses the same seed, roster, schedule, scoring, and trial
count across clean (no injuries/trades), baseline (the supplied rates), and
high-chaos (at least 0.003 injury and 35% trade rates) environments. Its compact
table reports point delta from baseline, average seed, W-L-T, No. 1-seed,
playoff, and championship rates; JSON returns the three complete typed views.
Team one always chooses the highest projected legal weekly add.
`--opponent-pickup-accuracy` controls how often synthetic opponents do the same;
on a miss they choose the second- or third-ranked add. The default is `1.0`, so
manager advantage is never assumed silently and no opaque points multiplier is used.
`--manager-matrix` automates the paired comparison at 100%, 85%, and 70%
opponent accuracy. It cannot be combined with `--scenario-matrix`; point deltas
use the parity run as their reference.
Simulated pickups rank complete add/drop pairs and reject moves that would leave
the roster unable to fill every configured active position. Trades apply the
same per-team legality check after both sides of the swap.
Injured IR players are protected from synthetic drops and trades. If their
temporary replacement is later swapped, IceLines transfers replacement
ownership so recovery releases the current substitute instead of leaving an
illegal extra roster player.
Each simulated morning opens one rotating pickup opportunity per team. A team
may therefore react to midweek waivers and schedule changes, while one shared
Monday-Sunday counter still prevents proactive pickups plus injury replacements
from exceeding four in the week.
Dropped players and released IR substitutes enter the configured two-day waiver
queue. They cannot be recycled by another team in the same transaction window
and return to the candidate pool only on the exact clearance date.
Pickup scoring subtracts a three-game retention cost when the proposed drop has
a better league-scored per-game rate than the add. Comparable-player schedule
streams remain available, but a quiet week alone cannot make an elite player the
preferred drop.
Team one reserves one weekly acquisition from proactive streaming through Friday
by default, then releases it Saturday if unused. The reserved move remains
available for an injury replacement; use `--pickup-reserve 0` to compare an
all-in four-stream strategy.
Season output reports `IR blocked`, the average number of long-injury replacement
attempts prevented specifically by an exhausted weekly acquisition budget.
The live weekly budget and morning briefing use the same policy: `can_add`
reports the platform hard limit, while `can_proactively_add` protects one move
through Friday. A confirmed IR/IR+ substitution may override that protection;
Saturday releases an unused reserve automatically.
When the roster has no status awaiting pregame refresh, a move with at least
`+6.0` projected net value and `+3.0` usable starts can trigger an explicit
exceptional-value review. Day-to-day, game-time-decision, stale, or unknown
evidence disables that exception and keeps the injury reserve intact.
`season-sim --reserve-matrix` calibrates the policy with paired all-in, strict,
and adaptive runs. Simulation uses +3 seven-day scheduled games as its
deterministic proxy for the live assistant's +3 optimized usable starts.

`fantasy assistant-setup` persists the league contract used by the developing
draft and morning assistant: 2 C, 2 LW, 2 RW, 3 D, 1 skater UTIL, 2 G, four
unrestricted bench slots, two IR, two IR+, four Monday-Sunday acquisitions,
two-day waivers, same-day free agents, and daily lineup changes. Platform
eligibility is stored independently so C/LW, C/RW, and LW/RW players retain all
of their legal slots.

`fantasy draft-board` scores the available pool with the active league scheme
using completed 2025-26 statistics by default, then explains open-starter,
replacement-level, multi-position, usable-date, quiet-slate, and exact schedule
collision adjustments. It accepts newline/CSV taken lists, keeps ambiguous or
unresolved names visible, reads PowerShell clipboard input through
`--taken-file -`, and supports a non-mutating hypothetical `--pick`. Current
2026-27 roster snapshots supply NHL team labels; `--stats-season` changes the
completed performance window without changing the schedule season.
`--eligibility-file` accepts common player-name and position columns, safely
persists resolved C/LW-style eligibility for the selected league, and includes
duplicate, ambiguous, unresolved, and invalid row diagnostics in JSON.

In The Rink, this is **The Bench — War Room — Draft Board**. Weekly adds are
the **Waiver Wire**, sleepers are the **Call-Up Board**, and schedule fit is
**The Gauntlet**. The hockey names are report headings; the CLI commands remain
stable and searchable by their literal purpose.

`fantasy weekly-budget` reads the persisted acquisition ledger in the league's
configured timezone and enforces the four-add Monday-Sunday contract.
`fantasy acquisition-record` is an explicit local bookkeeping mutation for a
move already completed on the fantasy platform; it refuses a fifth counted add
and starts the dropped player's exact two-day waiver window. Use `--kind waiver`
for claims and `--no-count` only when the platform says that move is exempt.

`fantasy weekly-pickups` optimizes the remaining days through Sunday. It builds
the legal daily lineup before and after every evaluated add/drop, counts only
starts that fit an active slot, applies the league scoring scheme as a per-game
projection, filters active waivers, and charges the remaining pickup budget.
`--candidates` controls how many top available players enter the pair search.

`fantasy sleepers` searches the unrostered skater pool for rate changes that a
full-season leaderboard can hide. It compares active-league fantasy points per
game, shots/hits/blocks rates, and power-play production with the prior season,
then adds multi-position and 2026-27 quiet-slate value. The default comparison
is 2025-26 versus 2024-25; use `--positions D` for Raddysh-style defense finds.
Players under 10 games are excluded, small samples are discounted, incomplete
baseline joins earn neither growth nor newcomer credit, and goalie discovery is
explicitly deferred.

Matchup planning meets in the **Faceoff Circle** for a **Tale of the Tape**.
Trade analysis moves to **The Boards**, with the **Trade Desk** for evaluating
an offer and the **Hot Stove** for finding plausible deals.

Availability observations are explicitly sourced and time-bounded.
`fantasy status-record` stores a confirmed, reported, estimated, or unknown
observation; `status-show` resolves the newest evidence and marks stale, future,
or missing evidence for refresh. `injury-plan` uses only fresh effective status,
fills strict IR before IR+, and never treats `Unknown` as confirmed healthy.
Goalie starter evidence is game-specific and independently sourced through
`goalie-start-record`. `goalie-plan` combines that evidence with the saved
roster, daily two-goalie capacity, NHL schedule, league scoring, and the
category minimum. It separates expected appearances from the confirmed floor,
requests refreshes for missing or stale evidence, and discloses a poor-start
SV%/GAA and points stress case. Opponent offense is indexed from current-team
skater goals/game, back-to-backs reduce unsourced workload probability, and
legal free-agent streams are ranked by marginal usable starts after daily
collisions. The two-versus-three-goalie comparison also preserves the configured
injury pickup reserve. Richer rest history and verified opponent shot-quality
inputs remain follow-ons.
For a busy slate, `goalie-start-import` accepts CSV columns `player,date,state,
source,source_url,observed_at,detail` from a file or stdin. `--source` and
`--observed-at` provide row fallbacks. The entire paste validates before an
atomic insert, and duplicate player/date rows are rejected. Morning actions are
lock-aware: early unknowns say to check later, the final three hours escalate
the refresh, the final hour says check now, and a started game becomes locked
instead of remaining a fake actionable stream.
`goalie-start-template` writes that exact import schema for every rostered goalie
playing on the selected date plus the best legal same-day streams. It preserves
the latest reported state as an editable starting point. The goalie plan also
publishes the next evidence-refresh checkpoint, next game lock, refreshes due
now, and unresolved rostered-goalie count for the focus date.
`fantasy morning` combines that injury plan, the goalie command center, and the
Monday-Sunday acquisition budget in a deterministic 07:00 league-timezone
briefing. It orders IR/IR+ moves, same-day goalie and injury refreshes, confirmed
playable starts, conditional goalie streams with a fallback, and the top
positive legal add/drop from the remaining-week optimizer. A rostered goalie is
never given a firm start action from schedule/workload probability alone:
reported, estimated, stale, and missing starter evidence remain conditional
until a fresh same-day confirmation. The five best pickup
alternatives remain in JSON, and no add is recommended when every evaluated
move is neutral or harmful. `--material-only` compares a persisted decision fingerprint and
suppresses unchanged text output; JSON retains the full typed view and marks
`suppressed_unchanged`. It remains advisory and performs no platform moves.
The briefing prints one goalie checkpoint summary with the next evidence
refresh, checks due now, and the next lock. JSON separates `generated_at` from
`evaluated_at`; `--at` lock, evidence, and stream decisions always use the
requested evaluation instant. A confirmed same-day stream ranks ahead of a
higher-volume but unconfirmed option, which remains the fallback.
Fresh confirmation is not treated as permanent: confirmed starters and backups
receive a final T−30 safety checkpoint. Once that window opens, `morning` emits
an explicit verify-now action so a late injury, warmup change, or starter swap
can reverse the earlier lineup or add recommendation before lock.
Goalie streams and skater pickups share the same transaction decision. With one
proactive acquisition left, the briefing labels them as mutually exclusive
alternatives instead of implying both can be made. If both optimizers select the
same player, one combined action retains the proposed drop and projected value.
Primary and fallback goalie streams independently inherit any legal add/drop
pairing found by the weekly optimizer. If no pairing exists, the briefing says
to verify an open roster spot before execution instead of presenting a
roster-full add as complete advice.
The briefing also carries the top five sleeper signals. When the leading
sleeper is already the best weekly pickup, a separate action says the breakout
evidence supports that move; otherwise the strongest qualifying riser becomes
a watch action without altering the transaction ranking.
Use `--at <RFC3339>` before lineup lock to reevaluate evidence freshness,
waiver usability, and material changes at that exact pregame time; omitting it
keeps the reproducible 07:00 baseline.

Watch-rule editors are intentionally narrow: TUI/web can create player rules and
toggle persisted rules, and the web watchlist can delete persisted rules.
Team/deployment rule editing stays on the CLI preview/save path until the shared
mutation contract carries those dimensions.

Favorites and groups share the local SQLite `GroupDb`. Web `/favorites` can
select any group for read-only inspection, but web add/remove forms intentionally
target only the canonical `Favorites` group. Create, rename, delete, and arbitrary
group membership edits remain on `icelines group ...` and the TUI Groups surface.

**Fantasy schemes:** `yahoo-standard`, `espn-standard`, `simple-pts`,
`dexters-dawgs`. The last reproduces Dexter's Dawgs scoring: skater G 3.25,
A 2.25, PPG 3, PPA 2, SHG/SHA/GWG 1, HIT/BLK 0.5; goalie W 3, L -0.5,
GA -0.25, SV 0.2, and SHO 3.

Fantasy roster shapes are local league setup rules, separate from scoring
schemes. The default `yahoo-standard` shape validates active roster composition
by canonical NHL/bundled player positions; Yahoo CSV position hints remain
diagnostic only. Shape changes stay on the CLI, while the web dashboard exposes
read-only validation and rejects GET-backed mutation.

Yahoo roster CSV import is local setup only: it writes FantasyDb league/team
membership after a dry-run/apply diagnostic pass. NHL API/bundled data remains
authoritative for player identity, teams, stats, and photos; Yahoo stat columns
are ignored.

### Data and history

```bash
# Fetch fresh data (optional — bundled data works immediately)
icelines fetch all              # rosters + stats (~5 min)
icelines fetch realtime         # hits, blocks, giveaways, takeaways, PIM
icelines fetch money-puck       # xG, CF%, FF%, xGF% from MoneyPuck (free)
icelines fetch contracts        # UFA/RFA/ELC contract status
# Licensed salary values (API key stays in the environment; values carry provenance)
CAPWAGES_API_KEY=... icelines fetch contracts --source cap-wages \
  --valuation-season 20262027 --cap-limit 104000000
# Free local overlay: validates NHL IDs, provenance URLs, and timestamps
icelines fetch contracts --source csv \
  --input examples/contracts-young-stars-20262027.csv \
  --valuation-season 20262027 --cap-limit 104000000

# Historical seasons (1987-88 through 2025-26, excluding 2004-05)
icelines data install --season 19881989    # Gretzky's first LA season
icelines data install --seasons 5          # newest 5 (already bundled)
icelines data install --seasons 38         # full history 1987–2025
icelines data list                          # show installed seasons + player counts
icelines data remove 19921993              # uninstall a season

# Multi-season queries (bundled; installs are only needed for fresher overrides)
icelines query leaders --seasons 10 --pos C --sort pts-pace --top 10
icelines query leaders --seasons 5  --sort pts-pace --top 10
```

**38 seasons available** — back to 1987-88 (Gretzky trade to LA Kings). Skip 2004-05 (full lockout).

### TUI (`icelines tui` or `icelines dashboard`)

Interactive dashboard. By default `icelines tui` opens the shared composable MDI
workbench: activity/catalog rail, scores ribbon, swappable left/right context
panes, central workspace, bound experience presets, active field summaries, and
a command bar. `Tab` / `Shift+Tab` moves focus across workbench zones in default
MDI; with a side pane focused, `←` / `→` cycles the shared pane binding. The
TUI workbench includes Tonight bench, Scoring room, Team room, The Bench, and
Admin room presets that swap workspace plus context panes together; web-derived
inspectors render as compact field/command summaries when cycled in the TUI.
Launching directly into a bound workspace, such as `icelines tui stats` or
`icelines tui --start scores`, applies the matching room preset immediately and
keeps the activity rail selected on that workspace; on shorter terminals the
rail scrolls to keep that selected room visible. Admin, Docs, and Groups
workbench destinations use named chrome labels in the dashboard footer. Hiding a
focused side pane returns focus to the central workspace. Use
`--classic` for the older tabbed single-document UI or `--standalone` for a
locked one-screen surface. Player cards lazy-load every player's full
historical career across all 38 bundled seasons on first open. Cross-league
cohort boards use the canonical CLI/web surfaces instead; from the command bar,
`:career league=OHL season=20142015 top=8` flashes the exact `query career`
and `/career` targets. Player cards also have a dedicated Records screen:
press `r` from a player card, or run `:records player Andre Burakovsky` in the
MDI command bar. Press `a` from a player card for the cached Awards / Trophy
Case screen; populate it with `icelines awards "Connor McDavid"`. Press `s`
for cached goal/assist/point plus shot-on-goal/attempt streaks, or run
`:streaks player Connor McDavid`. The player card is the hub for records,
awards, streaks, scouting, compare, groups/favorites, and fantasy watch
handoffs.

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Default MDI: move focus across workbench rail / panes / workspace. `--classic`: cycle tabs forward / backward. `--standalone`: no-op. |
| `↑↓` | Navigate within the focused zone or screen |
| `←→` | With a side pane focused, cycle pane composition; otherwise screen-local navigation |
| `Enter` | Rail focused: open selected workbench entry; workspace focused: drill into selection (team / player / game) |
| `Esc` / `q` | Back / quit |
| `?` | Help overlay |
| `R` | **Reports overlay** — toggle which Tier-1 reports populate columns |
| `y` | Season picker — jump to any of the 38 bundled seasons |
| `Shift+P` | Toggle Regular ↔ Playoff for the active season |
| `o` | Toggle the current section on the Stats / Queries screen |
| `[` / `]` | Cycle career-table column presets on a player card |
| `/` | Open the sort picker (search-as-you-type across 108 stats) |
| `r` | Refresh the current view |
| `d` | Jump to depth chart (or jump-to-date on Scores) |
| `F` | Toggle admin overlay |
| `g` | Add to group from a player card / team roster |

The Reports overlay (`R`) persists toggles to `~/.icelines/config.toml`. Disabled reports drop their columns from career tables, sort pickers, and query results — your view stays focused on the stats you care about.

### Other commands

```bash
icelines tonight                    # tonight's NHL games (live API)
icelines tonight --team EDM         # filter to one team
icelines schedule --days 7          # upcoming schedule
icelines team-season EDM            # team season-performance view
icelines trade "Bouchard" for "Fox" --team EDM  # depth chart trade impact

icelines group create "Watchlist"   # player/team groups (SQLite-backed)
icelines group add "Watchlist" "McDavid"
icelines group add "Watchlist" EDM  # team abbrevs are stored as team members
icelines group show "Watchlist"

icelines scheme list                # fantasy scoring schemes
icelines scheme show yahoo-standard # show weights

icelines snapshot list              # data snapshots
icelines snapshot verify            # integrity check

icelines docs                       # print the offline command reference
icelines export md --help           # durable Markdown/report exports
icelines serve                      # launch the axum web dashboard/API
```

---

## Data sources

| Source | What | Command |
|--------|------|---------|
| NHL API (free, public, no key) | Stats, rosters, bios, realtime, schedule | `icelines fetch all` |
| MoneyPuck (free CSV) | xG, CF%, FF%, xGF% at 5v5 | `icelines fetch money-puck` |
| CapWages (licensed, API key required) | Salary, cap hit, AAV, expiry, team cap share | `icelines fetch contracts --source cap-wages` |
| Local contract CSV | User-curated cap hit/AAV/expiry with per-row provenance | `icelines fetch contracts --source csv --input PATH` |
| Bundled (in binary) | 38 seasons 19871988–20252026, excluding 20042005 | — (zero config) |
| GitHub Releases | Optional season refresh/install tarballs | `icelines data install` |

Bundled data is refreshed during release/data-prep work and ships with each release. `icelines rank` and `query leaders` work immediately after install with no fetch required.

---

## Architecture

```
icelines-core    pure domain types, filters, scheme scoring - no I/O
icelines-query   Art Ross query parser, planner, executor
icelines-fetch   NHL API client, snapshot store, bundled data, MoneyPuck
icelines-web     axum web/API surface
icelines-cli     thin UI layer - commands, TUI, HTTP server (axum)
icelines-site    deferred mkdocs/static-site generator; no active CLI entry point
```

6-crate Rust workspace. Scenario coverage now includes **2,000+ persona/harness tests** plus broad L0/L1/L2 integration, system, mock NHL API, TUI, query, and web gates. See `design/notes/2026-05-09-scenario-harness-inventory.md` for the current harness map.

Current product intent and evidence posture live in
[`docs/vtrace/MISSION.md`](docs/vtrace/MISSION.md),
[`docs/vtrace/REQUIREMENTS.md`](docs/vtrace/REQUIREMENTS.md), and
[`docs/vtrace/WORK_PACKAGES.md`](docs/vtrace/WORK_PACKAGES.md). Public feature
claims should align with `design/specs/surface-parity.md`.

---

## Tests

```bash
cargo test                    # full workspace tests: L0, L1, L2, mock API, persona waves
cargo clippy -- -D warnings   # must be clean
cargo fmt --check             # must be clean
cargo audit                   # dependency vulnerability gate
```

Windows-friendly slices:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 list
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-query        # Tests / query
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-cli-tui      # Tests / cli-tui-bin
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 scenarios       # TUI + CLI + query + web scenario harnesses
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-clippy       # Quality / clippy
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-audit        # Quality / dependency advisories
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 tui-snapshots   # app snapshot module only
```

`ci-audit` installs `cargo-audit --locked` when missing. Vulnerability advisories
block CI and local release gates; warning-class advisories stay visible in the
release checklist until their dependency path is removed.

---

## License

MIT — see [LICENSE](LICENSE).

Copyright (c) 2026 Gio Della-Libera
