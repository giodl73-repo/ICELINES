# GLASS Review - Import the Rosters

## Findings

- Import output should lead with a summary: league, teams created/updated,
  rostered players imported, skipped rows, and error count.
- Users need a recovery path for each failed row, not a raw parser dump.
- TUI/web dashboard can start with command handoffs because local file selection
  is better handled by the CLI until a POST-backed upload exists.

## Required Pulse Constraints

- Keep CLI text readable at 80 columns and put row diagnostics after the summary.
- JSON must preserve structured diagnostics so future UI can render them without
  re-parsing text.
- Empty or invalid import files should recommend the expected headers and dry-run
  command.
