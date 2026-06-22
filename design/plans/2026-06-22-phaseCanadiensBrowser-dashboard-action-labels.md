# Phase Canadiens Browser - Dashboard Action Labels

Status: Closed
Date: 2026-06-22

## Intent

Make repeated dashboard workspace action links screen-reader-specific without
changing the visible compact controls.

## Scope

- Add workspace-specific accessible names to the central workspace action links:
  open full workspace, pin left, and pin right.
- Cover the labels through the existing dashboard shell route test.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web l1_dashboard_shell_renders_no_js_regions`
- `git diff --check`
