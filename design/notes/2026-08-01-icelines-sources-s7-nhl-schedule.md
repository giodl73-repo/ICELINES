# IceLines Sources S7 — NHL Schedule Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete parser slice; the mixed `nhl_api` module remains in progress

## Delivered

- Moved `ScheduledGame`, its result/state helpers, and game payload parsing to
  `icelines_sources::nhl::schedule`.
- Kept schedule endpoint selection, HTTP acquisition, retries, and game-week
  traversal in `NhlApiClient`.
- Preserved the public `icelines_fetch::nhl_api::ScheduledGame` path and the
  fetch-local parser seam used by existing tests.
- Preserved historical and current playoff-series field fallbacks, including
  seed-relative win counts and `gameNumberOfSeries` handling.

This remains a partial `nhl_api` decomposition. Boxscore, play-by-play, and
playoff-bracket parser families still require separate compatibility cuts.

## Verification

```text
cargo test -p icelines-sources schedule
2 passed; 0 failed

cargo test -p icelines-fetch parse_game_tests --lib
6 passed; 0 failed

CARGO_INCREMENTAL=0 cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
