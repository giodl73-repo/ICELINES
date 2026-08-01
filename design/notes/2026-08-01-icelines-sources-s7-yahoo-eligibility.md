# IceLines Sources S7 — Yahoo Eligibility and Non-Prospect Reuse

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; workspace/repository CI remains the merge gate

## Delivered

- Added `icelines_sources::yahoo_eligibility::parse_yahoo_eligibility_csv`
  over caller-supplied bytes and moved `YahooEligibility` to the source crate.
- Preserved UTF-8 BOM handling, lossy invalid-byte replacement, flexible Yahoo
  rows, required-header validation, Unicode names, optional images, and the
  legacy rule that rows without team or eligibility are ignored.
- Preserved `icelines_fetch::csv_loader::{load_csv_eligibility,
  YahooEligibility}`. Fetch still owns file reading and maps source parser
  findings into the existing `FetchError::CsvParse` surface.
- Proved that the same deterministic source boundary supports fantasy position
  eligibility, satisfying the architecture's first non-prospect consumer
  requirement. Yahoo membership and eligibility still provide no hockey
  identity, performance, contract, or control authority.

## Verification

```text
cargo test -p icelines-sources yahoo_eligibility
2 passed; 0 failed

cargo test -p icelines-fetch csv_loader --lib
5 passed; 0 failed

cargo test -p icelines-fetch --test integration_pipeline
10 passed; 0 failed
```
