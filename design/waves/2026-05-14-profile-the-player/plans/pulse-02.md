# Pulse 02 - Player Records TUI Screen

## Goal

Make individual player records a first-class TUI player screen instead of only
a CLI/web handoff.

## Work completed

1. Added `Screen::PlayerRecordsById(PlayerId)` and wired SDI/MDI render
   dispatch.
2. Added `icelines-cli/src/tui/screens/player_records.rs`, rendering all current
   player record metrics from `PlayerRecordsView`.
3. Added `r` navigation from the player card to the records screen.
4. Changed `:records player <name>` in the MDI command bar to resolve the player
   and open the records screen.
5. Updated README and COMMANDS docs for the new player records surface.

## Result

Player cards now have a dedicated Records screen covering:

1. NHL teams scored against.
2. NHL goalies scored against.
3. Fight opponents.

The screen uses the shared records ViewModel and the existing local
boxscore/play-by-play records provider.

## Gates

- `cargo fmt --check`
- `cargo check -p icelines-cli`
- `cargo test -p icelines-cli l0_profile_exec_records_player_opens_tui_records_screen -- --nocapture`
- `cargo test -p icelines-cli player_records -- --nocapture`
- `proof check design\waves\2026-05-14-profile-the-player design\waves\PHASES.md --errors-only`
