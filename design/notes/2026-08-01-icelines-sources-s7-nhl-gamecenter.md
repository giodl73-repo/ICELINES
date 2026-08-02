# IceLines Sources S7 — NHL Gamecenter Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete parser family; the mixed `nhl_api` module remains in progress

## Delivered

- Moved boxscore and play-by-play DTOs, parsing, and normalization to
  `icelines_sources::nhl::gamecenter` as one coupled semantic family.
- Kept gamecenter endpoint acquisition, raw-response return paths, retry
  behavior, and caller-owned persistence in `NhlApiClient`.
- Preserved all public `icelines_fetch::nhl_api` type and parser paths through
  compatibility re-exports.
- Preserved scorer identity fallbacks, goalie and skater stat parsing, TOI
  conversion, shot-family classification, event-owner team resolution,
  location gaps, empty-net goals, and penalty participant fields.
- Added direct source tests while retaining the legacy fetch and mock-endpoint
  suites as compatibility evidence.

Schedule, standings, playoff bracket, boxscore, and play-by-play interpretation
are now source-owned. Acquisition remains fetch-owned by construction.

## Verification

```text
cargo test -p icelines-sources gamecenter
2 passed; 0 failed

cargo test -p icelines-fetch boxscore_tests --lib
5 passed; 0 failed

cargo test -p icelines-fetch --test mock_nhl_api boxscore
5 passed; 0 failed
```
