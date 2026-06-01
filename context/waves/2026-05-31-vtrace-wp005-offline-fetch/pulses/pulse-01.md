# WP-005 Pulse 01 - Snapshot Seal Refusal

## Scope

Selected offline/fetch reliability boundary: named snapshot reads must not trust
draft or partial snapshot bytes. An unsealed snapshot is not reliable source
state even when the target file exists.

## Change

- Added `l0_snapshot_read_named_refuses_unsealed_snapshot` in
  `icelines-fetch/src/snapshot.rs`.
- Recorded `CHG-057` and `EVID-WP005-SNAPSHOT-SEAL-L0`.
- Moved `WP-005` from `proposed` to `in_progress`.

## Evidence

```powershell
cargo test -p icelines-fetch l0_snapshot_read_named_refuses_unsealed_snapshot --quiet
cargo fmt --check
cargo clippy -p icelines-fetch --lib --tests -- -D warnings -A clippy::too_many_arguments
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

The unallowed fetch package clippy command remains blocked by the existing
`clippy::too_many_arguments` warning in `icelines-fetch/src/fletch.rs`, outside
this pulse's touched snapshot boundary.

## Review

The test creates a draft roster snapshot, writes a file into it, and calls
`SnapshotStore::read` by name before sealing. The expected result is
`SnapshotError::NotSealed { name: "draft" }`, proving the read path refuses
unfinished snapshot bytes before deserialization.

## Status

`passed_with_risk`.

Remaining WP-005 work includes offline smoke, fetch failure mocks, data command
transcripts, partial-fetch resume/flag evidence, and locked shift-level refusal.
