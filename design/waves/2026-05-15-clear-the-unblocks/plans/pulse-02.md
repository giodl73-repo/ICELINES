---
wave: clear-the-unblocks
pulse: 02
date: 2026-05-15
status: planned
governing_roles:
  - bench
  - glass
  - forge
---

# Pulse 02 - Headshot and Admin-Overlay Spec Truth

## Goal

Update stale spec/index claims for headshot rendering and the TUI admin overlay
so the docs reflect existing focused test coverage.

## Owned Scope

- Update `design/specs/headshot-rendering.md` test coverage notes.
- Update `design/specs/tui-admin-overlay.md` test coverage notes.
- Update `design/plans/INDEX.md` Tier 2 backlog rows if the gaps are already
  covered.
- Add focused tests only if discovery finds a named coverage claim is still
  untrue.

## Non-goals

- No CDN/network tests for headshots.
- No admin-overlay v2 action menu.
- No visual redesign.

## Gates

- [ ] `cargo test -p icelines-cli headshot --quiet`
- [ ] `cargo test -p icelines-cli admin_overlay --quiet`
- [ ] `C:\src\proof\target\debug\proof.exe check design\specs\headshot-rendering.md design\specs\tui-admin-overlay.md design\plans\INDEX.md --errors-only`
