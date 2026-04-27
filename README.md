# IceLines — NHL Analytics Platform

NHL depth charts, pace-adjusted rankings, query engine, and fantasy league management — all from a single Rust CLI with 5 seasons of data bundled in.

**[→ View the site](https://giodl73-repo.github.io/ICELINES/)**

---

## What it does

- **Depth charts** — 4×3 forward lines + 3×2 defense pairs for all 32 teams, color-coded by fit vs the rest of the league
- **Query engine** — `icelines query leaders` with 30+ sort metrics, demographic filters, multi-season aggregation, and Y/Y improvement sort
- **Fantasy leagues** — create leagues and teams, compute scores against any scheme (Yahoo, ESPN, custom), simulate trades, run a web server
- **Advanced stats** — MoneyPuck xG/CF%/FF%, NHL realtime hits/blocks/giveaways, shooting%, TOI
- **Career analysis** — multi-season history, projections, similarity search, comps
- **No cold start** — works immediately after install, 5 seasons bundled in the binary

---

## Quick start

```bash
# Build (one-time)
cd src && cargo build --release

# Query — no fetch required, bundled data works immediately
icelines query leaders --pos C --age-max 23 --sort ppg --top 15
icelines query leaders --draft-year 2022 --sort pts-pace
icelines query leaders --nationality FIN --sort ppg
icelines query leaders --seasons 3 --pos C --top 10   # 3-season aggregate
icelines query leaders --sort improvement --pos F     # Y/Y breakout leaders

# Player profile with career arc and percentile rank
icelines query player "Connor McDavid" --percentiles

# Similarity search
icelines query compare "Matty Beniers" --similar 8

# Team depth chart
icelines team EDM

# Rankings
icelines rank --top 20 --pos D

# Fetch fresh data (optional — bundled data is current season)
icelines fetch all

# Fantasy league
icelines fantasy league-create "My League" --scheme yahoo-standard
icelines fantasy team-create "Gio's Rangers" --owner "Gio"
icelines fantasy team-add "Gio's Rangers" "McDavid"
icelines fantasy standings
icelines fantasy trade "Bouchard" --to-team "Other Team" --for-player "Werenski"
icelines fantasy serve --port 8080   # web dashboard
```

---

## Structure

```
icelines/
├── src/                    Rust workspace (4 crates)
│   ├── icelines-core/      domain types, filters, scheme scoring — no I/O
│   ├── icelines-fetch/     NHL API client, snapshot store, bundled data
│   ├── icelines-site/      mkdocs site generation
│   ├── icelines-cli/       CLI commands, TUI, HTTP server
│   ├── guides/             proof source docs → docs/guides/
│   └── presentations/      proof source docs → docs/presentations/
├── data/                   5 bundled seasons (20212022–20252026)
├── design/                 specs, plans, architecture, invariants
├── docs/                   generated output (mkdocs)
└── .roles/                 8 domain review roles
```

---

## Sort metrics

`icelines query leaders --sort <metric>`

| Category | Metrics |
|----------|---------|
| Points | `pts-pace`, `ppg`, `pts`, `goals`, `assists`, `gp` |
| Goals | `g-pace`, `gpg` |
| Power play | `pp-pts-pace`, `pp-g-pace`, `pp-pts`, `pp-g` |
| Shorthanded | `sh-g-pace`, `sh-g` |
| Other | `gwg-pace`, `gwg`, `shots-pace`, `shots` |
| Rates | `sh-pct`, `plus-minus`, `toi`, `fo-pct` |
| Physical | `hits-pace`, `hits`, `blocks-pace`, `blocks`, `takeaways`, `giveaways`, `pim` |
| Advanced | `xg`, `xg-per-60`, `cf-pct`, `ff-pct`, `xgf-pct` (requires `icelines fetch moneypuck`) |
| Trend | `improvement` (Y/Y PPG delta) |

---

## Data sources

| Source | What | Command |
|--------|------|---------|
| NHL API (free) | Stats, rosters, bios, realtime, schedule | `icelines fetch all` |
| MoneyPuck (free CSV) | xG, CF%, FF%, xGF% | `icelines fetch money-puck` |
| Bundled (binary) | 5 seasons 20212022–20252026 | — (zero config) |

---

## Tests

338 tests across three tiers:

```bash
cd src && cargo test           # all tests
cd src && cargo clippy -- -D warnings
cd src && cargo fmt --check
```

| Tier | Count | Scope |
|------|-------|-------|
| L0 unit | ~270 | Pure logic, in-module |
| L1 integration | ~46 | Real structures, httpmock fixture |
| L2 system | ~68 | Binary subprocess tests |

---

## License

MIT — see [LICENSE](LICENSE).

Copyright (c) 2026 Gio Della-Libera
