# IceLines Sources S7 — NHL Landing Contract Parser Split

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete parser slice; the mixed `nhl_api` module remains in progress

## Delivered

- Moved official NHL player-landing contract-hint interpretation to
  `icelines_sources::nhl::player_landing`.
- Preserved `icelines_fetch::nhl_api::parse_player_landing_contract` as a
  compatibility re-export and kept endpoint acquisition, retries, batching,
  and rate limiting in `NhlApiClient`.
- Added source tests for the current public payload's authoritative absence of
  contract fields and the already-supported future `currentContract` shape.

Landing contract hints remain nullable observations. They do not become
contract-control or legal-control authority without the terminal reviewed
ledger required by `contract_control_ledger.v1`.

## Verification

```text
cargo test -p icelines-sources player_landing
4 passed across unit and integration tests

cargo test -p icelines-fetch nhl_api --lib
14 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
