# WP-004 Pulse 04 - GP Threshold Export Evidence

## Scope

Add focused report/export evidence for the first-class leaders `--gp-min`
threshold so low-games-played rows are excluded before public table rendering
and the threshold is disclosed in both front matter and body metadata.

## Change

- `icelines-cli/src/commands/export.rs` now has an explicit fixture test for the
  `--gp-min` leaders export boundary.
- The fixture proves a player below the threshold is excluded even when that row
  would otherwise sort above the retained row.
- The test asserts `gp_min`, result counts, active filter body copy, and table
  ordering/reporting metadata.

## Evidence

```powershell
cargo test -p icelines-cli l0_export_leaders_gp_min_filters_rows_and_reports_threshold --quiet
```

## Result

`passed_with_risk`: selected Markdown leaders exports now have focused evidence
that GP thresholds filter rows before rendering and are disclosed before the
table. Lockout, October rollover, ambiguous/Unicode/duplicate names, trade
continuity, and full report/export matrix coverage remain open for later WP-004
pulses.

