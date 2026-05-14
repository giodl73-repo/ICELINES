# Pulse 05 - TUI storage error surfacing

## Findings

- R6 F-01: TUI Favorites swallows group DB/view-construction errors and renders empty/fallback state.

## Scope

Preserve storage and view-construction errors and surface them through the TUI status/chrome flash while keeping the legitimate empty-group state unchanged.

## Gates

- Add L0 renderer/state tests for storage failure vs empty group.
- `cargo fmt --check`
- `cargo test -p icelines-cli favorites`
- `cargo clippy --workspace -- -D warnings`
