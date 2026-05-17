# Pulse 15: Serve left pane leader fallback

## Goal

Keep the served dashboard left pane data-bearing even before the user has saved
favorites or watchlist entries.

## Changes

- Added a leaders preview to the dashboard template context.
- Rendered top leaders in empty favorites and watchlist panes.
- Styled the fallback rows as compact preview cards that link to `/leaders`.
- Added route/static assertions for the side-pane data fallback.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
