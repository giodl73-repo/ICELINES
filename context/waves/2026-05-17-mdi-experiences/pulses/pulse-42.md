# Pulse 42: Serve composition pinned-pane hardening

## Goal

Continue QA on pane-target navigation by fixing pinned pane loss when clicking
room or pane-control links that intentionally use full dashboard GET
navigation.

## Changes

- Added a dashboard composition-link handoff that preserves `left_workspace` and
  `right_workspace` before navigating.
- Kept normal browser modifier behavior for composition links.
- Strengthened the dashboard JavaScript static contract around composition URL
  rewriting.

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
