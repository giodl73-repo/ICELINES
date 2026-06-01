# Pulse 03: Poach imported-availability read-only boundary

## Scope

Close the selected Web poach GET path that reads existing local SQLite state for
imported fantasy availability, watch rules, and watchlist context.

## Change

- Promoted the immutable SQLite read-only connection helper so non-fantasy Web
  reads of the shared `icelines.db` can reuse the same no-sidecar boundary as
  FantasyDb reads.
- Updated Web poach imported-availability and watch read helpers to open existing
  `icelines.db` through the read-only helper instead of writable
  `rusqlite::Connection::open`.
- Added route evidence that `/api/v1/poach?availability=imported-available`
  renders without creating `icelines.db-wal` or `icelines.db-shm`.
- Serialized the poach route tests that depend on process-global HOME state so
  they do not race the temp-home sidecar preservation check.

## Evidence

```powershell
cargo fmt --check
cargo test -p icelines-web --test l1_router poach -- --nocapture
cargo test -p icelines-web --test l1_router fantasy -- --nocapture
cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git --no-pager diff --check
```

## Result

`closed_with_risk`

Selected poach imported-availability GET reads no longer create SQLite sidecar
state when an existing local FantasyDb is present. The pulse also reinforces the
dashboard poach/read-path evidence because dashboard poach summaries share the
same read-only imported-availability helper.

## Accepted risks

- The pulse covers selected Web poach GET/imported-availability reads and watch
  read helpers only; POST-backed watch mutations remain intentionally writable.
- Active-writer/concurrent-CLI database semantics remain pending and should not
  be inferred from immutable closed-database route evidence.
- Full `VAL-007` remains open for the final command/API transcript, but the
  poach, roster gaps, simulation, import-deferral, invalid-drop, and selected
  local-state preservation slices now have route/parser evidence.
