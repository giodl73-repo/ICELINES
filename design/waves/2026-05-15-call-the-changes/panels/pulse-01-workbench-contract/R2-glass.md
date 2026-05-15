# R2 Review - glass

## Findings

### F-01 - WARN: Experience tabs need visible composition labels
File: `design/waves/2026-05-15-call-the-changes/MDI-WORKBENCH-INVENTORY.md`
Finding: Bound experience tabs can be powerful, but users must be able to see
that a tab changes a composed workspace, not merely a single screen.
Consequence: If the UI renders these like ordinary old tabs, users will expect
screen cycling and miss why side panes, fields, and ribbon context changed.
Fix: Pulse 03/04 should label experience tabs with enough context to show their
bindings, such as "Tonight bench" with center/pane/field summary chips or a
compact tooltip/help row.

### F-02 - NOTE: The zone model passes the glanceability threshold
File: `design/waves/2026-05-15-call-the-changes/WAVE.md`
Finding: The wave now names activity rail, optional experience tabs, center
workspace, left/right panes, top ribbon, bottom command/status, and overlays.
Consequence: This gives TUI and web enough visible structure to avoid
command-only discovery.
Fix: Carry these zone names into user-facing help/docs in Pulse 05.
