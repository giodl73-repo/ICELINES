# Phase Canadiens Pulse 04 - Signals authority and shift policy

## Result

Passed. Signals authority copy and the historical shift-data policy lock were
carried through the selected user surfaces.

## Evidence

- `design/plans/2026-06-22-phaseCanadiensSignals-source-authority.md`
- `design/plans/2026-06-22-phaseCanadiensSignals-roster-authority.md`
- `design/plans/2026-06-22-phaseCanadiensSignals-markdown-authority.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-policy-lock.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-tui-policy-lock.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-mates-fallback-policy.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-tui-mates-handoff.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-tui-deployment-alias.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-command-doc-sources.md`
- `design/plans/2026-06-22-phaseCanadiensShifts-query-doc-policy.md`
- commits `4a75793` through `70d9875`

## Closeout

Signals remain direct inspection surfaces with shared authority copy. They do
not enter analytics cache, `StatId`, filters, or public leaderboards. Shift
support remains locked off until source, bundle, fetch, fixture, and join policy
evidence exists.
