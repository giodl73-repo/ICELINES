# BENCH Review - Shape the Rosters

## Findings

- Roster-shape validation needs direct tests for legal, underfilled, overfilled,
  unknown-player, duplicate, and goalie/skater mismatch states.
- Import dry-run must remain no-mutation even when validation warnings are
  generated.
- Surface tests should assert recovery text is command-copyable, not just that a
  table renders.

- bench
