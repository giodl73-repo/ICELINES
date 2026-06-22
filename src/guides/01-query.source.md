# The Query Engine

`icelines query` is the most powerful command in IceLines. Three modes:
**leaders** (ranked leaderboard), **player** (deep profile), **compare** (head-to-head or similarity).

---

## query leaders

Ranked leaderboard with flexible filtering and 30+ sort metrics.

### Basic usage

```bash
icelines query leaders                              # top 20 by pts/82
icelines query leaders --top 50                    # top 50
icelines query leaders --pos C --top 15            # centers only
icelines query leaders --team SEA                  # Kraken only
```

### Sort metrics

```bash
--sort pts-pace      # pts per 82 (default)
--sort ppg           # pts per game
--sort g-pace        # goals per 82
--sort pp-pts-pace   # PP pts per 82 (power play specialists)
--sort sh-pct        # shooting percentage
--sort plus-minus    # +/- (best defenders)
--sort toi           # ice time per game
--sort hits-pace     # hits per 82 (physical players)
--sort improvement   # Y/Y PPG delta (who's breaking out)
--sort xgf-pct       # Expected Goals For % at 5v5 (requires fetch money-puck)
```

See `src/data/sort-metrics.md` for the complete list.

### Demographic filters

```bash
# Young Finnish forwards
icelines query leaders --nationality FIN --pos F --age-max 26 --sort ppg

# 2022 draft class
icelines query leaders --draft-year 2022 --sort pts-pace

# Ontario/Quebec players
icelines query leaders --birth-province ON,QC --sort pts-pace --top 10

# Undrafted gems
icelines query leaders --undrafted --ppg-min 0.60 --sort ppg

# This season's rookies
icelines query leaders --rookie --sort ppg --top 15
```

### Statistical thresholds

```bash
# Minimum 40 GP, at least 0.80 PPG
icelines query leaders --gp-min 40 --ppg-min 0.80

# Top-pair defensemen (18.5+ min/game)
icelines query leaders --pos D --toi-min 18.5 --sort pts-pace

# Best positive +/- players
icelines query leaders --sort plus-minus --gp-min 50 --top 15
```

### Multi-season aggregation

Aggregate stats across N bundled seasons (requires `icelines data install --seasons N`):

```bash
# 3-year aggregate — most consistent C producers
icelines query leaders --seasons 3 --pos C --sort pts-pace --top 10

# 5-year aggregate — franchise-level production
icelines query leaders --seasons 5 --sort pts-pace --top 15
```

Column header shows `"Pts/82 (3yr)"` so it's always clear you're viewing aggregates.

### League percentiles

```bash
icelines query leaders --pos D --sort pts-pace --top 10 --percentiles
# Adds Pctl column: "100th", "99th", "98th" etc.
```

### Export

```bash
icelines query leaders --top 50 --json     # JSON array
icelines query leaders --top 50 --csv      # CSV with header row
icelines query leaders --top 50 --json > leaders.json
```

---

## query player

Deep profile on a single player: current stats, PP/SH breakdowns, career arc, league rank.

```bash
icelines query player "Connor McDavid"
icelines query player "McDavid" --percentiles        # shows league rank
icelines query player "Celebrini" --percentiles      # #1 RW, 98th percentile
```

Output includes:
- Current season: G, A, Pts, PPG, PP goals/pts, GWG, shots, SH%, +/-, TOI, FO%
- Contract status (if `fetch contracts` has been run)
- League rank among position peers
- Career arc: 5 seasons of season-by-season stats (up to 38 with data install)
- Peak season, career PPG

```bash
# Situational breakdown is parked until verified shift policy ships
icelines query player "McDavid" --breakdown situation
```

---

## query compare

Two modes: **head-to-head** or **similarity search**.

### Head-to-head

```bash
icelines query compare "McDavid" "MacKinnon"
icelines query compare "Bouchard" "Makar"
```

Shows side-by-side: position, age, draft, GP, PPG, Pts/82, Goals/82,
PP points, PP goals, GWG, shots, SH%, +/-, TOI, contract info.

### Similarity search

Find the N most similar players using Z-score Euclidean distance.
Matches on: PPG, goals-per-game, draft pedigree — within age ±2 cohort.

```bash
icelines query compare "Matty Beniers" --similar 8
icelines query compare "Lane Hutson" --similar 5
```

Output shows similarity % (100% = identical profile):

```
SIMILAR PLAYERS TO Matty Beniers (SEA · C · Age 24 · 2021 R1#2)
──────────────────────────────────────────────────────────────────
Rank  Player            Team  Age  Draft      PPG    Similarity
   1  Mason McTavish    ANA   23   21 R1#3   0.547      77%
   2  Marco Rossi       VAN   25   20 R1#9   0.700      73%
   3  Dawson Mercer     NJD   25   20 R1#18  0.512      70%
```

---

## Breakout leaders

Show players most improved year-over-year (requires 2 bundled seasons):

```bash
icelines query leaders --sort improvement --pos F --gp-min 40 --top 15
```

Shows a special table with `Curr | Prior | Δ PPG` columns.
Only includes players who appeared in both seasons with ≥10 GP — no rookie inflation.

---

## Contract filters (requires `icelines fetch contracts`)

```bash
icelines query leaders --ufa --sort pts-pace --top 20  # UFA free agents
icelines query leaders --rfa --pos C --top 15          # RFA centers
icelines query leaders --elc --sort ppg --top 10       # Entry-level players
icelines query leaders --expiry-year 2026 --sort pts-pace
```

If no contract data has been fetched, a hint message guides you.

---

## Examples gallery

```bash
# Who are the best U23 centers right now?
icelines query leaders --pos C --age-max 23 --sort ppg --top 15

# Which 2022 draft picks delivered?
icelines query leaders --draft-year 2022 --sort pts-pace --top 20

# Power play specialists (top PP pts/82, 40+ GP)
icelines query leaders --sort pp-pts-pace --gp-min 40 --top 15

# Best Finnish players in the NHL
icelines query leaders --nationality FIN --sort ppg --top 10

# Toughest players (most hits per 82)
icelines query leaders --sort hits-pace --gp-min 50 --top 15

# Who plays most like Beniers?
icelines query compare "Beniers" --similar 8

# 3-year consistent producers among Swedish D
icelines query leaders --nationality SWE --pos D --seasons 3 --sort pts-pace
```
