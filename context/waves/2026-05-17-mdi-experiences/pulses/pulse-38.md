# Pulse 38: Serve full slate workspaces

## Goal

Continue dashboard workspace QA by making `/scores` and `/schedule` render their
full pages in the center workspace instead of compact preview tables.

## Changes

- Reused the real Scores and Schedule route projections for dashboard workspaces.
- Extracted each page's `<main>` content so the dashboard shell, side panes, and
  bottom chrome stay in place.
- Added route assertions for full Scores and Schedule workspace fragments.

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
