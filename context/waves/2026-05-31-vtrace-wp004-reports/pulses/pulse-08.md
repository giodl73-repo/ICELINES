# WP-004 Pulse 08 - Full-Lockout Season Skip and Closeout

## Scope

Close the remaining ambiguity in the `VAL-002` lockout-skip claim by adding
focused evidence that the fully cancelled 2004-05 season is not offered as a
fetchable historical season while adjacent seasons remain available.

## Change

- Added `l0_available_seasons_skip_full_lockout_and_keep_neighbors` in
  `icelines-cli/src/commands/data.rs`.
- The test asserts `20042005` is excluded from `AVAILABLE_SEASONS` and that
  adjacent `20032004` and `20052006` seasons remain present.
- Closed WP-004 with accepted residual risk for broader cross-surface
  report/export matrix coverage, broader active-streak parity, and
  ambiguous-name disambiguation breadth to be rehearsed under WP-008.

## Evidence

```powershell
cargo test -p icelines-cli l0_available_seasons_skip_full_lockout_and_keep_neighbors --quiet
```

Result: passed.

## Review

Decision: `closed_with_risk`.

The selected historical-perspective fixture set now covers public-copy
disclosure, active-streak status labeling, completeness/skeleton disclosure,
GP thresholds, duplicate/Unicode names, trade continuity, explicit shortened
lockout and October rollover season windows, and the fully skipped 2004-05
lockout season. Full report/export matrix coverage remains a WP-008 residual
risk rather than a blocker for WP-004 package closure.
