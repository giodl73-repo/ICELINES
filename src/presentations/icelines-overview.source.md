# IceLines — NHL Analytics Platform

## What it is

IceLines is a Rust CLI for NHL analytics and fantasy hockey. Five seasons
of NHL data ship in the binary — no fetch, no account, no setup.

---

## Core capabilities

```proof:tree kind=org
root: IceLines
- Query Engine
  query leaders (30+ sort metrics, all filters)
  query player (career arc, percentile rank)
  query compare (head-to-head, similarity search)
- Team Analysis
  depth charts (4x3 forward, 3x2 defense)
  fit classification (Elite/Solid/Buried/Stretch)
  trade impact simulation
- Player Research
  scouting reports (8 sections, JSON export)
  career history (38 seasons, 1987–2025)
  projection engine (pace/regressed/composite)
  similarity/comps search
- Fantasy League
  leagues and teams (SQLite-backed)
  scoring (Yahoo/ESPN/custom schemes)
  trade evaluation and execution
  HTTP dashboard server
- Data
  bundled binary (5 seasons, no fetch)
  38 seasons installable via GitHub Releases
  MoneyPuck xG/CF%/xGF% (optional)
  NHL realtime stats (hits/blocks/giveaways)
```

---

## Data sources

```
NHL API (free, public, no key)
├── Rosters + headshots
├── Stats: G, A, GP, PP, SH, GWG, shots, TOI, FO%
├── Realtime: hits, blocks, giveaways, takeaways, PIM
└── Schedule + tonight's games

MoneyPuck (free CSV, optional)
└── xG, Corsi%, Fenwick%, xGF% at 5v5
```

---

## Quick start

```bash
cargo build --release -p icelines-cli

# Works immediately — no fetch required
icelines rank --top 10
icelines team EDM
icelines query leaders --pos C --age-max 23 --sort ppg --top 10
icelines query compare "McDavid" "MacKinnon"
icelines tui
```

---

## 38 seasons — 1987 to now

```bash
icelines data install --seasons 38
icelines query leaders --seasons 10 --sort pts-pace --top 10
icelines query compare "Beniers" --similar 8   # historical comps
```

---

## Fantasy

```bash
icelines fantasy league-create "My League"
icelines fantasy team-create "My Team" --owner "Gio"
icelines fantasy team-add "My Team" "McDavid"
icelines fantasy standings
icelines fantasy trade "Bouchard" --to-team "Other" --for-player "Hughes"
icelines fantasy serve --port 8080
```

---

## Architecture

```
icelines-core    pure domain types, filters, scoring — no I/O
icelines-fetch   NHL API client, snapshot store, bundled data
icelines-site    mkdocs static site generation
icelines-cli     thin UI layer — commands, TUI, HTTP server
```

338 tests · L0 unit · L1 integration · L2 system · mock NHL API fixture

---

## Repo

`github.com/giodl73-repo/ICELINES`

MIT License · Gio Della-Libera · 2026
