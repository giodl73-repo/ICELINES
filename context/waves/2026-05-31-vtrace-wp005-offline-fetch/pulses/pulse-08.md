# WP-005 Pulse 08 - MoneyPuck CSV Drift Fixtures

## Scope

Selected MoneyPuck CSV required-column and malformed-row drift boundary.

## Change

- Added a checked MoneyPuck CSV parser that verifies required headers before row
  deserialization.
- Changed `fetch moneypuck` to use the checked parser so source schema or row
  drift fails loudly before snapshot creation.
- Preserved the legacy in-process parser wrapper for existing callers while
  routing the user-facing fetch path through explicit errors.

## Evidence

```powershell
cargo test -p icelines-fetch moneypuck::tests::l0_parse_csv_checked --lib -- --nocapture
cargo fmt --check
cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings
cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

This closes the selected MoneyPuck CSV required-column and malformed-row drift
boundary. It does not close cache/refresh, upstream payload schema drift,
broader missing-source, abbreviation drift, or partial-fetch resume/flag
evidence.

## Status

`passed_with_risk`.
