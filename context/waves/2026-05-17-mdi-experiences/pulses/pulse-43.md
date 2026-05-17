# Pulse 43: Serve server-side pinned-pane links

## Goal

Continue pane-target QA by fixing pinned pane loss for no-JS and open-in-new-tab
flows on room and pane-control links.

## Changes

- Threaded pinned left/right workspace URL state into server-rendered dashboard
  catalog, room tab, and pane-control links.
- Kept existing unpinned link output unchanged.
- Added unit coverage for pinned pane preservation in generated composition
  links.

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
