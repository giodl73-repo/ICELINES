# Data Sources and History

IceLines uses the NHL public API exclusively for first-party data,
with optional MoneyPuck integration for advanced metrics.

---

## Data architecture

```
NHL API (free, public)
├── api-web.nhle.com/v1/
│   ├── /roster/{TEAM}/{SEASON}   → headshots, positions, sweater numbers
│   ├── /schedule/now             → tonight's games + upcoming week
│   └── /player/{ID}/landing      → player landing page (bio)
└── api.nhle.com/stats/rest/en/
    ├── /skater/bios              → nationality, draft, age, height/weight
    ├── /skater/summary           → G, A, GP, PP, SH, GWG, shots, TOI, FO%
    └── /skater/realtime          → hits, blocks, giveaways, takeaways, PIM

MoneyPuck (free CSV, optional)
└── moneypuck.com/moneypuck/playerData/seasonSummary/{YEAR}/regular/skaters.csv
    → xG, Corsi For%, Fenwick For%, xGF% (5v5 situation)
```

No API keys. No authentication. No rate limits for normal use.

---

## What's in each data source

### NHL bios (`fetch stats`)

Player biographical data: full name, position, birth date, nationality,
shooting hand, draft year/round/pick, height, weight, province of birth,
first NHL season (rookie detection).

### NHL stats (`fetch stats`)

Per-season statistics: G, A, Pts, GP, PP goals/pts, SH goals/pts,
GWG, OT goals, shots, shooting%, +/-, TOI per game, faceoff win%.

Pass `--type {regular|playoff|both}` to pick which game-type to fetch.
Default is `regular`; `playoff` writes co-located `playoff-bios.json` /
`playoff-stats.json` next to the regular files; `both` runs the full
pipeline twice into one snapshot.

```bash
icelines fetch stats   --season 20242025 --type playoff
icelines fetch goalies --season 20242025 --type playoff
icelines fetch all     --season 20242025 --type both       # full season-end snapshot
```

`--type playoff` skips realtime and MoneyPuck — the NHL realtime feed
is regular-season-only, and MoneyPuck has no playoff endpoint.

### NHL realtime (`fetch realtime`)

Physical and two-way stats: hits, blocked shots, missed shots,
giveaways, takeaways, penalty minutes.

### MoneyPuck (`fetch money-puck`)

Advanced shot metrics computed from play-by-play:
- `xG` — individual expected goals (all situations)
- `CF%` — Corsi For% at 5v5 (shot attempt share while on ice)
- `FF%` — Fenwick For% at 5v5 (unblocked shot attempt share)
- `xGF%` — Expected Goals For% at 5v5 (on-ice shot quality share)

These are silo'd — stored separately, optional, gracefully absent when not fetched.

---

## Bundled data

All 38 supported seasons are compiled into the `icelines` binary using Rust's `include_bytes!()`.
Zero network access required:

```
data/seasons/20252026/bios.json                  ~449 KB
data/seasons/20252026/stats.json                 ~417 KB
data/seasons/20252026/goalie-stats.json          ~22 KB
data/seasons/20252026/playoff-bios.json          (Hart.6.3 — 2026-05-02)
data/seasons/20252026/playoff-stats.json
data/seasons/20252026/playoff-goalie-stats.json
(× 38 seasons, excluding the 2004-05 lockout)
```

Bundled data is refreshed during release/data-prep work and published with each release.

The 2025-26 playoff files ship as `[]` until the Stanley Cup is contested;
the loader surfaces a clean `MissingBundle{Playoff}` error in that state.

---

## Historical seasons

The same 38 seasons can be refreshed from GitHub Releases, back to 1987-88:

```bash
icelines data install --seasons 38      # full history
icelines data install --season 19931994 # Gretzky's last great LA season
icelines data list                      # show what's installed
```

Installed data lives in `~/.icelines/seasons/YYYYZZZZ/`.

### Era guide

```
`````proof:tree kind=taxonomy indent-width=2
root: NHL Eras
- Gretzky-trade era (1987–1999)
  - Gretzky to LA (1988), Lemieux peak, 1994-95 lockout
- Pre-cap era (2000–2004)
  - Salary explosion, 30-team league, classic rivalries
- Lockout & cap era (2005–present)
  - Salary cap introduced, analytics revolution
  - Ovechkin + Crosby rookies (2005-06)
  - McDavid + Eichel rookies (2015-16)
  - Analytics mainstream (2018+)
```
```

### Why historical data matters

- **Contract comps**: Find what comparable players earned at age 24 in 2015
- **Generational comparison**: McDavid vs Gretzky pace at same age
- **Draft class analysis**: How does 2022 stack up against 2015?
- **Multi-season aggregation**: 10-year totals reveal true sustained excellence

---

## MoneyPuck integration

MoneyPuck publishes free season CSVs. Fetch once per season:

```bash
icelines fetch money-puck
```

After fetching, these metrics appear in `query leaders`:

```bash
icelines query leaders --sort xgf-pct --gp-min 40 --top 15   # possession leaders
icelines query leaders --sort cf-pct --pos D --top 20         # defensive possession D
icelines query leaders --sort xg-per-60 --pos F --top 10      # shot quality creators
```

If MoneyPuck data hasn't been fetched, these metrics return "—" gracefully.

---

## Snapshot store

`icelines fetch` creates named, sealed snapshots in `~/.icelines/snapshots/`:

```bash
icelines snapshot list        # all snapshots with tier, date, sealed status
icelines snapshot verify      # re-verify SHA-256 integrity hashes
icelines snapshot show <name> # full detail (files, hashes, parent chain)
icelines snapshot use <name>  # switch to a different snapshot
```

Snapshots are immutable after sealing. Each snapshot has:
- SHA-256 integrity hash per file
- Parent chain (stats snapshot links to its rosters snapshot)
- Tier classification (Rosters → Stats → Realtime → Contracts → MoneyPuck)

If a snapshot exists, it takes precedence over bundled data automatically.

---

## Refresh cadence

```
`````proof:tree kind=org indent-width=2
root: Data freshness
- Current season (bundled)
  - refreshed: release/data-prep workflow before publishing
  - use case: cold start, no fetch required
- Current season (snapshot)
  - refreshed: whenever you run icelines fetch stats
  - use case: latest stats mid-week
- Historical seasons (bundled or installed)
  - refreshed: only when a corrected bundle is intentionally published
  - use case: multi-season queries, comps research
```
```
