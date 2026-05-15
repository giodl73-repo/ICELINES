# Pulse 04 - Web Dashboard Workbench Catalog

## Goal

Promote `/dashboard` from a hardcoded workspace-nav shell to the web version of
the shared workbench: grouped catalog navigation, explicit zone labels, visible
pane affordances, and ViewModel-backed pane models/fields. Keep all full routes
canonical and keep the dashboard server-rendered/no-JS useful.

## Governing roles

- **keel**: use the same catalog IDs and zone semantics as TUI.
- **glass**: the browser dashboard must show what the activity/catalog rail,
  workspace, context panes, live ribbon, and command/status surface do.
- **forge**: keep route lowering in web adapters; do not make web depend on
  `icelines-cli`.
- **wire**: no GET mutation, no external route navigation, no cache-warming from
  catalog clicks.
- **bench**: route/template/static tests must fence no-JS rendering,
  workspace-fragment behavior, URL invariants, and accessibility labels.

## Owned scope

1. Render grouped catalog entries from the shared web adapter instead of hardcoded
   dashboard nav links.
2. Add visible left/right pane selectors or field affordances for the first safe
   ViewModel-backed pane models, not only filter/dimension panes.
3. If web tabs are rendered, treat them as bound experience tabs that swap
   dashboard workspace/pane/field bindings while preserving canonical full
   routes.
4. Preserve `/dashboard?workspace=...` and `?partial=workspace` behavior.
5. Preserve local-only side-pane state.
6. Update web route/static tests for catalog and pane semantics.

## Non-goals

- No SPA rewrite.
- No new web data source.
- No new mutation path.
- No TUI changes.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -p icelines-web --no-deps -- -D warnings`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 web-captures`
