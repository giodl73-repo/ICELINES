# Pulse 03 - Shot-Event Cache Loader

## Goal

Make the existing manifest-backed play-by-play cache explicitly serve Rocket
Richard scoring-event data, and add report-provider source states so later web
and API reports can tell the difference between "source missing" and "source
loaded but no scoring events matched."

## Governing roles

- **tape**: keep official NHL play-by-play as the only row source for shot
  events; preserve game and team identity from the payload/manifest.
- **edge**: loaded play-by-play with zero scoring events is not a missing
  source; missing source state must only mean the raw play-by-play body is not
  installed/readable.
- **wire**: reuse `DataKind::PlayByPlay`; scoring/shot cache labels are aliases,
  not a new cache shard.
- **bench**: add tests that would fail if scoring aliases created duplicate
  cache requests or if provider views marked an installed zero-event source as
  unavailable.

## Owned scope

1. Add scoring/shot-event aliases to game-cache artifact parsing while mapping
   them to `PlayByPlay`.
2. Add provider helpers for game scoring reports with explicit source-state
   semantics.
3. Update fetch/admin/help text so users see that play-by-play powers scoring
   events, not only records.
4. Update wave status/evidence.

## Non-goals

- No scoring web/API/TUI route yet.
- No shot maps, danger buckets, xG, or rink-orientation normalization.
- No new manifest shard or duplicate cache path.
- No network tests; all provider checks use fixtures/tempdirs.

## Implementation result

- Added scoring/shot-event game-cache aliases (`scoring-events`, `shot-events`,
  `shots`) that deduplicate to the existing `PlayByPlay` artifact and
  `DataKind::PlayByPlay` manifest shard.
- Added source-state-aware scoring report constructors and
  `load_game_scoring_report`, so missing play-by-play is unavailable while an
  installed zero-shot-event payload is complete.
- Updated admin/CLI/COMMANDS copy so play-by-play is discoverable as the source
  for Rocket scoring events.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo clippy -p icelines-core -p icelines-fetch -p icelines-web -- -D warnings`
- [x] `cargo clippy -p icelines-cli -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-aim-the-rocket design\waves\PHASES.md COMMANDS.md --errors-only`
