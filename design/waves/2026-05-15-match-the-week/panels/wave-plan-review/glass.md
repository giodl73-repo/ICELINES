# GLASS Review - Match the Week

## Findings

- The first read surface should make the matchup score, opponent, week range,
  and completeness obvious before showing player-level detail.
- TUI can start as a command handoff; a full screen is not needed until the
  ViewModel proves stable.

## Required Pulse Constraints

- Keep CLI text readable at 80 columns: matchup rows first, warnings after.
- JSON must carry enough structure for web/TUI surfaces to render without
  recomputing outcomes.
- Empty/setup state should tell the user how to add schedule rows.
