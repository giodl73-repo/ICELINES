# WP-010 Pulse 04 - Signals TUI + Web surface parity

Date: 2026-06-19

## Scope

Extend Phase Hurricane Signals beyond CLI/JSON by rendering the same
`PlayerSignalsView` on the TUI player card and on Web HTML/API player routes.
Signals remain descriptive and stay out of `StatId`, the `--filter` catalog,
leaderboards, reports, exports, and the analytics cache.

## Changes

- Added `icelines-web::handlers::signals` with:
  - `GET /player/:id/signals` HTML;
  - `GET /api/v1/player/:id/signals` JSON using route `player-signals`;
  - active-window player lookup with latest loaded career-row fallback, matching
    player-card behavior.
- Added a Signals link to the Web player-card action row and footer.
- Added a TUI player-card Signals block rendered from `PlayerSignalsView`, with
  CLI and Web handoffs.
- Updated `surface-parity.md`, `COMMANDS.md`, and the Signals methodology spec.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-cli --bin icelines signals` | passed |
| L0/L1 | `cargo test -p icelines-web signals` | passed |
| L1 | `cargo test -p icelines-web --test l1_router player_signals` | passed |
| L1 | `cargo test -p icelines-web --test ted_lindsay_route_inventory` | passed |
| Format | `cargo fmt --check`; `git diff --check` | passed |

The TUI and Web fences prove unavailable evidence stays unavailable/null and is
not rendered as `0.0`. The Web JSON route exposes the serialized
`PlayerSignalsView`, while the Web HTML route renders the same rows with evidence
tier and missing-input labels.

## Residual risk

- The TUI implementation is a player-card block rather than a dedicated modal or
  navigable screen; this is sufficient for parity but not a full inspection UI.
- Signals are still descriptive and scorer-biased where realtime rink-recorded
  events are inputs.
- No additional signal families, cache metric families, report/export columns,
  leaderboards, filters, or `StatId` promotions are claimed.
