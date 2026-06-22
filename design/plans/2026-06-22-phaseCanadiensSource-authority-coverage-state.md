# Phase Canadiens Source - Scoring authority coverage state

Status: Closed

## Intent

Give scoring JSON consumers a compact authority coverage status without forcing
them to interpret the raw `SourceState` completeness enum.

## Scope

- Add `source_authority.coverage_state` to game, team, and player scoring JSON.
- Map complete PlayByPlay authority to `covered`.
- Preserve partial, stale, and unavailable authority states.
- Cover loaded and missing scoring-cache paths in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router rocket_game_scoring`
- `git diff --check`
