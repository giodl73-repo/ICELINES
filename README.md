# IceLines — NHL Depth Chart Tracker

Real lineup cards and pace-adjusted rankings for all 32 NHL teams.

**[→ View the site](https://giodl73-repo.github.io/ICELINES/)**

## What it shows

- **4×3 forward lines** (LW / C / RW) and **3×2 defense pairs** for every team
- Players color-coded by fit: 🟢 elite fit · 🟡 solid · 🔵 buried · 🔴 overextended
- Rankings based on **points per game** (G+A / GP × 82) with goals/gp as tiebreaker
- Every player's pace-projected stats: `{GP}gp · {PPG} pts/gp · {proj} proj`

## Methodology

**Scoring**: Points per game (`(G+A) / GP × 82`) with goals/gp as tiebreaker.
All stats projected to an 82-game pace — a player with 60 pts in 40 games ranks
the same as a player with 120 pts in 82 games.

**Fit classification** (avg line on other 31 teams vs own line slot):

| Color | Label | Condition |
|-------|-------|-----------|
| 🟢 Green | ★ elite fit | avg elsewhere ≤ own line + 0.5 |
| 🟡 Yellow | ~ solid | avg elsewhere ≤ own line + 1.25 |
| 🔵 Blue | ↑ buried | avg elsewhere < own line − 0.75 |
| 🔴 Red | ↓ stretch | avg elsewhere > own line + 1.25 |

Stats sourced from the NHL API. Rosters, headshots, and all scoring stats come
from `api-web.nhle.com` and `api.nhle.com/stats/rest/en/` — no external CSV dependency.

## Structure

```
ICELINES/
├── scripts/
│   └── deploy.bat           # one-click build + publish to GitHub Pages
├── data/
│   └── rosters.json         # cached NHL roster + headshot data
├── docs/                    # generated site source (mkdocs input)
│   ├── index.md
│   ├── assets/
│   └── teams/
├── src/                     # Rust CLI (icelines)
│   ├── icelines-core/       # scoring engine, models, cross-team metrics
│   ├── icelines-fetch/      # NHL API client, snapshot store
│   ├── icelines-site/       # site generator (replaces gen_site.py)
│   └── icelines-cli/        # binary entry point
└── mkdocs.yml
```

## Usage

```bash
# First build the binary (one-time)
cd src && cargo build --release

# Fetch NHL data (rosters, stats) — creates a named snapshot
icelines fetch all

# Preview site locally
icelines build --no-site
PYTHONUTF8=1 mkdocs serve

# Deploy to GitHub Pages (or double-click scripts/deploy.bat)
icelines build && PYTHONUTF8=1 mkdocs gh-deploy

# Manage snapshots
icelines snapshot list
icelines snapshot verify
icelines snapshot use <name>

# Rankings
icelines rank --top 20
icelines team SEA
```
