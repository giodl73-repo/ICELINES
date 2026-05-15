# Pulse 02 - Shot-Quality Proxy Implementation

## Goal

Implement the pure core IceLines inside-shot proxy defined in
`FINISH-INVENTORY.md`, with known-value tests, before any trend rows or shot
streak leaderboards consume it.

## Governing roles

- **scout**: expose descriptive hockey language only: crease, inside, slot,
  outside, unknown. Do not call the proxy expected goals or danger parity.
- **edge**: missing coordinates must classify as `unknown`; negative x
  coordinates must be symmetric with positive x; threshold boundaries must be
  exact.
- **bench**: tests must use hand-calculated distances and bucket expectations.
- **wire**: implementation is pure core logic over already parsed event rows;
  it must not read cache or fetch data.

## Owned scope

1. Add core structs/functions for the IceLines inside-shot proxy.
2. Project proxy buckets from `ScoringEventInput.location`.
3. Add L0 known-value tests for thresholds, missing coordinates, and symmetry.
4. Export the proxy types through `icelines-core`.

## Non-goals

- No web/API surface changes in this pulse.
- No trend rows or shot streak aggregation yet.
- No third-party xG labels or model claims.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo clippy -p icelines-core -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-measure-the-finish design\waves\PHASES.md --errors-only`
