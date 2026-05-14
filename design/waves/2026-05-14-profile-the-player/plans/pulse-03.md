# Pulse 03 - Player streaks screen

## Goal

Make streaks a first-class player surface backed by cached game-level rows, not
season totals.

## Scope

- Add a core `PlayerStreaksView` that computes current and longest goal,
  assist, and point streaks from ordered per-game skater rows.
- Add a fetch provider that reads cached boxscore manifests and projects player
  game lines.
- Expose streaks through CLI, TUI, web HTML, and API JSON surfaces.
- Wire player-card discoverability and command-bar navigation.

## Role lenses

- **edge**: streaks are window/game-row semantics; never infer from aggregate
  season totals.
- **tape**: source is persisted boxscore `playerByGameStats` rows and the
  manifest, with missing rows represented as an empty local-input result.
- **forge**: ViewModel owns streak computation; renderers only format.
- **glass**: player card should make records, awards, and streaks discoverable
  with direct keys and links.
- **wire**: web route and JSON twin should share the same `PlayerStreaksView`.

## Gates

- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p icelines-core streaks -- --nocapture`
- `cargo test -p icelines-fetch streaks_provider -- --nocapture`
- `cargo test -p icelines-cli profile_parse_streaks -- --nocapture`
- `cargo test -p icelines-web`
- `C:/src/proof/target/debug/proof check . --errors-only`

## Outcome

Implemented:

- `icelines-core::PlayerStreaksView`
- `icelines-fetch::streaks_provider::load_player_game_lines`
- `icelines streaks <player> [--json|--csv] [--out PATH]`
- TUI `Screen::PlayerStreaksById`, `s` from player card, and
  `:streaks player <name>`
- Web `/player/:id/streaks` and `/api/v1/player/:id/streaks`
