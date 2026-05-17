# Pulse 37: Serve full team season workspace

## Goal

Continue dashboard workspace QA by making `/team/:abbr/season` render the full
team-season page in the center workspace instead of the compact summary.

## Changes

- Reused the real Team Season route projection for dashboard team-season
  workspaces.
- Extracted the team-season page's `<main>` content so the dashboard shell stays
  in place.
- Added a route assertion that `/dashboard?workspace=/team/:abbr/season`
  includes the full season summary and no nested page `<main>`.

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
