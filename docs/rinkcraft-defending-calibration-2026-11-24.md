# Defending: activity and suppression — 2026-11-24

This retrospective Rust study compares the existing Defending estimate with four
public-data alternatives across 2023–24, 2024–25 and 2025–26. Weights are fixed
hypotheses; no production card grades or simulator values change.

## Recipes and scope

The baseline is 50% blocks/60, 30% takeaways/60 and 20% fewer penalties/60, using
the existing favorable percentiles and all-strength NHL minutes. Its likely
meaning is recorded defensive activity and discipline, not isolated prevention.

Candidates rank same-season forwards and defensemen separately:

1. **Expected-goal suppression:** lower five-on-five on-ice xGA/60.
2. **Attempt suppression:** lower five-on-five unblocked attempts against/60.
3. **On/off suppression:** off-ice xGA/60 minus on-ice xGA/60. Higher is favorable;
   this sign is the reverse of a conventional on-minus-off against-rate delta.
4. **Activity + suppression:** 50% existing activity percentile, 25% lower on-ice
   xGA/60 percentile and 25% on/off suppression percentile.

The 1–20 conversion and quantized midranks are unchanged. Candidate reference
eligibility is 10 GP and 100 **five-on-five minutes**, with at least 20 measured
peers. The blend's two suppression components use the same measured reference
population. Baseline reference percentiles retain their original all-strength
population; they are not silently recomputed against the new exposure filter.

MoneyPuck season CSVs supply all numerator/denominator pairs for five-on-five
results. `icetime` and `timeOnBench` are seconds. Rows must match the selected
season and official NHL GP. Missing or mismatched exposure is explicitly flagged,
displayed as unavailable, and excluded from reference/paired cohorts. True zero
events with positive exposure remain zero. On/off is unavailable with no bench
time; an absent denominator never implies zero suppression.

The player table shows PK time share from NHL all-strength workload and
DZ/(DZ+OZ) faceoff shift starts from the five-on-five CSV. Neutral-zone and
on-the-fly starts are excluded from that ratio. These are context, not fitted
adjustments. Five-on-five isolates strength state; on/off adds a team comparison.
Neither removes teammate, opponent, score, deployment, aging or injury effects.
These are team outcomes while a player is present, not attribution of goals or
defensive mistakes to that player.

Source definitions: [MoneyPuck glossary](https://moneypuck.com/glossary.htm) and
[player dictionary](https://peter-tanner.com/moneypuck/downloads/MoneyPuckDataDictionaryForPlayers.csv).
The [public downloads](https://moneypuck.com/data.htm) are credited as MoneyPuck.com
throughout the source records and both preview pages.

## Findings

The three diagnostic catalogs contain 924, 920 and 940 official skaters. Admitted
five-on-five records cover 924, 920 and 922 respectively. Missing records remain
unscored, rather than borrowing all-strength minutes for a five-on-five rate.
All 2,784 existing six-score arrays are identical to the preceding study inputs.

Four recipes × two position groups × two sample thresholds × two season pairs
produce 32 comparison rows. Each uses identical players for baseline/candidate,
the same next-season five-on-five xGA/60 target, and the same workload criterion
in both seasons. As the target is lower-better, its sign is reversed before
correlation. Positive correlations therefore mean association with lower future
xGA/60. Unknown exposure, position-group changes and inadequate next-season
workload are excluded and reported as lacking an eligible same-group next row.

For the 40 GP / 500 five-on-five minute sample, favorable future-outcome
correlations are:

| Recipe | F: 2023–24 → 2024–25 (317) | D: 2023–24 → 2024–25 (163) | F: 2024–25 → 2025–26 (315) | D: 2024–25 → 2025–26 (159) |
| --- | ---: | ---: | ---: | ---: |
| Current activity estimate | .167 | −.101 | .134 | .158 |
| Expected-goal suppression | .430 | .371 | .283 | .339 |
| Attempt suppression | .494 | .523 | .327 | .320 |
| On/off suppression | .316 | .210 | .225 | .282 |
| Activity + suppression | .407 | .231 | .285 | .392 |

All 32 rows show a stronger favorable association than the baseline. These are
overlapping cohorts and correlated recipes, not 32 independent replications.
No confidence intervals or causal individual-skill claim is supplied. Team-system
persistence could explain part of the predictive association; on/off does not
solve that confounding.

The activity grade is more repeatable than these candidate grades in the larger
sample. Example: defense repeatability is .679 baseline versus .499 for attempt
suppression in the first pair; .620 versus .535 in the second. Stability alone
would favor the activity measure, while its association with future chance
prevention is weak. Those are different evaluation questions.

The 2025–26 record also distinguishes player styles: Fox is activity 12 versus
experimental xGA suppression 19; Slavin is activity 14 versus attempt suppression
20. Beniers is xGA suppression 13, attempt suppression 6 and on/off suppression 15.
These are contextual candidate grades, not newly validated ability ratings.

Practical implication: represent **defensive activity** and **chance suppression**
separately in the profile. Blocks and takeaways remain useful task/feat evidence;
they should not stand alone as proof that the team allows fewer chances. A future
Defending recipe can incorporate prevention, but this study does not automatically
promote the trial blend. A useful next sensitivity check is players who change
teams, followed by stronger deployment/teammate controls where available.

## Reproduction

Run one selected season at a time using the retained public-source cache:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-calibration-inputs --season 20232024 --cache-dir target/goalie-refresh/cache --output-dir target/defending-attribute-study/inputs/20232024 --offline
target/native-validation/debug/icelines.exe --no-setup --no-live profile-calibrate --focus defending --season 20232024 --catalog target/defending-attribute-study/inputs/20232024/rinkcraft-stat-attributes.json --season-package target/defending-attribute-study/inputs/20232024/season-package.json --output-dir target/defending-attribute-study/recipes-v2/20232024
# Repeat those two commands for 20242025 and 20252026, then:
target/native-validation/debug/icelines.exe --no-setup --no-live profile-carryover --reports target/defending-attribute-study/recipes-v2/20232024/calibration.json target/defending-attribute-study/recipes-v2/20242025/calibration.json target/defending-attribute-study/recipes-v2/20252026/calibration.json --output-dir target/defending-attribute-study/report-v2
```

Use new output directories for replay. Source manifests retain URLs, capture
dates and byte hashes; recipe/carryover reports retain their input hashes. This
is deterministic analysis with no simulation seed. The diagnostic catalog omits
EDGE acquisition and must not replace the full production attribute catalog.

Defending reports use calibration/carryover **v2**, with explicit outcome
direction and exposure metadata. Legacy Passing/Shooting v1 reports remain
readable with their original higher-better production outcomes. Old engines
reject v2, preventing them from silently treating higher xGA as favorable.

Local player comparison: `http://127.0.0.1:8780/defending/` (2025–26 window).
Season-pair comparison: `http://127.0.0.1:8780/defending/history/`.

## Completed validation

- Seventeen focused Rust tests pass: four existing candidate tests, four carryover
  tests, three defensive candidate tests, two defensive source tests and four
  existing skater source tests. They cover source GP/strength/exposure, zero and
  missing values, position reference separation, minimum peer counts, blend
  arithmetic, lower-better target direction and frozen outcome metadata.
- Locked native CLI build passes. Python independently verifies all 32 actual
  paired-cohort counts and favorable outcome correlations to within 1e-12.
  Acquisition, scoring and comparison remain Rust; Python is verification only.
- Full offline replay produces identical input catalogs, packages, capture
  manifests, candidate reports and final carryover artifacts. Final v2 JSON
  SHA-256: `8ee321da1253732fc3f1cd6fb0aaca6f714a2abf6b5873855059c1860feebc6c`.
- Re-reading the three earlier v1 season reports preserves all numeric fields
  of their 40 Passing/Shooting comparisons exactly.
- Browser acceptance passes both new v2 defensive pages and legacy v1 data with
  the updated templates: recipe/group/search/sort/sample controls, displayed
  correlations, five-on-five exposure labels, missing exposure as unavailable,
  no page errors and 390px page overflow. Screenshots were inspected.

No hosted publication, full app integration or WASM study export is claimed.
