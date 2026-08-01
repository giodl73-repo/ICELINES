# IceLines Sources S7 — NHL Standings Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete parser slice; the mixed `nhl_api` module remains in progress

## Delivered

- Moved NHL standings payload interpretation and normalized
  `NhlStandingsRow` ownership to `icelines_sources::nhl::standings`.
- Kept date-specific endpoint acquisition, retry policy, and the
  `NhlApiClient` methods in `icelines-fetch`.
- Preserved `icelines_fetch::nhl_api::{parse_standings, NhlStandingsRow}` as
  compatibility re-exports.
- Preserved the existing `TeamStandingInput` projection, localized provider
  field fallbacks, conference expansion, and missing-points-percentage
  calculation.

This is a partial decomposition of the mixed `nhl_api` module, not a completed
whole-module migration. The module inventory therefore retains its
`split_transport_parser` disposition.

## Verification

```text
cargo test -p icelines-sources standings
2 passed; 0 failed

cargo test -p icelines-fetch standings_tests --lib
2 passed; 0 failed

CARGO_INCREMENTAL=0 cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
