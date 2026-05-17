# Pulse 12: Serve command jump bar

## Goal

Keep the served dashboard command palette available without making it feel like
another always-open menu surface.

## Changes

- Reworked the command footer into a compact jump bar with `/` and Ctrl+K
  shortcut hints.
- Moved command examples behind a "Command examples" details block.
- Expanded the command placeholder with the most useful workspace jumps so users
  can act without opening the examples.
- Added CSS and route/static assertions for the compact command shell.

## Validation

- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
