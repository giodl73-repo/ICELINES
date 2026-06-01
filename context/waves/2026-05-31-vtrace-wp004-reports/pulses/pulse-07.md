# WP-004 Pulse 07 - Season Window Export Evidence

## Scope

Add focused report/export evidence that Markdown leaders exports honor explicit
historical season windows for a shortened lockout season and an October rollover
season.

## Change

- `icelines-cli/src/commands/export.rs` now writes Markdown front-matter
  `season` from the active `ViewContext` when one is available, instead of
  always using the current season string.
- A focused fixture covers the 2012-13 shortened lockout window and a 2025-26
  October-opening window.
- The fixture proves visible context, front matter, and rendered rows remain tied
  to the explicit season window and do not fall back to the default/current
  season.

## Evidence

```powershell
cargo test -p icelines-cli l0_export_leaders_honors_lockout_and_october_rollover_windows --quiet
```

## Result

`passed_with_risk`: selected Markdown leaders exports now have focused evidence
for explicit season-window handling across lockout and October rollover
boundaries. Full report/export matrix coverage remains open for later WP-004
pulses or WP-008 rehearsal.

