# WP-005 Pulse 10 - Upstream Payload Schema Drift Fixture

## Scope

Selected player landing upstream payload schema-drift boundary.

## Change

- Recorded existing L1 httpmock evidence that a malformed player landing payload
  does not become trusted career-history source data.
- The mocked upstream response returns a `200` JSON body without the required
  `seasonTotals` structure, and `NhlApiClient::fetch_player_career_history`
  surfaces a schema-related error.

## Evidence

```powershell
cargo test -p icelines-fetch --test career_landing_mock l1_fetch_player_career_history_surfaces_schema_error -- --nocapture
cargo fmt --check
cargo clippy -p icelines-fetch --test career_landing_mock --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

This closes the selected player landing upstream schema-drift boundary. It does
not close broader missing-source, abbreviation drift, or partial-fetch resume/flag
evidence.

## Status

`passed_with_risk`.
