# Pulse 05 - Player hub navigation polish

## Goal

Make the player card feel like a hub for every player-specific surface rather
than a dead-end detail card.

## Scope

- Surface records, awards, streaks, scouting, compare, group/favorite, and watch
  handoffs from the TUI player card.
- Add command-bar handoffs for scouting and linemate/deployment commands.
- Add web player-card links for the first-class player routes.
- Update docs and wave inventory so the player screen map matches the shipped
  navigation.

## Role lenses

- **glass**: users should discover the next player-specific screen from the
  current player, not memorize the whole CLI.
- **edge**: command-bar handoffs should name canonical commands/routes rather
  than duplicate business logic in the TUI.
- **forge**: keep navigation as UI wiring; no new player computations in this
  pulse.
- **wire**: web links should point only to real routes.

## Gates

- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p icelines-cli profile_parse_player_hub -- --nocapture`
- `cargo test -p icelines-web`
- `C:/src/proof/target/debug/proof check README.md COMMANDS.md design/waves/PHASES.md design/waves/2026-05-14-profile-the-player/WAVE.md design/waves/2026-05-14-profile-the-player/PLAYER-SCREEN-MAP.md design/waves/2026-05-14-profile-the-player/plans/pulse-05.md --errors-only`

## Outcome

Implemented:

- TUI player-card hub line for records, awards, streaks, compare, groups, and
  favorites.
- TUI command-bar handoffs for `:scouting player <name>` and
  `:mates player <name>`.
- Web player-card links for Records, Awards, Streaks, and Scouting.
