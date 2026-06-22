# Phase Canadiens Browser - Dashboard Pinned Labels

Status: Closed
Date: 2026-06-22

## Intent

Make pinned-pane dashboard controls uniquely named for assistive technologies.

## Scope

- Add accessible names to left pinned workspace controls for opening in center,
  swapping with center, and clearing the pin.
- Add matching accessible names to right pinned workspace controls.
- Cover both pinned panes through the existing dashboard pinned-pane route test.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web l1_dashboard_shell_renders_pinned_pane_workspaces`
- `git diff --check`
