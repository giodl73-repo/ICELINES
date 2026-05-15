# Pulse 02 - Shared Pane Binding Contract

## Goal

Add the shared typed metadata needed for pane composition. Bound experiences
should declare center workspace, supported surfaces, left/right pane bindings,
active fields, and top/bottom scopes through `icelines-core::workbench`, with
TUI/web adapters lowering those bindings to surface-specific targets.

## Governing roles

- **keel**: core identity first; surface adapters only lower shared metadata.
- **glass**: labels, descriptions, and defaults must be readable enough for
  picker UI.
- **forge**: keep core pure and small; use typed IDs and compile-time tables.
- **wire**: do not encode mutations as navigable pane targets.
- **bench**: add L0 tests for binding completeness and adapter tests for lowering.

## Owned scope

1. Extend `icelines-core/src/workbench.rs` with pane binding/composition metadata
   identified by the Pulse 01 inventory.
2. Add core tests for:
   - every bound experience has a center workspace;
   - every left/right pane binding references an existing pane model;
   - every active field references an existing field;
   - surface support is explicit;
   - no action/status pane is represented as a GET mutation.
3. Update `icelines-cli/src/tui/workbench.rs` and `icelines-web/src/workbench.rs`
   adapters only as needed to expose the new metadata.
4. Add adapter tests for route/screen lowering.

## Non-goals

- No visible TUI or web control wiring yet.
- No persistent user preferences.
- No data loading or new ViewModels.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-cli --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-core --no-deps -- -D warnings`
