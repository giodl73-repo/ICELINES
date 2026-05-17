# Pulse 30: Serve wide dashboard layout

## Goal

Make the served dashboard use a maximized browser window instead of staying
centered inside the global page width cap.

## Changes

- Let the dashboard shell break out to the viewport width while preserving page
  padding.
- Narrowed the side pane bands and gave the center workspace a larger minimum so
  wide screens prioritize the data table.
- Kept mobile layout stacked and full-width.
- Added static CSS assertions for the wide-shell and center-priority rules.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `cargo test -p icelines-web --test l1_router dashboard`
- `git diff --check`

## Status

Done.
