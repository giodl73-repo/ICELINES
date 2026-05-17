# Pulse 41: Serve pane navigation state hardening

## Goal

QA the pane-target navigation model and fix state-loss bugs found while
simulating center swaps and modifier-click pane pins.

## Changes

- Preserved composed dashboard layout state (`left`, `right`, and `experience`)
  whenever JavaScript rewrites a dashboard URL.
- Kept existing pinned pane URL state alongside the composed room and side-pane
  state.
- Strengthened the dashboard JavaScript static contract around state-copying.

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
