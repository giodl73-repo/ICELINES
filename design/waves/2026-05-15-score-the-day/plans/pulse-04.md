---
wave: score-the-day
pulse: 04
date: 2026-05-15
status: complete
governing_roles:
  - bench
  - wire
  - glass
  - forge
---

# Pulse 04 - CLI, Web, and TUI Read Surfaces

## Goal

Expose fantasy daily delta scoring through thin read-only surfaces.

## Owned Scope

- Add a CLI read command, likely `icelines fantasy daily --date YYYY-MM-DD`
  with JSON output.
- Add web/API read routes only after they project the shared ViewModel.
- Add TUI/workbench affordance or command handoff without duplicating scoring.
- Add L2 or route tests for output shape and missing-cache behavior.

## Non-goals

- No dashboard GET-backed mutation.
- No fantasy league write-management expansion.
- No live network fetch from the daily command.

## Outcome

- Added `icelines fantasy daily --date YYYY-MM-DD [--league] [--json]`.
- Added `GET /api/v1/fantasy/daily?date=YYYY-MM-DD`.
- Added TUI and web-dashboard command handoffs for `fantasy daily date=...`.
- Kept all scoring/data projection in the shared
  `FantasyDailyDeltaView`/`build_fantasy_daily_delta_view` path.

## Gates

- [x] focused CLI tests for `fantasy daily`
- [x] focused web tests if web/API routes are added
- [x] `cargo fmt --check`
