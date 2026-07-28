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
