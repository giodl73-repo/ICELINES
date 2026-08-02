# IceLines Sources S7 — CapWages Contract Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-parser/normalization slice

## Delivered

- Moved CapWages response DTOs, name normalization, season-label conversion,
  and contract projection to `icelines_sources::capwages`.
- Kept API-key handling, HTTP paging, bounded detail concurrency, worker
  errors, and team cap summaries in `icelines-fetch`.
- Preserved nullable contract values and provenance; unmatched or absent
  seasons still remain absent rather than becoming zero-valued contracts.
- Added direct source tests and retained the legacy fetch conversion and cap
  summary tests.

## Verification

```text
cargo test -p icelines-sources capwages
2 passed; 0 failed

cargo test -p icelines-fetch capwages --lib
3 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
