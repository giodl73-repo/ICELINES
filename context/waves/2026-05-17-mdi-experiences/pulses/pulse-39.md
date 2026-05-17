# Pulse 39: Serve full goalie and depth workspaces

## Goal

Continue dashboard workspace QA by making `/goalies` and `/depth` render their
full stat pages in the center workspace instead of compact preview tables.

## Changes

- Reused the real Goalies and Depth route projections for dashboard workspaces.
- Preserved Goalies query state while extracting the page `<main>` content.
- Added route assertions for full Goalies and Depth workspace fragments.

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
