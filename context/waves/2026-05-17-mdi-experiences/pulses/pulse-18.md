# Pulse 18: Serve clickable data preview

## Goal

Make the served dashboard center preview table actionable without requiring the
user to find the separate "Open full" header action.

## Changes

- Wrapped each center preview row cell in a link to the active workspace.
- Added hover styling for clickable preview rows.
- Added route/static assertions for the new linked row contract.

## Validation

- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `git diff --check`

## Status

Done.
