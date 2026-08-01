# IceLines Sources S7 — AHL HockeyTech Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-parser responsibility split

## Delivered

- Added `icelines_sources::ahl::hockeytech` as the deterministic owner of the
  Statview JSONP envelope, season and team catalogs, roster rows, skater rows,
  goalie rows, provider-contamination filtering, compatible roster-row
  deduplication, and normalized snapshot validation.
- Moved the normalized AHL snapshot, team, roster-player, skater, goalie, and
  provider catalog DTOs into `icelines-sources`.
- Preserved the established `icelines_fetch::ahl` type paths and public
  `parse_jsonp` facade. Fetch retains URL construction, HTTP/FLETCH execution,
  batching, timestamps, affiliation lookup, identity review, and projection
  composition.
- Retained provider-local identity semantics: HockeyTech player IDs never
  become NHL player IDs without the separately reviewed crosswalk.

## Verification

```text
cargo test -p icelines-sources "ahl::hockeytech"
1 passed; 0 failed

cargo test -p icelines-fetch ahl --lib
96 passed; 0 failed

cargo test -p icelines-sources --test architecture_dependencies
2 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
