# Pulse 35: Serve navigation QA hardening

## Goal

Exercise the new dashboard workspace navigation mode and fix routes that could
no-op or swap to the wrong center workspace.

## Changes

- Added a client-side workspace allowlist that matches the server workspace
  renderer.
- Narrowed server workspace acceptance for player, team, and game routes so
  unsupported subroutes do not masquerade as dashboard workspaces.
- Made failed workspace swaps fall back to normal navigation instead of getting
  stuck.
- Added route coverage for unsupported dashboard workspace targets.

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
