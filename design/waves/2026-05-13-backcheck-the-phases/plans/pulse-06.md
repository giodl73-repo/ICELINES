---
wave: backcheck-the-phases
pulse: 06
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - bench
  - forge
  - glass
  - tape
  - wire
---

# Pulse 06 - Selke Watch-Rule TUI Editor and UX

## Mission

Close the explicit Selke/Ted Lindsay carry-forward for watch-rule UX without
changing the scoring model: users can already create/toggle/history rules from
CLI/web and view rule state in TUI, but richer TUI arbitrary-rule editing remains
partial.

## Deliverables

- Inventory current CLI/TUI/web watch-rule and watchlist capabilities.
- Add a TUI rule editor only if it can use existing `WatchRulesView` and
  mutation/result contracts.
- Preserve local watchlist notes, fired-alert history, and dashboard watch
  command behavior.
- Update `design/specs/surface-parity.md`, `COMMANDS.md`, and any relevant TUI
  help/chrome docs.

## Likely Files

- `icelines-core/src/view_model/poach.rs`
- `icelines-cli/src/tui/screens/poach.rs`
- `icelines-cli/src/tui/screens/favorites.rs`
- `icelines-cli/src/tui/chrome.rs`
- `icelines-web/src/handlers/poach.rs`
- `icelines-web/src/handlers/dashboard.rs`
- `design/specs/surface-parity.md`
- `COMMANDS.md`

## Gates

- [ ] `cargo test -p icelines-core watch`
- [ ] `cargo test -p icelines-cli watch`
- [ ] `cargo test -p icelines-cli tui_watch`
- [ ] `cargo test -p icelines-web watch_rule`
- [ ] `cargo fmt --check`
- [ ] Surface parity row explains any remaining non-TUI editing limitation.

## Stop Conditions

- Stop if rule editing would require a ViewModel field or mutation result that
  does not exist; create a ViewModel pulse instead.
- Stop if any mutation would become GET-backed.
- Stop if unavailable shift/line data would be treated as negative evidence.
