# IceLines Sources S7 — Position Boxscore Contract Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-contract slice

## Delivered

- Moved player game-log and position-boxscore serde contracts to
  `icelines_sources::nhl::position_boxscore`.
- Kept HTTP acquisition, best-effort multi-game traversal, position-profile
  construction, and linemate-profile construction in `boxscore_client`.
- Preserved required-field schema failures and optional TOI/shift behavior by
  using the same DTOs directly in the existing generic JSON acquisition path.
- Added direct source tests for both provider payload families.

This eliminates another private provider schema from `icelines-fetch` without
misclassifying profile aggregation as source normalization.

## Verification

```text
cargo test -p icelines-sources position_boxscore
2 passed; 0 failed

cargo check -p icelines-fetch
passed
```
