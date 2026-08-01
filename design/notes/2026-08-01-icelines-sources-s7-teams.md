# IceLines Sources S7 — Team Catalog and Provider Normalization

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; workspace/repository CI remains the merge gate

## Delivered

- Added `icelines_core::teams::ALL_NHL_TEAM_ABBREVIATIONS` as the shared pure
  franchise-abbreviation catalog and proved it matches the existing named
  `CANONICAL_TEAMS` catalog exactly.
- Moved season-scoped NHL membership and ESPN-to-NHL abbreviation
  normalization into `icelines_sources::teams`.
- Preserved `icelines_fetch::teams::{ALL_NHL_TEAMS,
  nhl_teams_for_season, espn_to_nhl_abbrev}` as compatibility re-exports.
- Kept bundle/snapshot corroboration tests in `icelines-fetch`, where filesystem
  and bundled-data ownership belongs.
- Retained fail-closed unknown-code behavior and the historical Seattle,
  Arizona/Utah, Phoenix, and Atlanta semantics without adding team exceptions
  to downstream features.

## Verification

```text
cargo test -p icelines-core teams --lib
22 passed; 0 failed

cargo test -p icelines-sources teams
2 passed; 0 failed

cargo test -p icelines-fetch teams --lib
20 passed; 0 failed
```
