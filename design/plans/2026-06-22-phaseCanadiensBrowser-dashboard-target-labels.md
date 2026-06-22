# Phase Canadiens Browser - Dashboard Target Labels

Status: Closed
Date: 2026-06-22

## Intent

Make repeated dashboard target controls uniquely named for assistive
technologies.

## Scope

- Add accessible names to summary-row `Center`, `Left`, and `Right` target
  links.
- Add accessible names to contextual `More views` target links.
- Cover representative Poach summary and contextual action labels through the
  dashboard shell route test.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web l1_dashboard_shell_renders_no_js_regions`
- `git diff --check`
