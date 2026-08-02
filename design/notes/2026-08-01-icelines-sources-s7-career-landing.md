# IceLines Sources S7 — Career Landing Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-parser responsibility split

## Delivered

- Completed official player-landing parser ownership in
  `icelines_sources::nhl::player_landing`: career history, dated organization
  observations, contract hints, draft facts, and award rows are source-owned.
- Kept awards parsing UI-neutral by returning `PlayerAwardRow` values. The
  fetch facade still supplies player identity and `ViewContext` to construct
  `PlayerAwardsView`.
- Retained career/award filesystem stores, atomic persistence, live
  augmentation, and pre-NHL feature filtering in `icelines-fetch`; those are
  persistence and consumer-domain responsibilities, not provider parsing.
- Preserved all established `icelines_fetch::career_landing` entry points.

## Verification

```text
cargo test -p icelines-sources player_landing
5 relevant tests passed; 0 failed

cargo test -p icelines-fetch career_landing --lib
20 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
