# Possession and environment context validation — 2026-11-09

CHG-140 / WP-RINKCRAFT-16 / IF-RINKCRAFT-NUMBERS-001. Baseline `b9aaa18`
plus working tree. Contract: `design/specs/rinkcraft-context-evidence.md`.
Rinkcraft decision 0020 / context protocol 0.1 and producer-applied ten-lens review
govern interpretation. No independent panel or empirical role/skill/style validity.

## Change and input boundaries

EvidenceCard gains optional `context_evidence`: fixed source-record groups,
nullable original Measurements, explanations and available/ranked/declared counts.
Skaters expose takeaways/giveaways, penalties drawn/taken and on-ice xG for/against;
goalies expose their existing cumulative GSAx. Group/situation/period/exposure/
peers/source custody inherit from the parent card. Values/ranks are copied without
inversion, weighting or new statistical definitions. Missing, unavailable, unranked
and genuine-zero records remain distinct. These counts describe source coverage,
not measured roles or reliability.

Native/browser cards have compact context groups and evidence expanders. Comparisons
have a separate contextual table that reuses existing matched ComparisonRows for
subtraction; no added primary axes or headline candidates. The six/four primary
facets, leading/contrast/repeated-signal policies, class assignments and history
are unchanged. Defensive coverage, assignment ownership and goalie technique stay
unmeasured; team xGA cannot assign blame for a goal.

Unchanged retained inputs: three-season Rinkcraft archive (74 player-seasons,
1,020 measurements), legacy numerical/style catalog and authored class catalog.
No acquisition, source/claim addition or input/value/rank modification. Latest
supplied numerical year is 2025–26, not a live 2026–27 guarantee. Barkov's older
primary snapshot retains its own 2024–25 context; no forward fill.

## Executed checks

- Focused core Rinkcraft: 31 passed. New tests copy all original measurements,
  keep genuine zero available, preserve declared slots when absent, separate
  unranked values, retain raw penalty ranks, explain defender limits and use only
  total GSAx for goalie context. No skater records are assigned to goalies.
- Library Clippy core/fetch/web/WASM with warnings denied passed; TypeScript,
  five frontend tests and native CLI/web/full browser builds passed.
- Actual distribution verification: 40 authenticated shell assets, 75 packages,
  shell hash prefix `4e66b776bc54`; summed gzip 518,990 bytes, shell plus largest
  package 654,517 bytes, below unchanged 10 MiB budget. Rust 1.95.0 /
  wasm-bindgen 0.2.118. Build-info records parent `b9aaa18` plus dirty tree; no
  final-commit clean rebuild, publication or hosted CI claim.
- `rinkcraft-context.mjs`: 25 actual WASM cards contain 135 exact copied context
  Measurements (22 skaters x 6, 3 goalies x 1; each card's own selected year).
  Nine native/WASM full numerical cards and three complete comparison objects
  match exactly. Context keys never enter primary contrast headlines or alter
  primary facet/axis counts. Raw percentiles match the original Measurements.
- Missing metric, unavailable raw value and insufficient exposure retain six
  declared slots with 4/6 available and 3/6 ranked in the explicit mutation case.
  No invented zero, substitute metric or inverted penalty rank. Legacy cards
  without the optional field retain six primary facets and no context block.
- Native no-JS cards/comparisons, browser defender context and offline explicit
  peer handoff, 390 px widths and >=4.5:1 context text contrast in all three
  themes passed. Evidence limits/formulas/reference hashes remain inspectable.
  Native Fox and browser Fox/context screenshots inspected; main attributes
  remain above the compact supplementary groups.
- Existing attributes-first regression passed all 25 preserved vectors, nine
  exact native/WASM profiles and first-screen/feature/missing-transport guards.
  Contrast regression passed 133 compatible latest-source pairs, nine exact
  native/WASM comparisons and definition/coverage/legacy/offline/theme checks.
  Explicit card-comparison regression passed ten native pickers, three browser
  groups, IDs/year, class dates/hash, older fallback, unscouted/optional/stale
  source cases and no-JS/offline/mobile. Latest-source regression passed all 25
  exact native/WASM snapshots, period/bounds/missing/legacy and offline/mobile.

Both old previews were absent at startup; owned native and static loopback previews
were started after builds. Static verification uses native Python with queue 128.
No source-input repair or provider access was needed.

Ignored evidence: `target/rinkcraft-context-check/` and `target/rinkcraft-context-*`.
Research run records unchanged input hashes, actual commands, output/build/source
provenance. Seed/trials/predictive split N/A: display projection, not role prediction.
Neither software checks nor extra visible metrics establish individual defensive
skill, assignment compliance, stable talent, class fit or causality.
