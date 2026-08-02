# IceLines Sources S7 — Closing Mixed-Module Boundary Audit

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete

## Finding

After the 19 recorded parser/responsibility migrations, the three apparent
remaining `split_transport_parser` rows did not contain an unowned raw-provider
parser:

- `management_behavior_source` requests already-normalized source DTOs through
  `NhlApiClient`, joins them, derives behavior count facts, calibrates profiles,
  and builds rankings. That is acquisition orchestration plus feature-domain
  composition; its later domain extraction is independent of
  `icelines-sources`.
- `organization_window_history` consumes normalized standings, skater bio, and
  skater stat rows to build sealed Window calibration and holdout artifacts. It
  performs domain normalization and composition but no HTTP acquisition or raw
  provider parsing.
- `prospect_source_audit` expands requests, acquires and stores exact bytes,
  invokes `icelines-sources` adapters, reconciles fragments, and seals source
  packages. Compatibility inspection of reviewed artifacts is orchestration;
  adapter parsing and validation remain source-owned. The module belongs in
  fetch because it owns acquisition and package persistence.

The inventory now classifies those actual responsibilities instead of leaving
false parser-migration work. No feature scoring, ranking, Window calibration,
filesystem store, FLETCH call, or package activation was moved into
`icelines-sources` to make the inventory look complete.

## S7 boundary result

- Raw provider DTOs and deterministic parsers covered by this wave are owned by
  `icelines-sources`.
- Compatibility facades remain in `icelines-fetch` where existing consumers
  require them.
- Transport, caching, persistence, explicit review, and feature-domain
  composition remain outside the source crate.
- S6's real authorized identity, contract-control, and camp ledgers remain an
  authority-input gate, not an architectural parser blocker.

## Verification

```text
cargo test -p icelines-fetch --test source_module_inventory
3 passed; 0 failed

cargo test -p icelines-sources --test architecture_dependencies
2 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed
```
