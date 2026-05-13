# R4 Review - tape

## Findings

### F-01 - WARN: Team-season export must preserve source warnings
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-03.md`
Finding: Pulse 03 asks reports to include standings, strength of schedule, playoff distance, form, and quality ledger fields where `TeamSeasonView` exposes them. Exports must also carry the view's source warnings when standings or SOS are partial.
Consequence: A markdown report could look complete while hiding that standings or schedule-strength data was unavailable.
Fix: Add a report/export assertion that source warnings or source-state labels survive from `TeamSeasonView` into markdown/report output.

### F-02 - WARN: Career/docs parity must handle missing local career history
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-08.md`
Finding: Career cohort data is intentionally not bundled and depends on the local career-history store. Pulse 08's gates cover career tests but do not explicitly require the missing-store path.
Consequence: A TUI/docs affordance could imply the career board is fully available on cold install when `~/.icelines/career_history.json` has not been fetched.
Fix: Add a missing-career-store test or docs assertion that renders an explicit fetch instruction instead of empty success.
