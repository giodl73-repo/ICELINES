# IceLines Sources S7 — AHL Transaction Transport/Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; workspace/repository CI remains the merge gate

## Delivered

- Moved AHL transaction DTOs, HockeyTech page parsing, provider-local identity
  preservation, ADD/DEL classification, page reconciliation, and snapshot
  validation to `icelines_sources::ahl::transactions`.
- Kept season/team discovery, pagination requests, concurrency, FLETCH capture,
  and snapshot assembly in `icelines-fetch`.
- Preserved the public `icelines_fetch::ahl_transactions` type and parser paths
  through compatibility re-exports.
- Added an explicit source-error to `AhlFeedError` conversion so existing
  transaction-state consumers retain their `?`-based fetch boundary.
- Re-ran both snapshot parser/validation tests and the downstream state-ledger
  suite. Transaction absence still creates no assignment, release, contract,
  waiver, or control fact.

## Verification

```text
cargo test -p icelines-sources ahl::transactions
1 passed; 0 failed

cargo test -p icelines-fetch ahl_transactions --lib
3 passed; 0 failed

cargo test -p icelines-fetch ahl_transaction_state --lib
6 passed; 0 failed
```
