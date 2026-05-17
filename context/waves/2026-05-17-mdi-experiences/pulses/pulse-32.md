# Pulse 32: Serve full leaders workspace

## Goal

Make the served dashboard center workspace show the full Leaders experience
instead of the compact three-column preview when the active workspace is
Leaders.

## Changes

- Reused the Leaders route model builder for dashboard rendering.
- Added a full Leaders workspace branch with the position chips, stat filter,
  bio filters, sortable headers, player rows, and JSON link.
- Kept non-Leaders workspaces on the compact dashboard summary preview.
- Added a route assertion that `/dashboard?workspace=/leaders` renders the full
  Leaders surface and no longer renders the Leaders preview table.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
