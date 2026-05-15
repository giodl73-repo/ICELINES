# Pulse 03 - TUI Full-MDI Workbench Shell

## Goal

Replace default TUI MDI's tab/command-first navigation with a workbench shell:
activity/catalog rail, center workspace, left/right context panes, top live
ribbon, bottom command/status surface, and explicit focus movement. Preserve
`--classic` tab cycling and `--standalone` focus mode.

## Governing roles

- **keel**: use the shared catalog from Pulse 02 for screen identity and zone
  placement.
- **glass**: the default TUI must be glanceable: users can see available
  workspaces and understand each zone without opening help.
- **forge**: keep `App::screen` as the workspace discriminator. Avoid parallel
  enums and avoid clones of repo-backed view data.
- **wire**: screen selection cannot trigger live fetch or mutation beyond existing
  cache/read behavior for the selected surface.
- **bench**: regression tests must prove MDI, classic, and standalone navigation
  semantics differ intentionally.

## Owned scope

1. Render a catalog/activity rail in MDI mode from the shared catalog.
2. Add MDI zone focus state and Tab/Shift+Tab focus movement.
3. Add context-pane selection scaffolding for left/right panes, starting with
   existing Favorites/Watchlist and Schedule behavior but using the shared pane
   model vocabulary so future navigator, inspector, summary, timeline, compare,
   queue, source-state, and action/status panes fit the same rail.
4. Route catalog activation to `App::screen` using the shared TUI adapter.
5. Update TUI help/chrome strings touched by the new MDI behavior.

## Non-goals

- No web changes.
- No removal of `--classic`.
- No standalone behavior changes except tests proving it remains locked.
- No implementation of every context-pane option from the inventory.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo clippy -p icelines-cli --no-deps -- -D warnings`
