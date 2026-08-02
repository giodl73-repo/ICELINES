# IceLines Sources S7 — Contract CSV Parser Extraction

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; workspace/repository CI remains the merge gate

## Delivered

- Added `icelines_sources::contracts_csv::parse_contracts_csv`, a deterministic
  parser over caller-supplied bytes.
- Preserved selected-season filtering, stable player-ID ordering, duplicate
  rejection, consecutive-season validation, required monetary values, audit
  labels, absolute HTTP(S) provenance, RFC 3339 check times, and source row
  numbers.
- Kept file loading, the selected-bios identity join, and the existing
  `PlayerContract` projection in `icelines-fetch`.
- Preserved `icelines_fetch::contracts_csv::load_contracts_csv` and its public
  error variants for callers.
- Added an exact serialized `PlayerContract` compatibility assertion. Parser
  extraction therefore does not change the CLI-facing contract overlay shape.
- Strengthened the source-crate architecture test to scan every Rust source
  file for transport and persistence calls, in addition to checking forbidden
  direct dependencies.

No contract value gains legal-control authority merely by passing this parser.
The provider-neutral `contract_control_ledger.v1` review and terminal-coverage
gate remains the authority boundary used by the prospect census.

## Dependency boundary

```text
icelines-sources: CSV bytes -> validated ContractCsvRecord rows
icelines-fetch:   file path + selected bios -> existing PlayerContract rows
```

`icelines-sources` adds only deterministic `csv` and `url` parsing libraries;
it still has no HTTP client, async runtime, filesystem, database, cache,
snapshot, CLI, or renderer ownership.

## Verification

```text
cargo test -p icelines-sources
passed

cargo test -p icelines-fetch contracts_csv --lib
2 passed; 0 failed

cargo clippy -p icelines-sources --all-targets -- -D warnings
passed
```
