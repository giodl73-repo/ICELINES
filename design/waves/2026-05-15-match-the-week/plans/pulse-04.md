---
wave: match-the-week
pulse: 04
date: 2026-05-15
status: complete
governing_roles:
  - glass
  - wire
  - bench
  - forge
---

# Pulse 04 - CLI, Web, and TUI Matchup Surfaces

## Goal

Expose weekly fantasy matchups through thin, discoverable surfaces after the
shared ViewModel and builder exist.

## Owned Scope

- Add CLI setup/read commands for local matchup schedules and weekly matchup
  results.
- Add read-only web JSON route for the weekly ViewModel.
- Add TUI and web-dashboard command handoffs to canonical CLI/API surfaces.
- Keep web/dashboard GET routes read-only; any future web mutation must be
  POST-backed.
- Add focused CLI/TUI/web tests that assert surfaces project the shared
  ViewModel and preserve missing schedule/cache warnings.

## Non-goals

- No full TUI matchup screen.
- No web GET mutation.
- No Yahoo schedule import.
- No new scoring math in surface code.

## Gates

- [x] focused CLI tests for matchup schedule/read commands
- [x] focused web tests if JSON route is added
- [x] focused TUI/dashboard command parser tests
- [x] `cargo fmt --check`

## Result

Added thin weekly matchup surfaces over the shared fetch/core path: CLI
`fantasy matchup --date`, CLI `fantasy matchup-set --week --home [--away]`,
read-only JSON `/api/v1/fantasy/matchup?date=...`, and TUI/web-dashboard
command handoffs. Focused tests cover clap parsing, missing-schedule JSON,
web route behavior, and TUI/dashboard handoffs.
