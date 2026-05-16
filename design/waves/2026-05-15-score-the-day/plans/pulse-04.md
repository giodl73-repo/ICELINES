---
wave: score-the-day
pulse: 04
date: 2026-05-15
status: planned
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

## Gates

- [ ] focused CLI tests for `fantasy daily`
- [ ] focused web tests if web/API routes are added
- [ ] `cargo fmt --check`
