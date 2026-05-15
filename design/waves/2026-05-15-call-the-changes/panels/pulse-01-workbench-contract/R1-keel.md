# R1 Review - keel

## Findings

### F-01 - NOTE: Keep experience tabs on the shared identity axis
File: `design/waves/2026-05-15-call-the-changes/MDI-WORKBENCH-INVENTORY.md`
Finding: The inventory now distinguishes the screen catalog from bound
experience tabs, where a tab composes center workspace, pane bindings, ribbon
scope, and active fields.
Consequence: If Pulse 02 lets TUI and web define those experience tabs locally,
the workbench will drift into two separate navigation models even if the screen
catalog itself is shared.
Fix: Model bound experiences beside the shared catalog/field/pane metadata in
Pulse 02, then let TUI and web adapters lower them to surface-specific state.

### F-02 - NOTE: Preserve classic tab compatibility as a separate mode
File: `design/waves/2026-05-15-call-the-changes/plans/pulse-03.md`
Finding: The plan keeps `--classic` as the only mode where Tab/Shift+Tab cycle
screens.
Consequence: This cleanly separates legacy screen cycling from the new
workbench experience-tab concept.
Fix: Pulse 03 tests should assert that default MDI, classic, and standalone
interpret navigation differently by design.
