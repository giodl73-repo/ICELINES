# IceLines Sources S7 — MoneyPuck Player Parser Extraction

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; workspace/repository CI remains the merge gate

## Delivered

- Moved the player-season MoneyPuck CSV DTO, required-header validation,
  parsing, situation fan-in, and metric normalization to
  `icelines_sources::moneypuck`.
- Preserved the existing `MoneyPuckStats` serialized shape and legacy neutral
  defaults for zero denominators.
- Preserved `icelines_fetch::moneypuck::{parse_csv, parse_csv_checked, index,
  MoneyPuckCsvError, MoneyPuckRow, MoneyPuckStats}` through compatibility
  re-exports.
- Kept MoneyPuck URL construction and HTTP/FLETCH acquisition in
  `icelines-fetch`.
- Re-ran the committed schema fixture and every legacy player parser test
  through the compatibility facade. Team-game and goalie-game acquisition and
  feature composition remain separate future slices.

## Verification

```text
cargo test -p icelines-sources moneypuck
1 passed; 0 failed

cargo test -p icelines-fetch moneypuck --lib
19 passed; 0 failed
```
