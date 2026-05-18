# Pulse 45: Serve pinned pane header labels

## Goal

Continue pane-action QA by making pinned side panes identify their pinned
workspace in the pane header instead of showing the default pane binding label.

## Changes

- Updated left and right pane titles to render `Pinned: <workspace>` when a pane
  has a pinned workspace.
- Kept existing binding titles for unpinned panes.
- Extended dashboard shell coverage for pinned pane header labels.

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
