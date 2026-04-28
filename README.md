# IceLines — NHL Analytics Platform

NHL depth charts, pace-adjusted rankings, query engine, fantasy league management, and 38 seasons of history — all from a single Rust CLI with 5 seasons bundled in, no fetch required.

**[→ View the site](https://giodl73-repo.github.io/ICELINES/)**

---

## Quick start

```bash
git clone https://github.com/giodl73-repo/ICELINES.git icelines
cd icelines/src
cargo build --release
```

Works immediately — five seasons of NHL data ship inside the binary:

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
--age-min / --age-max    # age range
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

# Trades
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski"          # simulate
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Werenski" --execute # commit

# Web dashboard
icelines fantasy serve --port 8080
# GET /  → HTML standings
# GET /api/standings → JSON
# GET /api/teams     → JSON
# POST /api/trade    → simulation JSON
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
icelines-core    pure domain types, filters, scheme scoring — no I/O
icelines-fetch   NHL API client, snapshot store, bundled data, MoneyPuck
icelines-site    mkdocs static site generation
icelines-cli     thin UI layer — commands, TUI, HTTP server (axum)
```

4-crate Rust workspace. 338 tests: L0 unit · L1 integration · L2 system · mock NHL API fixture.

---

## Tests

```bash
cargo test                    # 338 tests — L0, L1, L2, mock API
cargo clippy -- -D warnings   # must be clean
cargo fmt --check             # must be clean
```

---

## License

MIT — see [LICENSE](LICENSE).

Copyright (c) 2026 Gio Della-Libera
