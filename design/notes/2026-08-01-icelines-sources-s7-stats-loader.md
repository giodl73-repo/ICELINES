# IceLines Sources S7 — Tier-1 Report Decoder Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-decoder split; loading/domain policy remains fetch-owned

## Delivered

- Replaced the stats loader's local NHL `{data,total}` DTO with the
  source-owned tolerant Tier-1 report envelope and decoder.
- Preserved missing-vs-empty semantics, legacy missing-`total` compatibility,
  snapshot/bundled fallback, filesystem errors, and the per-row season-ID
  fence.
- Kept repository construction, missing-source banners, scoring projection,
  identity merging, and snapshot policy in `icelines-fetch`.

Provider DTOs and byte decoding are now source-owned; the loader remains the
product composition boundary.

## Verification

```text
cargo test -p icelines-sources bundled_artifact
4 passed; 0 failed

cargo test -p icelines-fetch stats_loader --lib
19 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
