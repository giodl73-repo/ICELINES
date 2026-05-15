# KEEL review - Compose the Bench plan

## Findings

- The wave correctly treats pane composition as shared identity in
  `icelines-core::workbench`, with TUI and web as adapters. Keep that invariant:
  if a pane model, field, or bound experience exists only in a template or TUI
  renderer, the wave has split the system.
- TUI may hold active pane selection as long-lived App state; web should keep
  durable meaning in safe URL/read state and local-only visibility preferences.
  Do not invent server-side dashboard session state.
- Bound experiences must not imply new data semantics. They are compositions of
  existing ViewModels/routes/screens.

## Required checks

- Pulse 02 needs tests that every shared binding lowers to at least one valid
  surface target or is explicitly marked unsupported.
- Pulse 03/04 must preserve canonical workspace routes and screen identities.
