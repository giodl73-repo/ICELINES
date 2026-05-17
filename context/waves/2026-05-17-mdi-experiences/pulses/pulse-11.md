# Pulse 11: Serve side panes data-first

## Goal

Keep the served dashboard side columns focused on useful context instead of
opening with pane selectors and shared model metadata.

## Changes

- Moved favorites/watchlist and schedule content to the top of the side panes.
- Collapsed pane model details and pane switching links under a shared "Pane
  controls" details block.
- Updated the empty shared-pane copy so it directs users back to the center
  workspace instead of referring to metadata above the content.
- Added CSS and web route/static assertions for the new side-pane controls.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
