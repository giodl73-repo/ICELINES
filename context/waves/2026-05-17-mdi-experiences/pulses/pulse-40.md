# Pulse 40: Serve pane-target navigation

## Goal

Let dashboard users send supported app links to the left or right side pane
without replacing the center workspace.

## Changes

- Added optional `left_workspace` and `right_workspace` dashboard query state.
- Rendered pinned side-pane previews from the same workspace summary model used
  by compact center previews.
- Updated dashboard link interception so Ctrl-click pins a supported route left,
  while Ctrl+Shift-click pins it right.
- Preserved pinned pane state across center workspace swaps.

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
