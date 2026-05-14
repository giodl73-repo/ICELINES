# Pulse 04 - Player Awards Trophy Case

## Goal

Make NHL awards a first-class player surface backed by the official NHL landing
endpoint `awards[]` array.

## Work completed

1. Added `PlayerAwardsView` plus trophy and trophy-season rows in
   `icelines-core`.
2. Added landing-awards parsing and a local `player_awards.json` cache in
   `icelines-fetch`.
3. Added `icelines awards <player>` with table, CSV, JSON, and `--out` output.
4. Added web routes `/player/:id/awards` and `/api/v1/player/:id/awards`.
5. Added `Screen::PlayerAwardsById`, `a` from player cards, and
   `:awards player <name>` command-bar navigation.

## Result

The player Trophy Case now shows official NHL awards such as Art Ross, Hart,
Conn Smythe, Rocket Richard, and Ted Lindsay when the landing endpoint provides
them. The screen does not infer awards from season totals.

## Gates

- `cargo fmt --check`
- `cargo check -p icelines-cli`
- `cargo test -p icelines-core awards -- --nocapture`
- `cargo test -p icelines-fetch awards -- --nocapture`
- `cargo test -p icelines-cli awards -- --nocapture`
- `cargo test -p icelines-web`
- `proof check design\waves\2026-05-14-profile-the-player design\waves\PHASES.md --errors-only`
