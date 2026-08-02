# IceLines Sources S7 — Bundled Artifact Parser Splits

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete parser/conversion splits

## Delivered

- Moved the complete historical `PlayoffsBundle` schema and bracket conversion
  module to `icelines-sources`, retaining the fetch module as a compatibility
  facade.
- Added source-owned decoders for bundled bios, skater stats, goalie stats,
  transactions envelopes, and playoff bundles.
- Routed embedded and installed artifact bytes through the same decoders while
  preserving fetch-owned `include_bytes!` tables, home-directory discovery,
  snapshot precedence, installed-bundle fallback, and stale transaction
  reclassification.
- Preserved serialized schemas and all existing historical data paths.

## Verification

```text
cargo test -p icelines-sources playoffs_bundle
7 passed; 0 failed

cargo test -p icelines-sources bundled_artifact
3 passed; 0 failed

cargo test -p icelines-fetch bundled::tests --lib
19 passed; 0 failed

cargo test -p icelines-fetch --test integration_pipeline
10 passed; 0 failed
```
