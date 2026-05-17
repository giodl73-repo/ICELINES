# Pulse 31: Serve center card expansion

## Goal

Fix the served dashboard center card still looking like a narrow pane on wide
browsers after the outer dashboard shell went full-width.

## Changes

- Replaced the dashboard workspace partial's nested `<main>` with a `<section>`.
- Kept the ARIA workspace label and dashboard data attributes intact.
- Added a route assertion so the center workspace no longer inherits the global
  page-level `main` width cap.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
