# IceLines — NHL Analytics Platform

NHL depth charts, pace-adjusted rankings, query engine, fantasy league management, and 38 seasons of history — all from a single Rust CLI with **every season from 1987-88 to 2025-26 bundled in**, no fetch required.

**[→ View the site](https://giodl73-repo.github.io/ICELINES/)**

---

## Download (no coding required)

**[→ Download the latest release](https://github.com/giodl73-repo/ICELINES/releases/latest)**

1. Click the link above and download the file for your platform:
   - Windows → `icelines-windows-x86_64.zip`
   - Mac (Apple Silicon) → `icelines-macos-arm64.tar.gz`
   - Mac (Intel) → `icelines-macos-x86_64.tar.gz`
   - Linux → `icelines-linux-x86_64.tar.gz`
2. Extract the archive — you get a single `icelines` (or `icelines.exe`) file
3. Open a terminal in that folder and run:

```bash
icelines fetch all        # download current NHL data (~5 seconds)
icelines tui              # launch the full interactive app
icelines menu             # don't know which surface you want? Pick from a menu.

# Or boot directly on a specific surface:
icelines tui scores               # tonight's games
icelines tui goalies              # goalie leaderboard
icelines tui poach                # fantasy poacher board
icelines tui watchlist            # fantasy poacher watchlist
icelines tui player Bedard        # Bedard's card cold
icelines tui team EDM             # Edmonton depth chart
```

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
```

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
```

Players are color-coded by **cross-team fit** — how they'd rank on each of the other 31 teams:
- ★ **Elite** — true caliber for this slot on most rosters
- ~ **Solid** — fits their role
- ↑ **Buried** — underused, would play higher elsewhere
- ↓ **Stretch** — overextended in current role

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
icelines mates "Beniers" --top 5          # linemates (requires fetch shifts)
```

### Fantasy league

```bash
# Setup
icelines fantasy league-create "My League" --scheme yahoo-standard
icelines fantasy team-create "My Team" --owner "Gio"
icelines fantasy team-add "My Team" "McDavid"
icelines fantasy team-add "My Team" "Kucherov"

# Manage
icelines fantasy team-show "My Team"       # roster with per-player fantasy scores
icelines fantasy standings                 # league standings
icelines fantasy league-switch "My League" # switch active league
icelines fantasy team-use "My Team"        # mark your roster for gaps/poach
icelines fantasy gaps --category hits,blocks,shots
icelines fantasy simulate --weeks 4
icelines fantasy simulate --add "McDavid" --drop "Bouchard" --json

# Poacher
icelines poach --category hits,blocks --top 15
icelines poach --availability imported-available --category hits,blocks --top 15
icelines poach --team SEA --pos LW --json
icelines report poach --category shots --top 10 --out poach.md
icelines report weekly --league default --category hits,blocks
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
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski"          # simulate
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --execute # commit

# Web dashboard
icelines serve --port 8000
# GET /fantasy                 -> HTML gaps + simulation scenarios
# GET /api/v1/fantasy/gaps     -> FantasyRosterGapView JSON
# GET /api/v1/fantasy/simulate -> FantasySimulationView JSON
# GET /poach                   -> HTML poacher board
```

**Fantasy schemes:** `yahoo-standard`, `espn-standard`, `simple-pts`

### Data and history

```bash
# Fetch fresh data (optional — bundled data works immediately)
icelines fetch all              # rosters + stats (~5 min)
icelines fetch realtime         # hits, blocks, giveaways, takeaways, PIM
icelines fetch money-puck       # xG, CF%, FF%, xGF% from MoneyPuck (free)
icelines fetch contracts        # UFA/RFA/ELC contract status

# Historical seasons (1987-88 through 2024-25)
icelines data install --season 19881989    # Gretzky's first LA season
icelines data install --seasons 5          # last 5 seasons
icelines data install --seasons 38         # full history 1987–2025
icelines data list                          # show installed seasons + player counts
icelines data remove 19921993              # uninstall a season

# Multi-season queries (requires seasons installed)
icelines query leaders --seasons 10 --pos C --sort pts-pace --top 10
icelines query leaders --seasons 5  --sort pts-pace --top 10
```

**38 seasons available** — back to 1987-88 (Gretzky trade to LA Kings). Skip 2004-05 (full lockout).

### TUI (`icelines tui` or `icelines dashboard`)

Interactive dashboard with six tabs (League / Depth / Stats / Goalies / Scores / Schedule), plus Playoffs and Transactions overlays. Player cards lazy-load every player's full historical career across all 38 bundled seasons on first open.

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle tabs forward / backward |
| `1` / `↑↓` | Navigate within a tab |
| `Enter` | Drill into selection (team / player / game) |
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
icelines trade "Bouchard" for "Fox" --team EDM  # depth chart trade impact

icelines group create "Watchlist"   # player watchlists (SQLite-backed)
icelines group add "Watchlist" "McDavid"
icelines group show "Watchlist"

icelines scheme list                # fantasy scoring schemes
icelines scheme show yahoo-standard # show weights

icelines snapshot list              # data snapshots
icelines snapshot verify            # integrity check

icelines build                      # generate mkdocs site
icelines serve                      # serve site locally
icelines deploy                     # deploy to GitHub Pages
```

---

## Data sources

| Source | What | Command |
|--------|------|---------|
| NHL API (free, public, no key) | Stats, rosters, bios, realtime, schedule | `icelines fetch all` |
| MoneyPuck (free CSV) | xG, CF%, FF%, xGF% at 5v5 | `icelines fetch money-puck` |
| Bundled (in binary) | 5 seasons 20212022–20252026 | — (zero config) |
| GitHub Releases | 38 seasons 19871988–20252026 | `icelines data install` |

The bundled data refreshes weekly via GitHub Actions. `icelines rank` and `query leaders` work immediately after install with no fetch required.

---

## Architecture

```
icelines-core    pure domain types, filters, scheme scoring - no I/O
icelines-query   Art Ross query parser, planner, executor
icelines-fetch   NHL API client, snapshot store, bundled data, MoneyPuck
icelines-site    mkdocs static site generation
icelines-web     axum web/API surface
icelines-cli     thin UI layer - commands, TUI, HTTP server (axum)
```

6-crate Rust workspace. Scenario coverage now includes **2,000+ persona/harness tests** plus broad L0/L1/L2 integration, system, mock NHL API, TUI, query, and web gates. See `design/notes/2026-05-09-scenario-harness-inventory.md` for the current harness map.

---

## Tests

```bash
cargo test                    # full workspace tests: L0, L1, L2, mock API, persona waves
cargo clippy -- -D warnings   # must be clean
cargo fmt --check             # must be clean
```

Windows-friendly slices:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 list
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-query        # Tests / query
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-cli-tui      # Tests / cli-tui-bin
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 scenarios       # TUI + CLI + query + web scenario harnesses
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-clippy       # Quality / clippy
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 tui-snapshots   # app snapshot module only
```

---

## License

MIT — see [LICENSE](LICENSE).

Copyright (c) 2026 Gio Della-Libera
