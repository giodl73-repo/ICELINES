# R3 Review - forge

## Findings

### F-01 - NOTE: Keep catalog, field, pane, and experience types pure
File: `design/waves/2026-05-15-call-the-changes/plans/pulse-02.md`
Finding: Pulse 02 correctly assigns shared metadata to the foundation while
leaving concrete TUI `Screen` and web route lowering in adapters.
Consequence: This respects crate boundaries and avoids making `icelines-core`
depend on UI or web concepts.
Fix: Implement pure enum/static metadata in the lowest safe crate, then add TUI
and web adapter tests for lowering.

### F-02 - WARN: Bound experiences must not clone rendered ViewModels
File: `design/waves/2026-05-15-call-the-changes/MDI-WORKBENCH-INVENTORY.md`
Finding: A bound experience composes pane models and fields; it should not store
rendered pane data or cloned table rows.
Consequence: Storing rendered ViewModels in experience state would make swaps
stale and risk large clones in the long-lived TUI app.
Fix: Store IDs and field bindings only; derive current pane data from existing
ViewModels when rendering.
