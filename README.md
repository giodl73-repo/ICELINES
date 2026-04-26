# IceLines — NHL Depth Chart Tracker

Real lineup cards and pace-adjusted rankings for all 32 NHL teams.

**[→ View the site](https://giodl73-repo.github.io/ICELINES/)**

## What it shows

- **4×3 forward lines** (LW / C / RW) and **3×2 defense pairs** for every team
- Players color-coded by fit: 🟢 elite fit · 🟡 solid · 🔵 buried · 🔴 overextended
- Rankings based on **points per game** (G+A / GP × 82) with goals/gp as tiebreaker
- Every player's pace-projected stats: `{GP}gp · {PPG} pts/gp · {proj} proj`
- Trade analysis: who each team can deal and what they need

## Methodology

**Scoring**: Points per game (`(G+A) / GP × 82`) with goals/gp as tiebreaker.
All stats projected to an 82-game pace — a player with 60 pts in 40 games ranks
the same as a player with 120 pts in 82 games.

**Fit classification** (per player vs their line slot):

| Color | Label | Condition |
|-------|-------|-----------|
| 🟢 Green | ★ fit | avg elsewhere ≤ own line + 0.5 |
| 🟡 Yellow | ~ solid | avg elsewhere ≤ own line + 1.25 |
| 🔵 Blue | ↑ buried | avg elsewhere < own line − 0.75 |
| 🔴 Red | ↓ stretch | avg elsewhere > own line + 1.25 |

Stats sourced from Yahoo Fantasy Hockey 2025–26 season data and NHL API.

## Structure

```
ICELINES/
├── scripts/
│   ├── gen_site.py      # generate docs/ from CSV + GP data
│   ├── fetch_gp.py      # refresh NHL GP data → data/gp_data.json
│   └── deploy.bat       # one-click regenerate + publish
├── data/
│   └── gp_data.json     # cached NHL games-played data
├── docs/                # generated site source (mkdocs input)
│   ├── index.md
│   ├── assets/
│   └── teams/
├── src/                 # future Rust CLI
└── mkdocs.yml
```

## Usage

```bash
# Refresh NHL GP data from API
python scripts/fetch_gp.py

# Regenerate site
python scripts/gen_site.py

# Preview locally
PYTHONUTF8=1 mkdocs serve

# Deploy to GitHub Pages (or double-click scripts/deploy.bat)
PYTHONUTF8=1 mkdocs gh-deploy
```
