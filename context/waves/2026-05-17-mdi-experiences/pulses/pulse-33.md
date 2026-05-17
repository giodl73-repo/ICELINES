# Pulse 33: Serve workspace-local navigation

## Goal

Keep the Jack Adams dashboard shell in place when users navigate from the full
Leaders workspace, including player/team clicks, sort links, and filter forms.

## Changes

- Added dashboard JavaScript routing for same-origin app links.
- Added workspace-local GET form handling for embedded workspace filters.
- Marked full Leaders player/team links with explicit workspace targets while
  retaining normal `href` fallbacks.
- Preserved Leaders query state when `/dashboard?workspace=/leaders?...` renders
  the full Leaders workspace.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`
- `cargo build --release`

## Status

Done.
