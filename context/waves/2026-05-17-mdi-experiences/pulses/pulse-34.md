# Pulse 34: Serve full player workspace

## Goal

Make dashboard player navigation show the full player card in the center
workspace instead of the compact summary preview.

## Changes

- Reused the real Player route projection for dashboard player workspaces.
- Extracted the player page's `<main>` content so the dashboard keeps its shell,
  side panes, score ribbon, and command bar.
- Kept non-player workspaces on their existing dashboard summary rendering.
- Added a router assertion that `/dashboard?workspace=/player/:id` includes the
  full player card and no nested page `<main>`.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `cargo test -p icelines-web static_assets::tests::l0_dashboard_js_carries_workspace_fragment_contract`
- `git diff --check`
- `cargo build --release`

## Status

Done.
