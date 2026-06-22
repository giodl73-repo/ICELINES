# Phase Canadiens Source - Player streak authority

Status: Closed

## Intent

Make player streak source authority explicit for both boxscore-derived streaks
and play-by-play-derived shot streaks.

## Scope

- Add player streak JSON `meta.source_authorities`.
- Separate boxscore authority for goal, assist, and point streaks from
  play-by-play authority for shot-on-goal and shot-attempt streaks.
- Render matching authority labels in player streak HTML.
- Cover loaded JSON and missing-cache HTML paths in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router streaks`
- `git diff --check`
