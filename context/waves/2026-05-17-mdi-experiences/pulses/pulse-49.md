# Pulse 49: Serve full game workspace

## Goal

Keep score and schedule game clicks inside the MDI center with the full game
detail surface instead of falling back to a compact dashboard preview.

## Changes

- Exposed a reusable game-detail template builder for dashboard embedding.
- Added full `/game/:id` workspace rendering in the dashboard center pane.
- Added dashboard workspace template fields and rendering branch for full game
  surfaces.
- Extended dashboard workspace allowlist and label unit coverage for game
  workspaces.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `cargo test -p icelines-web static_assets::tests::l0_dashboard_js_carries_workspace_fragment_contract`
- `cargo test -p icelines-web dashboard::tests::l0_dashboard_workspace_rejects_external_or_internal_api_paths`
- `git diff --check`
- `cargo build --release`

## Status

Done.
