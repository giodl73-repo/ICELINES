# Window historical evaluation evidence

This directory is a reproducible, retrospective evaluation of the separate
`observed_history.v1` Frame. It does **not** calibrate or promote the current
`balanced.v1` descriptive Frame.

The four `standings-*.json` files seal final official NHL regular-season
standings from `https://api-web.nhle.com/v1/standings/{date}`. The outcome
adapter retains official point percentage and compares the Window board to its
empirical league percentile so both sides use the same 0..100 scale.

The origin roles were frozen as:

- 2021-22 features -> 2022-23 outcome: training
- 2022-23 features -> 2023-24 outcome: training
- 2023-24 features -> 2024-25 outcome: validation
- 2024-25 features -> 2025-26 outcome: retrospective holdout

Each board uses only bundled source-season `stats.json` and `bios.json` rows at
its feature cutoff. Aggregate multi-team rows are omitted instead of being
allocated without stint authority, and every affected team carries conservative
coverage and a limitation.

`evaluation-2022-23-through-2025-26.json` is the sealed combined result. As of
the 2026-07-28 method freeze:

| Split | MAE | Neutral-baseline MAE | Rank correlation | Status |
| --- | ---: | ---: | ---: | --- |
| Training | 24.861 | 25.806 | 0.236 | inconclusive |
| Validation | 23.695 | 25.706 | 0.335 | calibrated checkpoint |
| Retrospective holdout | 25.892 | 25.403 | 0.163 | inconclusive |

The headline is therefore `inconclusive`. The holdout is a completed historical
season evaluated after freezing its role; it is not described as an untouched
future-season result.

## Paired personnel evidence

`personnel-evidence-2024-25.json` seals a separate Jan. 31 -> Feb. 28, 2025
paired rolling-replay estimate. The actual later checkpoint includes all dated
personnel evidence known by Feb. 28; the paired counterfactual retains evidence
through Jan. 31 and omits only later events. Both use 1,000 trials and seed
`20242025`.

The interval contains 219 dated events across all 32 organizations. Eleven
organizations have a nonzero raw `nhl.expected_points` effect. No organization
crosses an empirical-percentile boundary, so aggregate personnel score deltas
are zero. The checked summary deliberately retains the raw effects and says
that the result is a paired seeded estimate, not a causal or calibrated claim.

The source forecasts, boards, input, and full attributed movement are generated
artifacts rather than checked fixtures. Reproduce the counterfactual forecast
with `icecast season --replay-mode rolling --through 2025-02-28
--ignore-replay-personnel-after 2025-01-31 --trials 1000 --seed 20242025`, then
use `window-build`, `window-movement`, `window-personnel-input-build`,
`window-personnel-attribution`, and `window-personnel-summary` in that order.
