# Pulse 14: Serve scores ribbon preview

## Goal

Make the served dashboard's top scores ribbon show actual game context rather
than another generic navigation message.

## Changes

- Added `scores_preview` to the dashboard template context using the same
  summary rows as the Scores workspace.
- Rendered the top scores ribbon as compact score chips with slate and game
  details.
- Preserved the full Scores page link while making the default top row
  data-first.
- Added web route/static assertions for the score preview strip.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
