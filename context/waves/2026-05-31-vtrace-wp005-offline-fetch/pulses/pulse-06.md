# WP-005 Pulse 06 - Snapshot Integrity/Missing-File Fixtures

## Scope

Selected snapshot integrity and missing-file fixture boundary.

## Change

- Added L0 evidence that sealed snapshot reads fail with
  `SnapshotError::IntegrityViolation` when tracked bytes differ from the
  snapshot integrity hash.
- Added L0 evidence that `snapshot verify` reports a tracked file deleted after
  sealing as `MISSING` instead of silently accepting an incomplete snapshot.
- Refactored the local `fletch` source-definition helper to retire the existing
  `too_many_arguments` lint blocker and keep the affected fetch library clippy
  gate clean.

## Evidence

```powershell
cargo fmt --check
cargo test -p icelines-fetch snapshot::tests::l0_snapshot --lib -- --nocapture
cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

This closes the selected query-time integrity mismatch and verify-time
missing-file snapshot boundary. It does not close cache/refresh, schema drift,
newer schema, abbreviation drift, broader missing-source, or
partial-fetch resume/flag evidence.

## Status

`passed_with_risk`.
