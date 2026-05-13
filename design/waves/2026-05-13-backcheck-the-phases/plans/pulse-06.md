---
wave: backcheck-the-phases
pulse: 06
date: 2026-05-13
status: done
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

- [x] `cargo test -p icelines-core watch`
- [x] `cargo test -p icelines-cli watch`
- [x] `cargo test -p icelines-cli tui_watch`
- [x] `cargo test -p icelines-web watch_rule`
- [x] `cargo fmt --check`
- [x] Surface parity row explains any remaining non-TUI editing limitation.

## Gate Notes

- `cargo test -p icelines-core watch` matched 7 tests.
- `cargo test -p icelines-cli watch` matched 21 tests.
- `cargo test -p icelines-cli tui_watch` matched 2 tests.
- `cargo test -p icelines-web watch_rule` matched 11 tests.
- TUI editing is intentionally limited to player-rule create and
  enable/disable; destructive delete remains outside TUI because the current
  schema cascades fired-alert history on delete.

## Stop Conditions

- Stop if rule editing would require a ViewModel field or mutation result that
  does not exist; create a ViewModel pulse instead.
- Stop if any mutation would become GET-backed.
- Stop if unavailable shift/line data would be treated as negative evidence.
