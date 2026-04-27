# IceLines Sort Metrics

Used with `icelines query leaders --sort <metric>`

<!-- proof:figure id="sort-metrics-pace" kind="table.reference" -->
## Pace metrics (per-82-game projections)

| Metric key | Description | Notes |
|-----------|-------------|-------|
| `pts-pace` | Points per 82 games | **Default sort** |
| `ppg` | Points per game | Same ranking as pts-pace, decimal display |
| `g-pace` | Goals per 82 games | |
| `gpg` | Goals per game | Same ranking as g-pace, decimal display |
| `pp-pts-pace` | PP points per 82 | Power play specialists |
| `pp-g-pace` | PP goals per 82 | |
| `sh-g-pace` | SH goals per 82 | Penalty killers |
| `gwg-pace` | Game-winning goals per 82 | Clutch scorers |
| `shots-pace` | Shots on goal per 82 | Volume shooters / possession |
| `hits-pace` | Hits per 82 | Physical play |
| `blocks-pace` | Blocked shots per 82 | Shot-blocking D |
| `takeaways` | Raw takeaways | Defensive skill |
| `xg-per-60` | Individual expected goals per 60 min | Requires `fetch money-puck` |

<!-- proof:figure id="sort-metrics-totals" kind="table.reference" -->
## Season totals

| Metric key | Description |
|-----------|-------------|
| `pts` | Season points |
| `goals` | Season goals |
| `assists` | Season assists |
| `gp` | Games played |
| `pp-pts` | PP points (season total) |
| `pp-g` | PP goals (season total) |
| `sh-g` | SH goals (season total) |
| `gwg` | Game-winning goals (season total) |
| `shots` | Shots on goal (season total) |
| `hits` | Hits (season total) |
| `blocks` | Blocked shots (season total) |
| `giveaways` | Giveaways (season total) |
| `pim` | Penalty minutes |

<!-- proof:figure id="sort-metrics-rates" kind="table.reference" -->
## Rate and advanced metrics

| Metric key | Description | Notes |
|-----------|-------------|-------|
| `sh-pct` | Shooting percentage | |
| `plus-minus` | Plus/minus rating | |
| `toi` | Average TOI per game (MM:SS) | |
| `fo-pct` | Faceoff win percentage | Centers only |
| `xg` | Individual expected goals (all situations) | Requires `fetch money-puck` |
| `cf-pct` | Corsi For % at 5v5 | Requires `fetch money-puck` |
| `ff-pct` | Fenwick For % at 5v5 | Requires `fetch money-puck` |
| `xgf-pct` | Expected Goals For % at 5v5 | Requires `fetch money-puck` |

<!-- proof:figure id="sort-metrics-trend" kind="table.reference" -->
## Trend

| Metric key | Description | Notes |
|-----------|-------------|-------|
| `improvement` | Y/Y PPG delta vs prior season | Shows Curr / Prior / Δ columns. Excludes players missing from prior season. |
