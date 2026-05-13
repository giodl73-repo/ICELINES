---
wave: backcheck-the-phases
pulse: 02
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - crest
  - edge
  - glass
  - wire
  - tape
---

# Pulse 02 - Jack Adams Web Dashboard Continuity

## Mission

Finish the browser dashboard continuity pass: users should stay inside the
Jack Adams shell unless they explicitly choose "Open full page." Workspace
links, side-pane links, command mutations, command examples, and docs must all
preserve dashboard state.

## Current Context

Recent commits made `/dashboard` visible, added a workspace rail, aligned the
web command grammar with TUI examples, and fixed command form workspace state
after JavaScript partial swaps.

## Deliverables

- Dashboard side-pane links route through `/dashboard?workspace=...`.
- Workspace cards stay dashboard-aware.
- Explicit "Open full page" remains the escape hatch.
- Tests prove side-pane and workspace links do not accidentally eject users
  from the dashboard.
- Release build is rebuilt if embedded templates/static assets change.

## Likely Files

- `icelines-web/src/handlers/dashboard.rs`
- `icelines-web/templates/dashboard.html`
- `icelines-web/templates/dashboard_workspace.html`
- `icelines-web/static/dashboard.js`
- `icelines-web/tests/l1_router.rs`
- `icelines-web/src/static_assets.rs`

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-web l1_dashboard_shell_renders_no_js_regions`
- [ ] `cargo test -p icelines-web l1_dashboard_workspace_partial_renders_fragment_only`
- [ ] `cargo test -p icelines-web dashboard_command`
- [ ] `cargo check -p icelines-web`
- [ ] `cargo build --release -p icelines-cli`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1`

## Stop Conditions

- Stop if making links dashboard-aware would hide canonical full-page routes.
- Stop if a link target is a mutation route; mutations must stay POST-backed.
