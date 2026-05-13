---
wave: backcheck-the-phases
pulse: 03
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - keel
  - scout
  - wire
  - tape
---

# Pulse 03 - Presidents Trophy Team Season Report Parity

## Mission

Backfill the known carry-forward from the team season-performance phase:
markdown/report export parity for `TeamSeasonView`. Team season is distinct
from roster/depth and should be exportable as a season-to-date assessment.

## Deliverables

- Inventory current CLI/TUI/web/dashboard team season surfaces.
- Add or amend report/markdown output for team season if missing.
- Ensure strength of schedule, home/away splits, playoff distance, quality
  wins/losses, form, and remaining pressure are represented where the ViewModel
  already exposes them.
- Update `design/specs/surface-parity.md`.

## Gates

- [ ] Focused core `TeamSeasonView` tests pass.
- [ ] Focused CLI/report tests pass.
- [ ] Focused web dashboard summary tests pass.
- [ ] Surface parity row names report/export status truthfully.

## Stop Conditions

- Stop if the ViewModel lacks required fields; create a follow-up ViewModel
  pulse rather than formatting synthetic surface-local metrics.
