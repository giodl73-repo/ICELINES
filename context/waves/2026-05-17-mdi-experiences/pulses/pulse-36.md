# Pulse 36: Serve full team workspace

## Goal

Continue the dashboard workspace QA pass by making team clicks behave like
Leaders and Player clicks: the center workspace should show the full team page,
not a compact summary preview.

## Changes

- Reused the real Team route projection for dashboard `/team/:abbr` workspaces.
- Extracted the team page's `<main>` content so the dashboard shell stays in
  place.
- Kept unsupported team subroutes guarded by the workspace allowlist.
- Added a route assertion that `/dashboard?workspace=/team/:abbr` includes the
  full roster and no nested page `<main>`.

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
