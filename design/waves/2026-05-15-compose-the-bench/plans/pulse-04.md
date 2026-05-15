# Pulse 04 - Web Pane Composition Controls

## Goal

Wire the shared pane binding contract into `/dashboard`. The browser dashboard
should expose server-rendered pane composition controls and bound experience
presets with useful no-JS output, safe URL/read state, and canonical full-route
workspace links.

## Governing roles

- **keel**: web composition must lower shared metadata, not invent a second pane
  catalog.
- **glass**: controls must remain glanceable across desktop/tablet/mobile.
- **forge**: keep handlers/templates simple and typed; avoid route-local clones.
- **wire**: GET state is read-only; favorite/watch/admin writes stay POST-backed.
- **bench**: add route/template tests for no-JS rendering, URL allowlisting,
  bound experience tabs, pane selectors, and mutation boundaries.

## Owned scope

1. Project shared pane bindings and bound experiences into dashboard template
   structs.
2. Render pane composition controls for left/right panes in the dashboard shell
   or workspace fragment.
3. Preserve canonical workspace routes and `?partial=workspace` behavior.
4. Keep side-pane visibility local browser state; pane composition URL state is
   allowed only if read-only and allowlisted.
5. Update dashboard JS/CSS only as needed for progressive enhancement.

## Non-goals

- No SPA rewrite.
- No new mutation routes.
- No live network fetches from pane selection.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -p icelines-web --no-deps -- -D warnings`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 web-captures`
