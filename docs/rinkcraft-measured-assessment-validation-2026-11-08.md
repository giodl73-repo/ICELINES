# Visible measured assessment validation — 2026-11-08

CHG-135 / IF-RINKCRAFT-NUMBERS-001. Baseline `ab76d65` plus this working tree.
Contract: `design/specs/rinkcraft-measured-assessment.md`. Rinkcraft protocol 0.1,
decision 0015 and ten-lens producer-applied review govern interpretation. No new
sources, player labels or independent class/skill/style validation.

## Input and reproduction

Unchanged archive:
`C:/src/Rinkcraft/outputs/numerical-history/2026-11-08-01/history.json`, also
ignored browser `public/rinkcraft-history.json`. Three supplied seasons, 25-player
cohort, 74 reported player-seasons, 1,020 numerical measurements. Primary 345
measurements, 47 style readouts and class catalog remain unchanged.

```
icelines --no-setup --no-live profile-assess --history C:/src/Rinkcraft/outputs/numerical-history/2026-11-08-01/history.json --player 8478402
```

Repeated for all 25 IDs from the existing numerical cohort; ignored outputs:
`target/rinkcraft-assessment-pilot/{id}.json`. No network/cache mutation by the
new CLI command. Valid explicit cutoff produced two seasons; malformed `2024`
bound rejected with YYYYZZZZ guidance; unknown player produced JSON null.
Seed/trials/splits N/A: deterministic descriptive selection/subtraction.

## Observed summaries

- McDavid repeated: primary assists/60 (minimum P86.3), individual xG/60 (P84.1);
  both P80+ in all three supplied ranked years. Largest recent shortlist shift:
  primary assists P99.4 to P86.3 (-13.1 pp), raw 1.17 to 0.80 /60.
- MacKinnon repeated lead: SOG/60 (minimum P98.0), then primary assists (P96.3).
  Largest recent shortlist shift: individual xG/60 P67.8 to P92.0 (+24.1 pp),
  raw approximately 0.78 to 0.93 /60.
- Makar repeated: individual xG/60 (minimum P97.7), SOG/60 (P94.4).
- Hellebuyck has no all-supplied-year P80 shortlist headline. Largest recent
  shortlist shift is observed save-fraction rank P95.1 to P52.8 (-42.3 pp),
  with raw save percentage 92.49% to 89.53%. Rank shift and raw percentage change
  are different quantities; neither identifies goalie mechanics or cause.
- Barkov is reported in 2/3 supplied seasons. Both headline repetition and latest
  pair assessment are withheld; no older pair substitutes for missing 2025–26.

These are descriptions under declared selection rules, not overall grades,
statistically significant changes, persistence estimates or class transitions.

## Executed checks

- Focused core Rinkcraft suite: 24 passed. New tests cover exact P80 inclusion,
  deterministic ties, selecting largest signed rank shift/raw difference, cutoff
  before selection, single/nonconsecutive years, missing latest without fallback
  and incompatible latest position groups.
- Native CLI/web builds passed, including the explicit-file CLI season validation.
  Library Clippy core/fetch/web/WASM with warnings denied; TypeScript; five focused
  frontend history/profile tests; format/diff checks passed.
- Full browser build and actual WASM/package verification passed. Rust 1.95.0,
  wasm-bindgen 0.2.118. Metadata records parent `ab76d65` plus dirty working tree.
  Final JS/asset build after visible source-credit refinement passed. Shell:
  40 authenticated assets, 75 packages, hash prefix `2cb991cfbf3d`; summed gzip
  shell 496,549 bytes; shell plus largest package 632,077 bytes, within 10 MiB.
  No post-commit clean-build claim or publication.
- `rinkcraft-assessment.mjs`: all 25 actual CLI/WASM history/assessment objects
  exactly equal; five exact native/WASM projections; repeated shortlist/minimum
  rules, goalie shortlist, missing-latest suppression, cutoff parity, legacy
  history rendering, always-visible summary with closed full table, no-JS native,
  offline browser and 390 px layout passed.
- Existing annual-history, integrated, style-comparison and style E2Es passed:
  all 1,020 values/source components and reference fingerprints, archive inputs,
  prior 345 primary measurements/47 style readouts, temporal/playoff/legacy guards,
  exact comparisons, all 25 primary cards and offline/mobile behavior.
- Five-case native no-JS regression passed. Native/browser screenshots inspected:
  class identity remains prominent; measured summary appears above the closed
  annual table and retains its own dates/coverage and MoneyPuck credit.

Ignored evidence: `target/rinkcraft-assessment-check/`, CLI pilot outputs and
`target/rinkcraft-assessment-browser-build.log`. Research retains run input/output
hashes and copied acceptance artifacts under its ignored measured-assessment run.

## Limits

Fixed shortlists and P80/minimum-two-year policy are editorial display choices.
Minimum annual rank selects a headline; it is not a new ability score or confidence
bound. Selection may change near thresholds, correlated signals are not independent,
and the maximum recent shift is selected descriptively without a significance
claim. Annual cohorts, usage, recording and provider/model uncertainty remain.
Scope and available-year counts stay separate. No passing mechanics, direction,
carry/dump choice, pursuit success, assignments, goalie technique or chemistry
is inferred, and documentary class hypotheses remain unchanged.
