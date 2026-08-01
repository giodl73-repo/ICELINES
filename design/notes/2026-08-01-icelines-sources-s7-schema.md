# IceLines Sources S7 — Provider DTO Schema Extraction

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; workspace/repository CI remains the merge gate

## Delivered

- Moved the NHL stats, bios, roster, goalie, contract, pagination, localized
  string, and ESPN transaction DTO schemas plus serde validation to
  `icelines_sources::schema`.
- Replaced `icelines-fetch/src/schema.rs` with a compatibility facade, keeping
  every existing `icelines_fetch::schema::*` import valid.
- Moved the core `Tier1Row` implementation for `SkaterTimeOnIce` with its local
  DTO owner. This preserves Rust coherence while keeping report loading and
  snapshot I/O in `icelines-fetch`.
- Re-ran source schema tests, legacy fetch schema tests, the end-to-end
  integration pipeline, and all 42 stats-loader integration tests.

This completes every module classified `move_sources` in the measured
inventory. Mixed transport/parser and source/domain modules remain explicit
split slices; the extraction does not authorize moving them wholesale.

## Verification

```text
cargo test -p icelines-sources schema
5 passed; 0 failed

cargo test -p icelines-fetch schema --lib
6 passed; 0 failed

cargo test -p icelines-fetch --test integration_pipeline
10 passed; 0 failed

cargo test -p icelines-fetch --test stats_loader
42 passed; 0 failed
```
