# Skater attribute recipe comparison — 2026-11-23

The Rust `profile-calibrate` command compares five hypotheses without changing
production card grades or simulator values. Its JSON is also the data source for
a standalone interactive report. This is a single captured-season diagnostic,
not evidence that a candidate measures underlying skill better.

## Recipes

Passing candidates rank either primary assists/60, or
`(primary + 0.5 × secondary assists) × 60 / minutes`. The half-credit weight is
a hypothesis. Credited assists describe scoring involvement, not all passing.

Shooting retains the baseline goals/60 percentile (50%) and shots/60 percentile
(30%). The final 20% becomes the percentile of
`100 × (goals − xG) / (unblocked attempts + prior)`.
Goals, xG and attempts all come from the same MoneyPuck.com goalie-in-net shot
population. The tested priors are 0, 25 and 100 attempts with zero excess goals.
They shrink the signed residual toward zero; they are not empirically fitted.
The existing NHL production components keep their original populations.

All recipes use the existing nine-decimal quantization, favorable midrank and
1–20 conversion. Forward and defense reference cohorts remain separate, with
10 GP, 100 minutes and at least 20 measured peers. Smaller recipients remain
viewable but do not enter the summary cohort. Missing opportunities remain
unavailable; signed residuals and recorded zeroes are retained.

## What the captured sample shows

There are 787 current skaters and 612 eligible paired comparisons: 404 forwards
and 208 defensemen. Every eligible player has the inputs for all five candidates.

| Candidate | Mean absolute change, F | Mean absolute change, D | Largest change, F / D |
| --- | ---: | ---: | ---: |
| Primary assists only | 2.58 | 2.97 | 11 / 16 |
| Primary + half secondary | 1.05 | 1.05 | 5 / 5 |
| Adjusted finishing, no prior | 0.46 | 0.40 | 3 / 2 |
| Adjusted finishing, 25 prior | 0.48 | 0.44 | 2 / 2 |
| Adjusted finishing, 100 prior | 0.51 | 0.50 | 2 / 2 |

Primary-only Passing produces large changes for players whose assists are
mostly secondary. That is a change in the construct being rewarded, not proof
those players are poor passers. Half credit is a gentler comparison: Beniers
10→9, Aho 19→18, Fox 20→20. MacKinnon and McDavid also remain 20.

Shooting moves less because only one fifth of its recipe changes. With the
25-attempt prior: MacKinnon 17→18, McDavid 15→15, Aho 18→17, Fox 12→11 and
Beniers 13→13. Shrinking raw residuals does not guarantee smaller grade changes:
re-ranking changes relative positions too.

The report compares Pearson correlations with PP time share on identical paired
players. Primary-only Passing falls from 0.574→0.490 for forwards and
0.626→0.530 for defense; half credit barely changes the defense association
(0.626→0.623). This neither isolates deployment effects nor validates a formula.
No recipe is promoted. A useful next check is repeatability in held-out seasons
and strength-specific comparisons, rather than selecting the most attractive
single-season movers.

## Reproduction and validation

Inputs were refreshed from the retained NHL, EDGE, official goal-log and
MoneyPuck captures using the same acquisition path documented in
[skater-context validation](rinkcraft-skater-context-validation-2026-11-23.md).
The refresh adds PP share and same-archive shot/goal counts to context values.
All 3,948 existing six-score arrays remain exactly identical to that baseline.

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-calibrate --season 20262027 --catalog target/attribute-calibration/inputs/rinkcraft-stat-attributes.json --season-package target/attribute-calibration/inputs/season-package.json --output-dir target/attribute-calibration/report
```

Choose a new output directory for replay. The command writes `calibration.json`,
`calibration.md` and `index.html`. The report records both input SHA-256 hashes:

- Catalog: `4a6069ee9937fa3e63da30d4f63a1f677ca666c63bf4f3493f1ef78a1ad66cfa`
- Package: `4e6b2b070c029e327f0bb2a0d3888d7a792ecca0d2082106db5f74f731e2c34a`
- Report: `c8d729193f14bcde145d97fceb7469ae6aea1bddfdf990c9f7cb124a361503ed`

An independent replay produced identical JSON bytes. Four Core tests cover
equal-assist discrimination, position cohorts, missing opportunities, signed
residuals, prior behavior, incompatible recipes and count/time checks. The native
CLI builds with the locked dependency graph. Browser acceptance checks all five
recipes, cohort counts, filters, search, sorting, player links, no page errors and
390px page overflow; desktop/mobile screenshots were inspected.

The loopback report is served at `http://127.0.0.1:8780/calibration/`. Its player
links open the existing skater preview. This is a CLI-generated review artifact;
no calibration WASM export, full app integration or hosted release is claimed.
