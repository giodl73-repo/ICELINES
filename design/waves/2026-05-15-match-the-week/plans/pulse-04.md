---
wave: match-the-week
pulse: 04
date: 2026-05-15
status: planned
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

- [ ] focused CLI tests for matchup schedule/read commands
- [ ] focused web tests if JSON route is added
- [ ] focused TUI/dashboard command parser tests
- [ ] `cargo fmt --check`
