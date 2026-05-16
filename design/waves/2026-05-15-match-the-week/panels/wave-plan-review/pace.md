# PACE Review - Match the Week

## Findings

- Weekly matchup scoring should be framed as descriptive points earned inside an
  ISO week, not a projection of who will win.
- The daily-delta contract already defines one-game scoring from finalized
  lines, so weekly totals can be a transparent sum of daily team totals.
- Winner rules need to be named: higher weekly points wins, equal points tie,
  bye rows are informational and not wins.

## Required Pulse Constraints

- State `week_start` and `week_end` in every weekly ViewModel output.
- Preserve deterministic ordering for matchup rows and team rows.
- Do not introduce opponent-strength adjustments, schedule predictions, or
  official-Yahoo claims in this wave.
