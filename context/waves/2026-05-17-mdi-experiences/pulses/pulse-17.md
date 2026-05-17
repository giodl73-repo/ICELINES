# Pulse 17: Serve unified navigation drawer

## Goal

Remove the remaining extra top navigation row by combining workspace and room
navigation into one collapsed browser dashboard drawer.

## Changes

- Merged the Workspaces and Rooms disclosures into one "Navigation" details
  block.
- Kept workspace catalog groups and bound room links inside separate labeled
  sections within the drawer.
- Added CSS for the unified drawer layout and row scrolling.
- Updated web route/static assertions for the combined navigation shell.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
