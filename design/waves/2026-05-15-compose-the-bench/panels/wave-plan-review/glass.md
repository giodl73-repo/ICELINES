# GLASS review - Compose the Bench plan

## Findings

- The plan is useful only if the center workspace remains visually dominant.
  Pane controls should be compact, visible, and reversible; they must not become
  another hidden command language.
- Bound experiences can keep tabs, but the tab label should mean a composed
  working mode, not a legacy screen switch.
- TUI focus styling and web active/selected states need explicit tests because
  pane composition introduces more selected objects than the prior rail-only
  shell.

## Required checks

- Pulse 03 should test visible labels for the active left/right pane models.
- Pulse 04 should test no-JS desktop/mobile markup for pane selectors and active
  experience state.
