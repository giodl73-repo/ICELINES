# WP-004 Pulse 06 - Trade Continuity Export Evidence

## Scope

Add focused report/export evidence that a traded player remains one aggregate
leaders row and uses the last-stint team display for the selected season window.

## Change

- `icelines-cli/src/commands/export.rs` now has a fixture test with one skater
  split across two team stints in the same season.
- The fixture proves the export renders a single aggregate row, not one row per
  stint.
- The fixture proves the report uses last-stint team display while preserving
  aggregate GP/goals/assists/points totals.

## Evidence

```powershell
cargo test -p icelines-cli l0_export_leaders_traded_player_renders_once_with_last_stint_team --quiet
```

## Result

`passed_with_risk`: selected Markdown leaders exports now have focused evidence
for traded-player continuity. Lockout, October rollover, and full report/export
matrix coverage remain open for later WP-004 pulses.

