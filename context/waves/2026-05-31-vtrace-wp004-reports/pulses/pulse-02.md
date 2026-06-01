# WP-004 Pulse 02 - Active Streak Status Label

## Scope

Add an explicit active-streak status label so current streak outputs distinguish
ongoing streaks from inactive streaks without requiring readers or renderers to
infer meaning from the numeric `current` value alone.

## Change

- `icelines-core/src/view_model/streaks.rs` now adds `current_status` to player
  and team-player streak rows.
- The shared ViewModel emits `ongoing` when `current > 0` and `inactive` when the
  loaded most-recent game broke the streak.
- The CLI streaks table/CSV surface and TUI player-streaks table render the
  status label as an additive column.
- Focused ViewModel tests assert active and inactive labels for goals, assists,
  points, shots-on-goal, and shot-attempt streaks.

## Evidence

```powershell
cargo test -p icelines-core view_model::streaks::tests:: --quiet
cargo test -p icelines-cli streaks --quiet
cargo fmt --check
cargo clippy -p icelines-core --lib --tests -- -D warnings
cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings
```

## Result

`passed_with_risk`: selected player and team-player streak ViewModels now expose
and render active-streak status labels. Lockout, October rollover,
ambiguous/Unicode/duplicate names, trade continuity, GP thresholds, and
skeleton/completeness disclosure remain open for later WP-004 pulses.
