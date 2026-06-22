# Phase Canadiens Source - Scoring authority metric family

Status: Closed

## Intent

Make scoring source authority useful to downstream consumers by naming the exact
metric family covered by cached official NHL play-by-play.

## Scope

- Add `source_authority.covered_metrics` to scoring JSON metadata.
- Cover goals, shots on goal, attempts, unblocked attempts, missed shots,
  blocked shots, shot percentage, and strength state.
- Keep raw `source_state` and the human-readable authority label unchanged.
- Document the field in the command reference.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router rocket_game_scoring`
- `git diff --check`
