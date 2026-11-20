# Multi-season record-reference replay and attribute coverage ledger

Exploratory implementation. This tests frozen observational references and reports
loaded component coverage; it does not fit or release 1–20 capability scores.
The calendar split follows the authored 2023–24/2024–25 epoch and 2025–26 target.
The earlier conditional-finishing pilot remains separate and unchanged.

## Reference design

- Fit on two named completed training seasons only; apply unchanged to a later
  completed season. Keep the partial current season as a separate monitor.
- Within each season/position/metric use the empirical midrank CDF; average the
  training seasons equally. This prevents a larger season receiving extra weight
  merely because more players cleared eligibility.
- Require at least 20 measured eligible records in every training-season stratum.
  Missing support produces no frozen percentile. Existing 10-GP/100-minute and
  25-SOG diagnostic settings come from the observational family experiment, not
  the registered component pilot's five-on-five coverage/precision requirements.
- Preserve both player-season support and unique-player counts. Repeated players
  are not independent samples. No confidence interval is supplied; an uncertainty
  design must respect player/game/time dependence before promotion.
- Keep forward/defense references separate for this record diagnostic. They do
  not implement the attribute contract's common task benchmark/fixed reference
  mix. Within-position ranks are not interchangeable capability scores.

The frozen CDF for x is the mean, over training seasons, of
100 × (records below x + half the ties) / measured eligible records in that year.
A new season is not forced to contain exactly 10% above the frozen P90 benchmark.
The epoch stores training-only source/input fingerprints, sorted reference members,
per-year source reference hashes, support and a model fingerprint. A test mutates
future records and verifies the fitted epoch is unchanged; altering a training
record changes it. Chronological overlap and duplicate seasons are rejected.

The rolling-origin 2023–24 → 2024–25 result is a record-reference stress test,
not an attribute grade predating the registered earliest target. The primary
replay fits both training seasons and evaluates 2025–26. Since these later seasons
were examined during earlier exploration, this is not an untouched confirmatory
holdout or prospective validation. Historical data were captured retrospectively;
point-in-time availability and source/model vintage are not established.
Partial-season maxima have unequal opportunity exposure relative to full seasons.

## Coverage ledger

Every source profile has six branch-appropriate attributes and every required
component ID. Definitions/weights/measurement contracts are stored once; outdated
historical coverage prose in the recipe is not copied as current coverage.

The ledger distinguishes recorded context from a loaded calibrated task estimate.
Skating maxima/bursts support Speed context; shot maxima support Velocity context;
raw goals/SOG support unadjusted Finishing outcome context. None supplies the other
facets or changes the primary five-on-five task scope. All facet rows retain
`unmeasured_task` / `not_calibrated`, and all headlines remain `unrated`.
This describes the loaded full-cohort capture, not the absence of other studies.
The earlier conditional finishing pilot and its source-vintage/position limits
are preserved; its readout is not silently inserted into a different season.

Goalies retain their six separate attributes and 18 facets. Available stopping
outcomes are recorded as broad context, not allocated as technique measurements
for Tracking, Positioning, Movement, Rebounds, Recovery or Puck Play.

## Reproduce

The manifest pins the existing exposure inventory and shared-observation bytes,
a baseline directory and train/validation/monitor seasons. The reader verifies
every baseline profile hash, identity and supplied observation linkage. Outputs
require a new directory; complete validation precedes output writes.

```text
cargo test -p icelines-sources --example rinkcraft_calibration_epoch --locked
cargo run -p icelines-sources --example rinkcraft_calibration_epoch --locked -- MANIFEST_JSON NEW_DIRECTORY
python tools/verify_rinkcraft_calibration_epoch.py MANIFEST_JSON OUTPUT_DIRECTORY NEW_RECEIPT_JSON
```

Three Rust tests cover temporal exclusion, training sensitivity, equal-season
weighting, inadequate support, order and season uniqueness. Three CLI negative
checks rejected split overlap, changed input hash and existing output. The
independent Python oracle checked all 3,941 profile hashes, 88,763 component rows,
18,639 reference-member checks, 10,178 frozen CDF results and 5,282 family decisions.
It recomputes frozen percentiles from stored observations, checks those against
source values with 1e-12 numeric tolerance, verifies source memberships/unique IDs,
family decisions, census and summaries. No capability score was released.
The final census groups counts by family; its clarification left the epoch model
and coverage ledger byte-identical to the first run. The final second output is
the retained deliverable; development output is not a publication.

Producer-applied lenses: methods (chronology, source vintage, dependence, exposure
and exploratory status), evidence custody (hash linkage and scoped context),
goaltending (outcomes are not technique) and consumer boundary (code in IceLines;
dated outputs in PUCK). These are self-review lenses, not an independent panel.
No research-repository definitions, public cards, native/WASM APIs, simulations,
main branch or hosted deployment changed.
