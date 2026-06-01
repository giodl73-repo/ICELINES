# WP-005 Pulse 05 - Data/Fetch Command Transcript Boundaries

## Scope

Selected data/fetch/snapshot command transcript boundary for offline and
missing-source behavior.

## Change

- `data install --season 20042005` now validates the unavailable full-lockout
  season before creating local season directories, so the no-op remains a true
  isolated-home no-op.
- `fetch boxscore --for-favorites --dry-run` and
  `fetch play-by-play --dry-run` now refuse under `--no-live` before constructing
  the production NHL API client or live schedule path.
- Added L2 subprocess evidence for lockout `data install`, `data-status`,
  `fetch sync --dry-run`, `snapshot verify`, and the no-live fetch refusal
  boundaries.

## Evidence

```powershell
cargo fmt --check
cargo test -p icelines-cli --test system_tests l2_wp005 -- --nocapture
cargo clippy -p icelines-cli --test system_tests --bin icelines --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

The selected L2 evidence covers explicit offline/missing-source command copy and
cache-write boundaries for data install/status, fetch sync, snapshot verify, and
live-only fetch command surfaces. It closes the selected command transcript gap
while leaving cache/refresh, schema drift, integrity mismatch, newer schema,
missing-source, abbreviation drift, and partial-fetch
resume/flag fixture breadth pending.

## Status

`passed_with_risk`.
