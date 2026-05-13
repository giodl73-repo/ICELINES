# IceLines Tutorial — Zero to First Result

Five minutes to your first NHL analytics query. No account, no API key,
no setup beyond building the binary.

---

## Step 1 — Build (60 seconds)

```bash
git clone https://github.com/giodl73-repo/ICELINES.git icelines
cd icelines/src
cargo build --release
```

The binary lands at `src/target/release/icelines` (or `icelines.exe` on Windows).
Add it to your PATH or run it directly.

---

## Step 2 — First query (10 seconds)

No fetch required. Five seasons of NHL data ship inside the binary.

```bash
icelines rank --top 10
```

You'll see the top 10 NHL skaters by pts/82 — pace-adjusted so a player
with 60 pts in 40 games ranks equally to one with 120 pts in 82 games.

---

## Step 3 — Team depth chart

```bash
icelines team EDM
```

This renders Edmonton's lineup as a 4×3 forward grid + 3×2 defense grid.
Each player is color-coded by fit:

- ★ **Elite** — true caliber for this line slot on most teams
- ~ **Solid** — fits their role, slight upgrade available elsewhere
- ↑ **Buried** — underused, would play higher elsewhere
- ↓ **Stretch** — overextended in current role

---

## Step 4 — Filtered query

```bash
# Best U23 centers by points per game
icelines query leaders --pos C --age-max 23 --sort ppg --top 10
```

This uses `query leaders` — the main analytics command. You can combine
any filters: position, age, nationality, draft class, PPG threshold, etc.

---

## Step 5 — Player deep dive

```bash
icelines query player "Macklin Celebrini" --percentiles
```

Shows the full player profile: current stats breakdown (GP, G, A, PP, TOI, +/-),
league rank among position peers, and career arc.

---

## Step 6 — Compare two players

```bash
icelines query compare "McDavid" "MacKinnon"
```

Side-by-side comparison including age, draft, PPG, Pts/82, Goals/82,
PP points, GWG, shots, SH%, +/-, and TOI.

---

## Step 7 — Similarity search

```bash
icelines query compare "Matty Beniers" --similar 8
```

Finds the 8 most similar players by statistical profile (PPG, GPG, draft pedigree)
within the same age cohort. Useful for contract comp research.

---

## Step 8 — Historical data

```bash
# Install all 38 seasons (1987-88 → 2025-26)
icelines data install --seasons 38

# 10-year aggregate — franchise-level consistency
icelines query leaders --seasons 10 --pos C --sort pts-pace --top 10
```

---

## Step 9 — Fantasy league

```bash
icelines fantasy league-create "My League"
icelines fantasy team-create "My Team" --owner "Me"
icelines fantasy team-add "My Team" "McDavid"
icelines fantasy team-add "My Team" "Kucherov"
icelines fantasy team-add "My Team" "Bouchard"
icelines fantasy team-show "My Team"
icelines fantasy team-use "My Team"
icelines fantasy gaps --category hits,blocks,shots
icelines fantasy simulate --weeks 4 --add "McDavid" --drop "Bouchard"
```

The interactive dashboards can run the same fantasy workflow from their command
bars. Launch `icelines tui` and press `:`, or run `icelines serve --port 8000`
and open `/dashboard`, then try:

```text
gaps cats=hits,blocks,shots top=8
poach rw cats=hits,blocks free top=12
simulate add=Connor_McDavid drop=Bench_Forward weeks=3
fantasy simulate add Connor_McDavid drop Bench_Forward
```

---

## Step 10 — Interactive TUI

```bash
icelines tui
# or just: icelines
```

8-screen terminal UI. Arrow keys to navigate, `/` to search, `q` to quit.

---

## What's next

| Goal | Command |
|------|---------|
| Analyze a draft class | `icelines class 2022` |
| Find peers for a player | `icelines peers "Lane Hutson"` |
| Project rest-of-season | `icelines project "Celebrini"` |
| Scouting report | `icelines scouting "Bouchard" --format json` |
| Team trade impact | `icelines trade "Heiskanen" for "Hedman" --team DAL` |
| Y/Y breakout leaders | `icelines query leaders --sort improvement --pos F` |
| PP specialist leaderboard | `icelines query leaders --sort pp-pts-pace --top 15` |
| Fetch fresh stats | `icelines fetch all` |

Full guide index: [docs/guides/](guides/)
