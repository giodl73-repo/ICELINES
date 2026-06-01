# WP-005 Pulse 02 - Offline Query Smoke

## Scope

Selected offline/query reliability boundary: global `--no-live` must prevent
live-only CLI views from calling the NHL API, while bundled-backed query paths
continue to answer from packaged source data without creating local cache state.

## Change

- Added no-live guards to `icelines-cli/src/commands/tonight.rs` for `tonight`
  and `schedule`.
- Strengthened system tests so `--no-live schedule` exits with a disabled-live
  source-state message and no cache writes.
- Added isolated-home smoke evidence that `--no-live query leaders` returns a
  JSON envelope from bundled data without creating `~/.icelines/data` or
  `~/.icelines/cache`.
- Recorded `CHG-058` and `EVID-WP005-OFFLINE-SMOKE-L2`.

## Evidence

```powershell
cargo test -p icelines-cli --test system_tests l2_cmd_no_live -- --nocapture
cargo test -p icelines-cli commands::tonight --bin icelines -- --nocapture
cargo fmt --check
cargo clippy -p icelines-cli --test system_tests --bin icelines --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

The system-test slice covers both sides of the offline smoke claim. The live-only
schedule path now surfaces explicit disabled-live source-state copy and returns
without cache creation. The bundled-backed leaders query still succeeds under
`--no-live`, emits a JSON envelope, and does not create local data or live API
cache directories in an isolated home.

## Status

`passed_with_risk`.

Remaining WP-005 work includes fetch failure mocks, data command transcripts,
partial-fetch resume/flag evidence, and locked shift-level refusal.
