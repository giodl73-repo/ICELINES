# PACE Review - Score the Day

## Findings

- Daily delta must be described as points earned on a date from finalized game
  lines, not a projection and not a season pace adjustment.
- The scoring formula must name every included category and reuse `Scheme`
  weights. Daily scoring should not inherit the season-score min-GP projection
  threshold.
- Tie-breaking must be stable and documented: daily points descending, then team
  or player display name.

## Required Pulse Constraints

- Pulse 02 needs manually calculated L0 expected values for skater and goalie
  daily rows.
- Any rounding policy shown by CLI/web/TUI must be consistent with the ViewModel.
