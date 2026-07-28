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
4. Conversion rows expose `expected_hit`, `breakout`, `miss`, and `developing`
   comparison classes. Program rows expose class counts without changing rank
   floors or hiding blockers.
5. The 2022-23 frozen cohort replay covered 247 players and 32 organizations.
   All had official performance authority; 19 organizations cleared the
   independent cohort-size and baseline-confidence ranking gates.

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
