# Attribute carryover — 2026-11-24

This extends the five diagnostic recipes from
[the first comparison](rinkcraft-attribute-calibration-2026-11-23.md) to three
completed seasons. Formula definitions were fixed in commit `477c56e2` before
this historical comparison. No recipe weights are fitted and no production
grades or simulator values change. The following-season outcomes are outside
each earlier-season input window; this is a retrospective check, not a blinded
preregistered trial.

## Measurement

Each season uses separate F/D reference ranks from the unchanged 10 GP / 100 min
system. Each adjacent-season comparison matches player IDs and requires the same
position group and workload threshold in both seasons. The report repeats the
check at 40 GP / 500 min, using the original ranks rather than recalibrating them.
Missing recipe inputs or outcomes exclude the pair from both sides. Attrition
and missing comparison counts remain visible.

- Grade repeatability: Pearson correlation between each grade and the following
  season's corresponding grade, using identical baseline/candidate player pairs.
- Future production: correlation between each earlier grade and the same future
  NHL all-strength outcome—assists/60 for Passing, goals/60 for Shooting.
- Component repeatability: correlation of the raw primary-assist, weighted-assist
  or adjusted-finishing measurement across the two seasons.
- Mean absolute grade movement: describes individual year-to-year changes; it
  does not establish correctness.

No confidence intervals, deployment adjustment or causal skill identification
are claimed. Survivorship, role persistence, teammate changes, injuries and aging
remain relevant. Stable grades can reflect stable opportunity; low finishing
repeatability can reflect sampling variation. Small correlation differences do
not establish a better formula. This tests Passing and Shooting only.

## Findings

The inputs contain 924, 920 and 940 skaters respectively for 2023–24, 2024–25 and
2025–26. Assist credits reconcile for 924, 918 and 922 players. The other source
rows remain unavailable; no inferred zeroes were used. There are 40 comparison
rows: two season pairs × five recipes × two position groups × two sample filters.

For the larger sample (40 GP / 500 min in both seasons), grade repeatability is:

| Candidate | F, 2023→24 | D, 2023→24 | F, 2024→25 | D, 2024→25 |
| --- | --- | --- | --- | --- |
| Primary assists only | .630 → .558 | .631 → .479 | .683 → .615 | .576 → .513 |
| Primary + half secondary | .630 → .627 | .631 → .624 | .683 → .684 | .576 → .590 |
| Adjusted finishing, 25 prior | .668 → .668 | .624 → .597 | .685 → .681 | .530 → .476 |

Cells show baseline → candidate correlations. Pair labels refer to starting
years: e.g. 2023→24 means 2023–24 inputs compared with 2024–25 outcomes.

Primary-only Passing is less repeatable in both position groups and both season
pairs, at both sample thresholds. It also has weaker association with the common
following-season assist-rate target in all those comparisons. This argues against
discarding secondary assists from the broad Passing estimate on this evidence.
Half credit stays close to the baseline, with mixed small differences in future
production association; it does not establish a reason to change the recipe.

Adjusted-finishing Shooting has no consistent advantage for forwards. For
defensemen, all three finishing candidates have lower grade repeatability and
weaker following-season goal-rate association in both pairs and both sample
filters. The raw defense finishing residual barely repeats: with the larger
sample, its correlations range from .049 to .076 across these candidates/pairs,
versus .339 to .348 for forwards. This does not establish that defensemen lack
finishing skill; shot volume, the xG model and role differences remain plausible
contributors to the noisy measurement.

Practical decision: retain the baseline Passing/Shooting recipes. Display assist
mix and adjusted finishing as distinct measured profile details, with their
windows and opportunities, rather than promoting these candidates into the main
ability scores. Repeating a grade or predicting production is useful evidence,
but neither alone validates it as isolated capability.

## Acquisition and replay

`profile-calibration-inputs` acquires one selected season of NHL summary, bios,
realtime, penalties and TOI reports, plus MoneyPuck.com all-situation assist
credits and the published shot archive. It reuses the checksum-verified source
cache. It deliberately omits EDGE acquisition: its generated catalog is a
diagnostic artifact with complete baseline Passing/Shooting inputs, not a
replacement six-attribute production catalog.

MoneyPuck assist rows are accepted only when GP and primary+secondary totals
match NHL. Missing or mismatched rows remain unavailable, including when NHL
reports zero assists. Shot-quality goals/xG/attempts use the same goalie-in-net
archive population. Older published shooter IDs such as `8478178.0` are accepted
as whole numeric identities; fractional IDs remain invalid.

```powershell
# Run separately for each selected season: 20232024, 20242025, 20252026.
target/native-validation/debug/icelines.exe --no-setup profile-calibration-inputs --season 20232024 --cache-dir target/goalie-refresh/cache --output-dir target/attribute-calibration/history-inputs/20232024
target/native-validation/debug/icelines.exe --no-setup --no-live profile-calibrate --season 20232024 --catalog target/attribute-calibration/history-inputs/20232024/rinkcraft-stat-attributes.json --season-package target/attribute-calibration/history-inputs/20232024/season-package.json --output-dir target/attribute-calibration/history-recipes/20232024
target/native-validation/debug/icelines.exe --no-setup --no-live profile-carryover --reports target/attribute-calibration/history-recipes/20232024/calibration.json target/attribute-calibration/history-recipes/20242025/calibration.json target/attribute-calibration/history-recipes/20252026/calibration.json --output-dir target/attribute-calibration/history-report
```

Choose new output directories for replay. Add `--offline --no-live` to the input
command to require retained captures. Capture manifests record URLs, access dates
and byte hashes; season reports record catalog/package hashes; the carryover
report records its input-report hashes. No random seed or simulation is involved.

Public source descriptions: [MoneyPuck downloads](https://moneypuck.com/data.htm),
with explicit MoneyPuck.com credit in both generated reports and browser views.

## Completed validation

- Seven Core tests pass across the candidate and carryover engines. They cover
  ID matching despite row reordering, shared future outcomes, both-season sample
  thresholds, missing comparisons, attrition, season gaps, duplicate players and
  recipe drift, alongside the existing rank/prior tests.
- Four source tests pass, including same-GP/total assist reconciliation, absent
  versus recorded-zero rows, mixed situations, duplicate actors, wrong seasons
  and whole numeric shooter IDs versus invalid fractional IDs.
- Locked native CLI build passes. Python inspection independently reconciles all
  recorded production rates with NHL counts/time and verifies all 40 actual-data
  cohort sizes and future-production correlations to within 1e-12. Python is
  verification only; production acquisition, scoring and comparison are Rust.
- Full offline replay reproduces all three catalogs, packages, capture manifests,
  recipe reports and the carryover report byte for byte. Final report SHA-256:
  `affeac5e5feb36955a02ebfaab74473ca3a6a951105dc967e0b77a313a3662fb`.
- Browser acceptance covers both season pairs, both workload filters, every
  attribute/position filter combination, displayed correlations, no page errors
  and 390px page overflow. Desktop/mobile screenshots were inspected.

The local report is served at `http://127.0.0.1:8780/calibration/history/`.
No hosted build, WASM carryover export or full application integration is claimed.
