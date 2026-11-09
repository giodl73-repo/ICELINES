# Numerical snapshot/source selection validation — 2026-11-08

CHG-136 / WP-RINKCRAFT-12 / IF-RINKCRAFT-NUMBERS-001. Baseline `97efa86`
plus this working tree. Contract: `design/specs/rinkcraft-snapshot-selection.md`.
Rinkcraft decision 0016, selection protocol 0.1 and producer-applied ten-lens
self-review govern interpretation. No new source, class or empirical validity. No 2026–27 numerical source is
supplied; latest available here means 2025–26, not a live-current-season guarantee.

## Inputs and changed behavior

Unchanged `C:/src/Rinkcraft/outputs/numerical-history/2026-11-08-01/history.json`
(three supplied seasons, 74 player-seasons, 1,020 measurements), legacy 2024–25
numerical/style catalog and class catalog. Native source preference is HISTORY,
then NUMBERS; browser uses the equivalent optional authenticated archive/legacy
asset. Source selection has no provider reads, backfill or cache mutation.

Primary cards default to 2025–26 for 24 players; Barkov has an explicit 2024–25
older fallback because his latest supplied player row is absent. No reason for
that absence is inferred. Exact year requests never substitute a different year.
The primary fallback does not alter the full-window measured assessment: Barkov's
recent pair and repetition headline remain unresolved. All tracking stays in its
measured year; 2025–26 McDavid has two style readouts, 2024–25 has five.

Native numbers JSON, comparison and HTML default to the newest supplied source,
with exact `season=YYYYZZZZ` selection. Browser selector and card compare action
use the same Rust policy. A missing comparison player cannot fall back to another
year. Explicit requests retain player identities, with missing data/error stated.
A card compare action chooses a compatible partner in that card's source year.

## Checks

- Core Rinkcraft: 24 tests pass, including new latest/fallback/exact missing,
  filter-before-selection, reversed/empty windows, missing comparison, explicit
  source years and legacy/archive original-text parsing assertions.
- Focused core/fetch/web/WASM library Clippy with warnings denied; TypeScript;
  five frontend tests; native CLI/web and full browser builds passed.
- Full browser build verifies actual WASM queries and all 75 prepared packages.
  Rust 1.95.0 / wasm-bindgen 0.2.118; build metadata records parent `97efa86`
  with dirty working tree, not a clean final-commit build. Final JS/asset pass:
  40 authenticated shell assets; shell plus largest package gzip 639,266 bytes,
  within the unchanged 10 MiB budget. No publication or post-commit rebuild.
- `rinkcraft-snapshots.mjs`: 25 exact native/WASM snapshot objects (24 latest,
  one labeled fallback), three source comparison seasons, missing exact/player
  refusal, bounded and legacy source selection, card-to-comparison period,
  both-side unavailable selection, native no-JS, browser offline and 390 px checks.
- `rinkcraft-integrated.mjs`: numerical cards and three frozen 2024–25 native/
  WASM comparisons, current primary views, temporal/playoff guards, unscouted
  numerical card, offline and mobile passed. Frozen pilot comparisons now
  explicitly request their historical source season.
- Historical source/style tests retain all 25 cards, 345 original measures and
  47 style readouts; four exact style comparisons, source/context/legacy guards,
  goalie gaps, no-JS/offline/mobile passed. Frozen numerical test verifies all
  345 formula/value/rank cells and original source hashes explicitly in 2024–25.
- Assessment/history tests pass: all 25 exact CLI/WASM assessment objects, five
  native cases, 1,020 annual values/components, missing latest suppression,
  temporal/competition/legacy/offline/mobile behavior. Resident player history
  regression passes four regular/playoff windows, exact values and stale-dialog
  behavior; it does not establish full-career or phone/memory acceptance.
- Screenshots inspected: native missing-latest card and latest browser McDavid.
  Class identity remains prominent; numerical period/fallback note, dated
  measured summaries and source credit are visible. No fabricated tracking data.

Ignored logs/results/screenshots: `target/rinkcraft-selection-*`,
`target/rinkcraft-snapshot-check/` and existing focused E2E result directories.
Deterministic selection/subtraction; seed/trials/predictive split N/A.
One UI regression (card action retaining incompatible old partner) was fixed
and the integrated test rerun. A style run with an unpopulated catalog was rerun
successfully. The atlas run interrupted by the owned-server rebuild was rerun completely:
25 native class/stat cards, 26 atlas links, actual WASM class selection and
skater/goalie atlas opening passed. The expanded snapshot test reopens the
collapsed package panel before loading another year, and explicitly chooses an
available comparison partner after switching out of a year that includes Barkov. These are software/display checks,
not evidence of role/skill/style taxonomy validity.

Final expanded snapshot run passed in full. Five fresh browser contexts initialized
the source selector without page errors; the acceptance test explicitly waits for
initial readiness before opening background native test pages. Both previews
remain loopback-only on 8765 (native) and 8766 (browser/WASM).
