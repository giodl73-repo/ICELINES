# Pulse 04 - Player route cache ownership

## Findings

- R1 F-01: `/player/:id` mutates the shared web repository during a read-only page request.

## Scope

Move lazy career fan-out out of the shared active-season repo write path. Use a bounded request/cache layer or request-local repository construction so player pages remain concurrent reads against shared state.

## Gates

- Add a regression that opening a player page does not increase shared active repo career windows.
- `cargo fmt --check`
- `cargo test -p icelines-web`
- `cargo clippy --workspace -- -D warnings`

## Closeout

Closed in this pulse:

- HTML and JSON player-card handlers now build career fan-out in a request-local `StatsRepository`.
- The shared web `WebState.repo` stays read-only during player-card rendering.
- Existing active player stats, identity, and contract rows are copied into the local repo before fan-out so current-season projections remain intact.
- Added an L1 JSON route regression proving career rows still render while shared repo window count stays unchanged.
