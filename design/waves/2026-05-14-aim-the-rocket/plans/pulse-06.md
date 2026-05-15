# Pulse 06 - Player Scoring Profiles and Projections

## Goal

Complete the first Rocket Richard wave with a player-level scoring profile
surface that reuses official NHL play-by-play scoring events and avoids
claiming proprietary scoring-chance or betting-model outputs.

## Governing roles

- **scout**: player rows should explain shot volume and finishing from owned
  data, not project lineups or odds.
- **edge**: player matching must use optional shooter/scorer IDs and preserve
  missing-source state.
- **wire**: expose HTML and JSON twins; GET routes only read cached
  `DataKind::PlayByPlay`.
- **bench**: use local manifest fixtures; no network tests.

## Owned scope

1. Add a DataStore-backed `load_player_scoring_profile` provider.
2. Add `/player/:id/scoring` and `/api/v1/player/:id/scoring`.
3. Reuse the scoring report template shape for player summary, splits, and
   event detail.
4. Document the routes and close the wave evidence.

## Non-goals

- No betting odds, win probability, projected lineups, or third-party xG.
- No new cache shard; this reads play-by-play.
- No TUI/player-card tab in this pulse.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-core -p icelines-fetch -p icelines-web -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-14-aim-the-rocket design\waves\PHASES.md COMMANDS.md design\specs\surface-parity.md --errors-only`
