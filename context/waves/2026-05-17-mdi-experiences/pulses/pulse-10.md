# Pulse 10: Serve dashboard data-first shell

## Goal

Make the served browser dashboard feel like an immediate workspace instead of a
menu catalog, with leaderboards and other data previews occupying the center.

## Changes

- Replaced the center workspace intro with a compact heading, full-page action,
  and table-style data preview.
- Collapsed secondary workspace links and workbench wiring behind details
  sections so they no longer lead the page.
- Shortened the top activity catalog and experience tabs by rendering compact
  chips while retaining details in link titles.
- Increased dashboard workspace previews to ten rows so the center pane reads as
  a real leaderboard/data surface.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`

## Status

Done.
