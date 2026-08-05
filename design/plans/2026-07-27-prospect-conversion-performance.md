# Prospect Conversion Performance Authority

**Date:** 2026-07-27  
**Status:** Complete  
**Specification:** [`../specs/prospect-development-study.md`](../specs/prospect-development-study.md)

## Outcome

Turn the frozen prospect-program baseline into an honest historical learning
loop. IceLines now derives later NHL quality from official player landing
histories, keeps position and sample size explicit, identifies hits, breakouts,
misses, and still-developing results, and makes those facts reusable by JSON,
CLI, future cards, fantasy, and simulation consumers.

## Delivered

1. `prospect_conversion_performance.v2` freezes the baseline/outcome horizon,
   scoring method, source coverage, player samples, component weights, raw
   metrics, normalized values, confidence, and official evidence URLs.
2. Forwards, defensemen, and goalies use separate quality models. Small NHL
   samples are reliability adjusted; verified zero-game histories are observed
   zeros; missing official inputs fail closed.
3. `prospect-conversion` derives performance automatically and can save it with
   `--performance-out`, or replay an authored v2 document with `--performance`.
   It can also save the exact adapted `prospect_conversion_input.v2` cohort with
   `--input-out`; retained calibration proofs must keep input, performance, and
   board artifacts together. `prospect-conversion-replay` rebuilds the board
   directly from the retained input through the canonical core builder.
   `--archive-out` is the preferred retention path: the typed
   `prospect_conversion_archive.v1` stores all three siblings, fingerprints
   each one, and fails validation if the board cannot be reproduced.
4. Conversion rows expose `expected_hit`, `breakout`, `miss`, and `developing`
   comparison classes. Program rows expose class counts without changing rank
   floors or hiding blockers.
5. The 2022-23 frozen cohort replay covered 247 players and 32 organizations.
   All had official performance authority; 19 organizations cleared the
   independent cohort-size and baseline-confidence ranking gates.
6. A separately labeled August retrospective replay recovered the frozen
   2022-23 AHL source population under an age-24 ceiling and retained a
   fingerprinted 543-player, 32-organization archive through 2025-26. It does
   not overwrite or impersonate the earlier 247-player proof.

## Validation gates

- Focused core conversion tests, including horizon filtering and confidence
  shrinkage.
- CLI parsing regression for derived/performance-output options.
- Real all-organization replay with a separately frozen performance document.
- Workspace format, check, test-slice, audit, release, smoke, and packaging
  gates before the release tag.

## Remaining boundary

The conversion document is a UI-neutral data source, not a native prospect card
renderer. Web/TUI visualizations may consume it later, but may not recompute its
hockey logic. Larger authored cohorts and longer historical windows should be
added before using thin organization slices as scouting conclusions.

## 2026-08-04 follow-up

`prospect_arrival_calibration.v1` now exposes a leakage-safe, same-position
nearest-cohort arrival base rate for simulation consumers. It requires a
retained earlier conversion board, rejects the target from its own outcome
cohort, applies minimum-sample and signal-distance gates, and shrinks the local
rate toward the complete position cohort. The missing July 247-player proof was
not reconstructed or relabeled. Instead, a new explicit retrospective cohort
was built from the frozen 2022-23 AHL snapshot, sealed at 543 players across all
32 organizations, and retained as a fingerprinted archive. The CLI can now
derive a current target input from one canonical skater career study and retain
that derivation with `--input-out`. Smits' 29.06 attention-free signal calibrated
to 57.3519% NHL arrival from 50 nearest defenseman comparables and now supplies
the Rangers scenario's historical prospect-arrival authority. The scenario's
full +2.2 impact is separately bound to established role: the 22% neighbor
establishment rate shrinks toward the 18.9024% complete-defenseman rate for a
21.1150% cumulative three-season probability. A constant-hazard horizon
projection reduces the applied 2026-27 event probability to 7.6015%. The
11-of-31 established-given-arrival share remains descriptive rather than
equating one NHL game with a full breakout.
