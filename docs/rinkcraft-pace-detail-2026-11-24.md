# Observed counts behind season pace — 2026-11-24

Estimated Record adds a collapsed “Observed record & pace” disclosure. It shows
the recorded count, observed GP and calculation for each projected count. The
front's rounded totals and per-game rates remain unchanged. This is arithmetic
explanation, without an additional source-audit layer.

Example from the retained Alex Killorn snapshot:

| Field | Observed | Arithmetic | Rounded front total |
|---|---:|---|---:|
| Shots | 33 | 33 / 24 GP × 84 games = 115.50 | 116 |
| Assists | 6 | 6 / 24 GP × 84 games = 21.00 | 21 |
| Goals | 3 | 3 / 24 GP × 84 games = 10.50 | 11 |
| Hits | 17 | 17 / 24 GP × 84 games = 59.50 | 60 |
| Blocks | 6 | 6 / 24 GP × 84 games = 21.00 | 21 |

## Implementation

The existing shared Rust pace calculator now supplies optional observed GP,
recorded count and preformatted calculation text. The observed count is recovered
from the full-precision per-game rate and GP only if it is finite, nonnegative,
within u32 and within 1e-6 of a whole count. Rounded or fractional rate inputs do
not receive a claimed whole observed count. They retain a rate-based formula.
Known zero counts remain zero. Missing rates or zero GP produce no calculation.
Percentages and average TOI receive no count calculation.

Metadata defaults to absent for older records. Catalog validation binds a supplied
observed GP to the containing stat profile. Native Askama and browser render the
Rust-projected strings; neither selects a new hockey scoring rule. No projected
numeric value, attribute grade, feat, task selection or simulation input changes.

The disclosure stays collapsed initially and is hidden in print, including when
the user leaves it open. Existing single-page Letter output remains intact.

## Frozen local validation

All 787 retained skaters have five observed count fields (3,935 fields), including
424 known zeroes. The existing admitted NHL snapshot remains the input; no new
source fetch occurred. Population projection preserves original achievement
objects and all retained integer attribute scores. Input catalog SHA-256:
`9fc4ee1625e407c88592bffe80c11bc0b6cc566105d282d4ae080cb86a574e77`.

- Native CLI/Askama, WASM and browser builds passed.
- 247 browser tests and 22 focused Rust tests passed, including exact arithmetic,
  fractional-rate withholding, zero/missing distinction, old-record decoding and
  observed-GP mismatch rejection.
- Five live cards (Killorn, MacKinnon, Fox, Aho, Beniers) passed displayed count,
  GP/formula and actual worker parity checks; older-catalog fallback, retained
  task/workload sections and mobile overflow checks passed without page errors.
- With details open, all five Letter PDFs independently passed one-page,
  612×792-point, primary-value preservation and hidden-detail checks using pypdf.
  The expanded Killorn disclosure was visually inspected.

Retained ignored helper: `target/pace-detail-study/project.mjs`. Acceptance:
`node icelines-browser/e2e/rinkcraft-pace-detail.mjs` against the existing 8780
preview; it does not start a server. No hosted publication or merge.

Preview: <http://127.0.0.1:8780/?player=8473986>.
