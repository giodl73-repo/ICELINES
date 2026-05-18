# Pulse 48: Serve explicit target controls

## Goal

Make dashboard pane targeting discoverable from visible controls instead of
requiring users to remember modifier-clicks.

## Changes

- Added a Target column to generic dashboard preview tables with explicit
  Center, Left, and Right actions.
- Added Center, Left, and Right actions to dashboard workspace cards.
- Extended dashboard JS to treat `data-dashboard-target="left|right"` as
  explicit pane-target navigation, matching Ctrl-click behavior without
  requiring modifier keys.
- Added static-asset and route coverage for the explicit target controls.

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
