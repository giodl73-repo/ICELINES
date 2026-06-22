# Phase Canadiens Source - Scoring authority metadata

Status: Closed

## Intent

Make scoring reports state their source authority explicitly, so major-stats
comparison work can distinguish cached official NHL play-by-play scoring-event
metrics from season-total or inferred metrics.

## Scope

- Add `meta.source_authority` to game, team, and player scoring JSON routes.
- Render the same authority label in Web scoring report banners.
- Preserve raw `source_state` for machine audit and expose authority as a
  stable summary over the PlayByPlay source.
- Cover loaded and missing scoring-cache states in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router rocket_game_scoring`
- `git diff --check`
