# Pulse 04 - Shot and Attempt Streak Leaderboards

## Goal

Extend player/team streak leaderboards with shot and attempt streaks derived
from per-game play-by-play event aggregation, not season totals.

## Governing roles

- **scout**: streak language should describe finishing pressure and shot volume,
  not predictions or betting signals.
- **edge**: zero-attempt games must break streaks; missing play-by-play must
  remain distinct from loaded games with zero matching player events.
- **bench**: add L0/L1 tests for current and longest shot/attempt streaks,
  including zero-event breaks and traded/team-filtered rows.
- **wire**: read only existing `DataKind::PlayByPlay` cache entries; do not add
  live fetches or GET-backed warming.

## Owned scope

1. Add per-player per-game shot/attempt line inputs derived from
   `ScoringEventInput`.
2. Extend `PlayerStreaksView` / `TeamPlayerStreaksView` with shot-on-goal and
   attempt streak metrics.
3. Wire the fetch provider to aggregate play-by-play events into streak inputs
   where cached data exists.
4. Add tests proving zero-attempt games break shot/attempt streaks and missing
   sources remain explicit.

## Non-goals

- No new scoring trend windows.
- No web/TUI template redesign beyond fields required by existing ViewModels.
- No season-total inference for missing games.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-fetch --quiet`
- [ ] `cargo clippy -p icelines-core -- -D warnings`
- [ ] `cargo clippy -p icelines-fetch -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-measure-the-finish design\waves\PHASES.md --errors-only`
