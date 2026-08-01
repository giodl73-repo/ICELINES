# IceLines Sources S7 — NHL Shift-Chart DTO Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-contract slice

## Delivered

- Moved official shift-chart response and interval-row DTO ownership to
  `icelines_sources::nhl::shift_chart`.
- Preserved the public `icelines_fetch::shift_chart` paths through compatibility
  re-exports and kept endpoint acquisition in `NhlApiClient`.
- Kept overlap/chemistry aggregation and its report types in `icelines-fetch`;
  those are feature-domain composition, not provider parsing.
- Added a direct source test for camel-case decoding and nullable provider
  duration while retaining the existing overlap and response tests.

This cut makes official deployment payloads reusable without moving product
interpretation into the source boundary.

## Verification

```text
cargo test -p icelines-sources shift_chart
1 passed; 0 failed

cargo test -p icelines-fetch shift_chart --lib
existing response and overlap tests pass
```
