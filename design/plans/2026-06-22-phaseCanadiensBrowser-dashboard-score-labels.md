# Phase Canadiens Browser - Dashboard Score Labels

Status: Closed
Date: 2026-06-22

## Intent

Make the compact dashboard scores ribbon links explicit for assistive
technologies.

## Scope

- Add accessible names to score preview chips.
- Add an accessible name to the full scores workspace link.
- Cover the rendered labels through the dashboard shell route test.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web l1_dashboard_shell_renders_no_js_regions`
- `git diff --check`
