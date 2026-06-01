# WP-005 Pulse 11 - Abbreviation Drift Fixtures

## Scope

Selected ESPN/NHL team abbreviation drift boundary.

## Change

- Recorded existing L0 mapper evidence for ESPN shorthand, relocation, and
  unknown team abbreviations.
- Recorded existing L1 bundled-transaction evidence that supported historical
  transaction rows carry canonical team abbreviations, while legacy `ARI` and
  `ATL` rows remain explicit historical exceptions.

## Evidence

```powershell
cargo test -p icelines-fetch teams::tests::l0_espn_to_nhl --lib -- --nocapture
cargo test -p icelines-fetch transactions::convert::tests::l0_convert --lib -- --nocapture
cargo test -p icelines-fetch --test transactions_storage l1_bundled_team_abbrevs_all_canonical -- --nocapture
cargo fmt --check
cargo clippy -p icelines-fetch --lib --test transactions_storage --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

This closes the selected abbreviation-drift boundary. It does not close broader
missing-source or partial-fetch resume/flag evidence.

## Status

`passed_with_risk`.
