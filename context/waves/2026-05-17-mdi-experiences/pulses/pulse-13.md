# Pulse 13: Serve workspace catalog disclosure

## Goal

Remove the last always-open workspace menu row from the served dashboard while
keeping the full catalog available when users need it.

## Changes

- Wrapped the activity catalog rail in a collapsed "Workspaces" details block.
- Kept the catalog groups and workspace links intact inside the disclosure.
- Added CSS for the collapsed catalog list and preserved horizontal scrolling
  after the disclosure is opened.
- Updated web route/static assertions for the new catalog shell.

## Validation

- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
