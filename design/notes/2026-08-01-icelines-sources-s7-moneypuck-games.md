# IceLines Sources S7 — MoneyPuck Game Parser Splits

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete provider-parser slices; feature derivation remains fetch-owned

## Delivered

- Moved team-game and goalie-game CSV row contracts, required-column checks,
  normalization, duplicate detection, and parse errors to `icelines-sources`.
- Preserved public fetch module parser/type/error paths through compatibility
  re-exports.
- Kept trailing xG, opponent adjustment, special-teams rates, goalie form,
  workload readiness, fingerprints, and endpoint naming in `icelines-fetch` as
  feature/acquisition composition.
- Added direct source tests for normalization, duplicate rejection, and mixed
  goalie identity while retaining all existing strict-cutoff feature tests.

## Verification

```text
cargo test -p icelines-sources moneypuck_team_game
2 passed; 0 failed

cargo test -p icelines-fetch moneypuck_team_game --lib
5 passed; 0 failed

cargo test -p icelines-sources moneypuck_goalie_game
2 passed; 0 failed

cargo test -p icelines-fetch moneypuck_goalie_game --lib
existing parser/form tests pass
```
