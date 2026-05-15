# Pulse 03 - Player Scoring Trend Rows

## Goal

Add ViewModel-owned player scoring trend rows built from
`PlayerScoringProfileView.events`, consuming the IceLines inside-shot proxy from
Pulse 02 before any web/API template changes.

## Governing roles

- **scout**: trend labels must describe hockey context: recent volume,
  conversion, inside looks, and location coverage. Do not call rows
  projections, bets, or expected goals.
- **edge**: missing coordinates and missing player IDs must stay explicit.
  Loaded play-by-play with zero matching events is different from missing
  play-by-play.
- **bench**: each window and percentage rule needs L0 known-value tests,
  including zero-shot and unknown-location cases.
- **wire**: trend rows are pure ViewModel projections over already parsed
  `ScoringEventInput`; do not read manifests, mutate caches, or fetch data.

## Owned scope

1. Add player scoring trend row structs to `icelines-core`.
2. Populate last-3, last-5, last-10, and season-loaded trend rows from
   `PlayerScoringProfileView.events`.
3. Count attempts, unblocked attempts, shots on goal, goals, inside-shot proxy
   buckets, and unknown-location events per row.
4. Include conversion as nullable shot percentage (`goals / shots_on_goal`) and
   source-state fields for games/events loaded.
5. Add L0 tests for window selection, player matching, zero-shot conversion,
   and unknown-location counts.

## Non-goals

- No web/API template or route changes in this pulse.
- No shot-streak aggregation yet.
- No third-party xG, danger-bucket parity, or proprietary model claims.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo clippy -p icelines-core -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-measure-the-finish design\waves\PHASES.md --errors-only`
