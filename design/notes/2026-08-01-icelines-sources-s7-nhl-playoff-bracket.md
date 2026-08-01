# IceLines Sources S7 — NHL Playoff Bracket Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete parser slice; the mixed `nhl_api` module remains in progress

## Delivered

- Moved playoff bracket DTOs, series helpers, and payload normalization to
  `icelines_sources::nhl::playoff_bracket`.
- Kept year-specific endpoint acquisition and retry behavior on
  `NhlApiClient` in `icelines-fetch`.
- Preserved all public `icelines_fetch::nhl_api` bracket types and
  `parse_playoff_bracket` through compatibility re-exports.
- Preserved both legacy nested-round and current flat-series shapes, rank and
  win-count fallbacks, winner inference, conference normalization, and
  historical per-game fields.

Boxscore and play-by-play remain the next coupled NHL parser family. The mixed
module inventory row remains `split_transport_parser` until all appropriate
families are separated.

## Verification

```text
cargo test -p icelines-sources playoff_bracket
2 passed; 0 failed

cargo test -p icelines-fetch --test mock_nhl_api playoff_bracket
8 passed; 0 failed

cargo test -p icelines-fetch --test integration_pipeline
10 passed; 0 failed
```
