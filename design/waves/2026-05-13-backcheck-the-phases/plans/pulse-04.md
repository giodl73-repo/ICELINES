---
wave: backcheck-the-phases
pulse: 04
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - crest
  - broadcast
  - glass
  - tape
---

# Pulse 04 - Visual and CREST Regression Captures

## Mission

Make the Prince of Wales visual work harder to regress. Representative TUI,
web, CLI, and report outputs should have capture/golden evidence and CREST
review notes that future pulses can trust.

## Deliverables

- Inventory existing visual tests and captures.
- Add missing representative captures where practical.
- Record CREST review notes for dashboard, fantasy/poach, team season, and
  report surfaces.
- Update visual-system docs with true capture commands.

## Discovery Scope

- `design/specs/visual-system.md`
- `design/notes/*prince*`
- `icelines-cli/tests/*prince*`
- `icelines-web/tests/*`
- `scripts/test-slice.ps1`
- `COMMANDS.md`

## Gates

- [ ] `cargo test -p icelines-cli prince_tui`
- [ ] `cargo test -p icelines-cli --test prince_cli_visual`
- [ ] `cargo test -p icelines-web l1_static_css_contains_prince_route_layout_classes`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 web-captures`
- [ ] Capture paths or browser-tooling blocker are recorded in the pulse result.

## Stop Conditions

- Stop if browser capture tooling is unavailable; document the blocker and keep
  static/template tests as the pulse result.
