# IceLines Query Engine — Specification

**Version**: 0.1  
**Date**: 2026-04-26  
**Status**: Design — informed by Natural Stat Trick, Hockey Reference, MoneyPuck, Evolving Hockey, QuantHockey

---

## What the major sites do that IceLines should match

| Site | Killer feature |
|------|---------------|
| Natural Stat Trick | Strength-state drilling (5v5 / PP / PK) with score state |
| Hockey Reference | Similarity scores + Play-Index custom query builder |
| MoneyPuck | xG model + "deserve to win" / luck vs. skill charts |
| Evolving Hockey | RAPM (player value isolated from teammates) + percentile context |
| QuantHockey | Cross-league + demographic + geographic filters |

---

## Three query modes

### Mode 1 — Leaderboard (`icelines query leaders`)
Ranked table filtered by any combination of dimensions.

```bash
# Top 20 centers by 5v5 points pace
icelines query leaders --pos C --situation 5v5 --sort pts-pace --top 20

# U23 Swedish forwards with >0.8 PPG in last 3 seasons
icelines query leaders --pos F --age-max 23 --nationality SWE \
  --ppg-min 0.80 --seasons 3

# Power play specialists: top PP points, min 20 PP minutes
icelines query leaders --situation pp --sort pp-pts-pace --pp-toi-min 20

# Top defensemen by xGF% at 5v5
icelines query leaders --pos D --situation 5v5 --sort xgf-pct
```

### Mode 2 — Player profile (`icelines query player`)
Deep dive on one player across all contexts.

```bash
# Full situational breakdown: 5v5 / PP / PK / all-situations
icelines query player "Nathan MacKinnon" --breakdown situation

# Career arc: season-by-season with percentile rank among contemporaries
icelines query player "Connor McDavid" --breakdown career --percentiles

# Last 20 games rolling (hot/cold streak)
icelines query player "Matty Beniers" --last-n 20
```

### Mode 3 — Comparison (`icelines query compare`)
Side-by-side or similarity search.

```bash
# Head-to-head explicit comparison
icelines query compare "McDavid" "MacKinnon" --situation 5v5

# Find 5 players most similar to Beniers at his age
icelines query compare "Beniers" --similar 5 --by career-arc

# Draft class: rank 2022 first-rounders by xGF% contribution
icelines query compare --draft-year 2022 --round 1 --sort xgf-pct
```

---

## Filter dimensions (complete set)

### Time
| Flag | Values | Notes |
|------|--------|-------|
| `--season YYYY` | e.g. 2025 | Single season (YYYY = start year) |
| `--seasons N` | 1–5 | Rolling N-season aggregate |
| `--last-n N` | games | Last N games only |
| `--date-from / --date-to` | YYYY-MM-DD | Custom date window |
| `--game-type` | regular, playoffs, preseason | Default: regular |

### Situation / strength state
| Flag | Values | Notes |
|------|--------|-------|
| `--situation` | all, 5v5, pp, pk, 4v4, 3v3 | Default: all |
| `--score-state` | tied, leading, trailing, close | Within ±1 goal = "close" |
| `--home-away` | home, away, both | Default: both |

### Player demographics
| Flag | Values |
|------|--------|
| `--pos` | C, LW, RW, F, D, G |
| `--age-min / --age-max` | integers |
| `--nationality` | ISO-3166 alpha-3 codes, comma-separated |
| `--region` | NorthAmerica, Scandinavia, CentralEurope, Russia |
| `--draft-year` | single year |
| `--draft-round` | 1–7 |
| `--draft-pick-max` | overall pick ceiling |
| `--undrafted` | flag |
| `--rookie` | flag — first NHL season |
| `--handedness` | L, R |

### Statistical thresholds
| Flag | Meaning |
|------|---------|
| `--gp-min N` | Minimum games played |
| `--toi-min N` | Minimum TOI (minutes per game or total) |
| `--ppg-min F` | Minimum points per game |
| `--xgf-pct-min F` | Minimum xGF% (requires Tier 4 data) |

### Output / display
| Flag | Meaning |
|------|---------|
| `--sort METRIC` | Sort column |
| `--top N` | Show top N results |
| `--rate / --total` | Per-game rates vs. season totals |
| `--percentiles` | Show league percentile for each stat |
| `--json / --csv` | Structured export |

---

## Metrics catalog

> **Phase Lindsay update (v0.2, 2026-05-03)**: this section is
> **superseded** by the central `StatId` catalog at
> `icelines-core::stats_catalog`. Every available metric — 108 total
> across 9 categories (Identity / Scoring / SpecialTeams / TwoWay /
> TimeOnIce / OnIceGoals / Possession / Goalie / Derived) — is
> enumerated in code with `cli_key`, `label`, `short_label`,
> `category`, `unit`, `higher_is_better`, `applies_to`, and
> `available_since` accessors.
>
> CLI usage: `icelines query leaders --sort <cli_key>` accepts every
> `StatId::cli_key()` value in addition to the legacy ~37 alias
> strings (`pts-pace`, `ppg`, etc.). See L.5.1 in
> `design/plans/2026-05-02-phaseLindsay-stat-catalog.md`.
>
> The legacy Tier 1/2/3 inline tables below are kept for historical
> reference — the *canonical* catalog is the code, not this spec.

### Tier 1 — Available now (NHL API, bundled data)
| Metric | Key | Notes |
|--------|-----|-------|
| Goals | `g` | Season total |
| Assists | `a` | Season total |
| Points | `pts` | G + A |
| Games Played | `gp` | |
| Goals/82 | `g-pace` | Pace projection |
| Points/82 | `pts-pace` | Pace projection |
| Goals per game | `gpg` | |
| Points per game | `ppg` | |
| PP Goals | `pp-g` | |
| PP Points | `pp-pts` | |
| SH Goals | `sh-g` | |
| GWG | `gwg` | |
| Shots | `shots` | |
| Shooting % | `sh-pct` | |
| TOI/game (ES) | `es-toi` | From shift data (Tier 3) |
| Zone Start % | `zs-pct` | From shift data (Tier 3) |

### Tier 2 — Available via shift data (Tier 3, `icelines fetch shifts`)
| Metric | Key | Notes |
|--------|-----|-------|
| ES TOI/game | `es-toi` | Even strength time on ice |
| PP TOI/game | `pp-toi` | Power play time on ice |
| PK TOI/game | `pk-toi` | Penalty kill time on ice |
| OZ Start % | `oz-pct` | Offensive zone face-offs |
| DZ Start % | `dz-pct` | Defensive zone face-offs |
| Linemate quality | `qot` | Avg teammate pace score |

### Tier 3 — Future (Natural Stat Trick / MoneyPuck scraping)
| Metric | Key | Source |
|--------|-----|--------|
| Corsi For % | `cf-pct` | NST |
| Fenwick For % | `ff-pct` | NST |
| xG For % | `xgf-pct` | MoneyPuck / NST |
| Individual xG | `ixg` | MoneyPuck |
| GAR | `gar` | Evolving Hockey |
| xGAR | `xgar` | Evolving Hockey |
| RAPM | `rapm` | Evolving Hockey |
| PDO | `pdo` | NST (SH% + SV%) |

---

## Similarity scoring

Modeled after Hockey Reference (similarity scores) and Evolving Hockey (Z-score distance).

```bash
icelines query compare "Matty Beniers" --similar 10
```

Algorithm:
1. For the target player at their current age, collect: GPG, PPG, xGF% (if available), draft position, position
2. Z-score normalize each dimension against all active players at same position/age ±2
3. Compute Euclidean distance in Z-score space
4. Return top N closest — lower distance = more similar

Output:
```
SIMILAR PLAYERS TO MATTY BENIERS (SEA · C · Age 22 · 2022 R1#2)
────────────────────────────────────────────────────────────────
 Rank  Player              Team  Age  Draft     PPG   Similarity
    1  Logan Cooley        UTA   21   22 R1#3   0.58     94%
    2  Dylan Guenther      UTA   21   22 R1#9   0.65     91%
    3  Wyatt Johnston      DAL   23   21 R1#23  0.82     88%
    4  Marco Rossi         VAN   23   20 R1#9   0.52     85%
```

---

## Strength-state implementation

The key differentiator of Natural Stat Trick. Requires shift data (Tier 3).

Current status: IceLines has shift data infrastructure (`ShiftProfile`, `avg_ev_toi_seconds_per_game`) but no on-ice event data (goals/shots while on ice at 5v5 vs. PP).

Phase 5 addition: Pull play-by-play from NHL API and cross-reference with shifts to compute:
- `5v5_points`, `5v5_toi` — purely even strength production
- `pp_points`, `pp_toi` — purely power play production
- `5v5_pts_per_60` — the gold standard forward ranking metric

---

## Priority implementation order

### Phase 5A — Query CLI (builds on PlayerFilter)
1. `icelines query leaders` — full filter set against Tier 1 metrics
2. `--percentiles` flag — shows rank among position peers
3. `--rate` flag — toggle pace vs. totals
4. `--json` / `--csv` export

### Phase 5B — Similarity scoring
1. Z-score normalization per position/age cohort
2. `icelines query compare --similar N`
3. Career arc comparison (`--by career-arc`)

### Phase 5C — Strength states (requires Tier 3 data)
1. `icelines fetch shifts` → compute 5v5/PP/PK splits
2. `--situation` filter in all query commands
3. `icelines query leaders --situation 5v5 --sort pts-pace` works

### Phase 5D — Advanced metrics (requires Tier 4 data scraping)
1. NST scraper for CF%, FF%, xGF%
2. MoneyPuck xG download
3. `--sort xgf-pct` available in query leaders

---

## Example queries IceLines will answer (Phase 5A)

```bash
# Who are the best U23 centers right now?
icelines query leaders --pos C --age-max 23 --sort ppg --top 15

# Which 2022 draft picks have delivered?
icelines query leaders --draft-year 2022 --sort pts-pace

# Best Finnish players in the NHL
icelines query leaders --nationality FIN --sort ppg

# Power play specialists (top PP points pace, any position)
icelines query leaders --sort pp-pts-pace --top 20

# Defensemen who contribute offensively
icelines query leaders --pos D --sort pts-pace --top 20

# Players who overcame injuries (≥10 GP but <40 GP, high pace)
icelines query leaders --gp-min 10 --gp-max 40 --sort pts-pace --top 15

# Who plays most like McDavid at the same age?
icelines query compare "Connor McDavid" --similar 5 --age-match

# Best players from Canadian Prairie cities
icelines query leaders --nationality CAN --birth-province AB,SK,MB

# Draft value analysis: undrafted players with >0.6 PPG
icelines query leaders --undrafted --ppg-min 0.60 --sort ppg
```
