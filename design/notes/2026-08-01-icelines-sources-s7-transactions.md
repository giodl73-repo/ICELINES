# IceLines Sources S7 — ESPN Transaction Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-parser/normalization split

## Delivered

- Moved raw ESPN transaction conversion to `icelines-sources`, including
  description sanitization, classification, season-aware team normalization,
  ET date bucketing, stable IDs, and unmapped-team warnings.
- Moved ESPN season-window generation and permissive page parsing with explicit
  additive-drift paths to the source crate.
- Preserved fetch-owned HTTP, content-type rejection, retry/backoff, circuit
  breaking, partial outcomes, FLETCH caching, and snapshot persistence.
- Restored the existing crate-visible helper facade used by both the ESPN client
  and FLETCH transaction batching.

## Verification

```text
cargo test -p icelines-sources transactions
16 passed; 0 failed

cargo test -p icelines-fetch transactions --lib
13 passed; 0 failed

cargo test -p icelines-fetch fletch --lib
17 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
