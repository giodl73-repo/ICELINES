# Pulse 47: Serve workspace orbit controls

## Goal

Expand the player detail ring into a broader center-stable orbit model so users
can keep the middle workspace fixed while cycling or pinning side-pane context.

## Changes

- Added a left context ring that cycles side-pane context through Favorites,
  Watchlist, and League leaders while keeping the center workspace stable.
- Generalized the right detail ring for player and team workspaces.
- Added direct `Pin left` and `Pin right` actions in the center workspace header.
- Added keyboard shortcuts for orbit controls: `[` cycles left context, `]`
  cycles right detail, and `\` swaps the active pinned side pane with center.
- Added duplicate-safe ring cycling so the right ring skips the centered
  workspace and reports ring position in the control label.
- Added route and static-asset coverage for the orbit controls.

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
