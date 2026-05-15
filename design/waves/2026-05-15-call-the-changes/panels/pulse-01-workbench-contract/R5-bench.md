# R5 Review - bench

## Findings

### F-01 - WARN: Test atomic experience swaps
File: `design/waves/2026-05-15-call-the-changes/MDI-WORKBENCH-INVENTORY.md`
Finding: Bound experience tabs swap center workspace, pane models, ribbon scope,
and active fields together.
Consequence: A partial swap could leave the center on one experience while panes
or fields still describe the previous experience.
Fix: Pulse 02 should test the static bindings, Pulse 03 should test TUI swaps,
and Pulse 04 should test dashboard swaps preserve the workspace URL and pane
state invariants.

### F-02 - NOTE: Existing gate split is appropriate
File: `design/waves/2026-05-15-call-the-changes/plans/pulse-02.md`
Finding: The pulse split fences core metadata first, then TUI and web rendering,
then docs/closeout.
Consequence: This gives each implementation pulse a testable boundary instead
of mixing shared identity, rendering, and documentation in one large change.
Fix: Keep Pulse 02 layout-free and require later pulses to consume its metadata.
