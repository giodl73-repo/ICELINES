# Phase Canadiens Strength - Strength summaries

Status: Closed

## Intent

Promote cached NHL play-by-play scoring reports from raw situation-only splits to
major-stats-style aggregate strength buckets while keeping situation-code
auditability intact.

## Scope

- Add aggregate scoring report splits for even strength, power play, and penalty
  kill across raw NHL `situationCode` values.
- Surface the aggregates in game, team, and player scoring report JSON.
- Render a Web scoring report By strength table before the detailed By
  situation table.
- Preserve existing situation and event `data-*` strength-state hooks.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core scoring::tests::l0_scoring`
- `cargo test -p icelines-web --test l1_router rocket_game_scoring`
- `git diff --check`
