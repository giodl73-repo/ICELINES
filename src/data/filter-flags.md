# IceLines Filter Flags

All flags for `icelines query leaders`. All filters combine with AND logic.

<!-- proof:figure id="filter-time" kind="table.reference" -->
## Time / aggregation

| Flag | Type | Description | Example |
|------|------|-------------|---------|
| `--seasons N` | int 1–38 | Aggregate across last N bundled seasons | `--seasons 3` |

<!-- proof:figure id="filter-demographics" kind="table.reference" -->
## Demographics

| Flag | Type | Description | Example |
|------|------|-------------|---------|
| `--pos` | string | Position: C, LW, RW, D, F (all forwards), G | `--pos C` |
| `--team` | string | Team abbreviation | `--team EDM` |
| `--age-min` | int | Minimum age (inclusive) | `--age-min 18` |
| `--age-max` | int | Maximum age (inclusive) | `--age-max 23` |
| `--nationality` | string | ISO-3166 alpha-3 code(s), comma-separated | `--nationality FIN` |
| `--birth-province` | string | Province/state codes, comma-separated | `--birth-province ON,QC` |
| `--handedness` | L or R | Shooting/catching hand | `--handedness L` |

<!-- proof:figure id="filter-draft" kind="table.reference" -->
## Draft / eligibility

| Flag | Type | Description | Example |
|------|------|-------------|---------|
| `--draft-year` | int | Draft year | `--draft-year 2022` |
| `--draft-round` | int 1–7 | Draft round | `--draft-round 1` |
| `--draft-pick-max` | int | Maximum overall pick number | `--draft-pick-max 30` |
| `--undrafted` | flag | Only undrafted players | `--undrafted` |
| `--rookie` | flag | Only players in their first NHL season | `--rookie` |

<!-- proof:figure id="filter-stats" kind="table.reference" -->
## Statistical thresholds

| Flag | Type | Description | Example |
|------|------|-------------|---------|
| `--ppg-min` | float | Minimum PPG (per game, 0.0–2.5) | `--ppg-min 0.80` |
| `--gp-min` | int | Minimum games played | `--gp-min 40` |
| `--gp-max` | int | Maximum games played | `--gp-max 30` |
| `--toi-min` | float | Minimum TOI per game (minutes) | `--toi-min 18.5` |
| `--plus-minus-min` | int | Minimum +/- rating | `--plus-minus-min 5` |
| `--shots-pg-min` | float | Minimum shots per game | `--shots-pg-min 3.0` |

<!-- proof:figure id="filter-contract" kind="table.reference" -->
## Contract status (requires `icelines fetch contracts`)

| Flag | Type | Description |
|------|------|-------------|
| `--ufa` | flag | Only UFA players (unrestricted free agents) |
| `--rfa` | flag | Only RFA players (restricted free agents) |
| `--elc` | flag | Only players on entry-level contracts |
| `--expiry-year` | int | Players with contracts expiring in this year |

<!-- proof:figure id="filter-output" kind="table.reference" -->
## Output / display

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--sort` | string | `pts-pace` | Sort metric (see sort-metrics.md) |
| `--top` | int | 20 | Number of results to show |
| `--rate` | flag | off | Show per-game rates instead of per-82 projections |
| `--percentiles` | flag | off | Show league percentile rank for each result |
| `--json` | flag | off | Export as JSON array |
| `--csv` | flag | off | Export as CSV with header row |
