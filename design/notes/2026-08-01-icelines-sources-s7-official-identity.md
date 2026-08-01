# IceLines Sources S7 — Official Identity Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete official-identity acquisition split

## Delivered

- Moved official NHL player-search parsing, exact-name and surname candidate
  discovery, landing-page identity corroboration, and deterministic catalog
  merging into `icelines_sources::ahl::identity`.
- Moved the official NHL landing identity/draft assertion parser into
  `icelines_sources::nhl::official_identity_landing`.
- Preserved the `icelines_fetch::ahl` and
  `icelines_fetch::official_identity_acquisition` compatibility paths while
  keeping HTTP execution, cache replay, draft-coordinate eligibility, and
  explicit reviewer finalization in `icelines-fetch`.
- Kept proposals fail-closed: search results supply only provider evidence;
  landing corroboration supplies birth date and identity evidence; neither
  parser approves a canonical identity or establishes organization control.

The subsequent HockeyTech provider split is recorded separately in
[`2026-08-01-icelines-sources-s7-ahl-hockeytech.md`](2026-08-01-icelines-sources-s7-ahl-hockeytech.md).

## Verification

```text
cargo test -p icelines-sources "ahl::identity"
1 passed; 0 failed

cargo test -p icelines-fetch ahl --lib
96 passed; 0 failed

cargo test -p icelines-fetch official_identity_acquisition --lib
7 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
