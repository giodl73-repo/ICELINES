# Pulse 16: Serve right pane schedule preview

## Goal

Make the served dashboard right pane data-first by showing schedule rows instead
of a generic list of schedule links.

## Changes

- Added a `schedule_preview` dashboard template context using the Schedule
  workspace summary rows.
- Rendered schedule preview cards at the top of the right pane.
- Moved the older schedule links behind a "Schedule views" details disclosure.
- Added route assertions for the new schedule preview controls.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
