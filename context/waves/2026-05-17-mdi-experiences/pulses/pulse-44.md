# Pulse 44: Serve pinned pane actions

## Goal

Make pane-target navigation easier to rearrange after a user pins content into
the left or right side pane.

## Changes

- Added pinned pane header links for Open in center, Swap with center, and Clear
  pin.
- Generated the action hrefs server-side so they work with no JavaScript and
  preserve dashboard room/pane composition state.
- Extended dashboard route coverage to assert the pinned pane action links are
  present and carry the expected workspace state.

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
