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

## Generating the site

```bash
pip install mkdocs-material
python gen_site.py   # regenerates all docs/
mkdocs serve         # preview locally at http://127.0.0.1:8000
mkdocs gh-deploy     # push to GitHub Pages
```
