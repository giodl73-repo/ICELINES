# Browser WASM pulse 23 — distributed binary/native parity

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-02 selected parity evidence.
Previous turn was progress: actual exported-file verification and package
reimport. Goal remains active.

## Change

Added `icelines-fetch/examples/browser_parity_goldens.rs`. It loads the fixed
completed 2024-25 regular/playoff bundles through the native `load_into_repo`
path using an empty temporary SnapshotStore. It emits expectations from native
repository views, without calling BrowserEngine or its row mapping. Filter
predicates are explicit numeric comparisons, independent of the browser query
parser. Native shared sort/accessor contracts remain authoritative.

Added `scripts/verify-browser-native-parity.mjs`. It runs that native example,
loads the actual distributed WASM binary and each matching catalog package,
then compares every ordered result row and context/floor against expectations.
The `build-browser.mjs --check` pipeline now runs this gate, preserving the
caller-selected offline Cargo mode. Temporary data stays under target/.

## Verification

`node scripts/verify-browser-native-parity.mjs --offline` passed:

- 270 cases: regular/playoff × nine combined skater/goalie sorts × five floors
  (default, explicit 0, 1, 5, 10) × three filters per kind.
- Skater filters: empty, `p >= 100`, `g >= 30`; goalie filters: empty,
  `goalie-games >= 10`, `goalie-games >= 100`.
- 33,613 complete ordered row comparisons; 90 empty-result cases.
- Every projected field: ID/name/team/position, GP/goals/assists/points, pace,
  goalie wins/save percentage/GAA, hits and blocks, including nulls and numeric
  precision after serialization. Season/type and effective floor also match.

The Rust example was formatted with rustfmt; Node syntax check passed. Cargo
reported incremental-cache hard-link fallback warnings but completed normally.
No production engine or browser UI changes were needed. Native/WASM parity
includes actual native loading and actual WASM execution rather than compiling
BrowserEngine twice on the host.

## Limits and remaining gates

This is fixed bundled-season integration parity, not independent mathematical
proof: sort comparators and domain accessors intentionally share the native
contracts. Hand-audited sort goldens remain a separate distribution check.
It does not establish every CLI route, arbitrary imported/live reports,
all historical seasons, traded-player adversarial corpus breadth, deployed
browser CORS, offline reopening, mobile/resource acceptance or Pages release.
Those broader requirements remain open; no complete parity or ship claim.
