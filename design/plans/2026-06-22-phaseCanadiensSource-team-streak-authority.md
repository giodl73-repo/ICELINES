# Phase Canadiens Source - Team streak authority

Status: Closed

## Intent

Bring team streak leaders onto the same explicit source authority contract as
player streaks.

## Scope

- Add team streak JSON `meta.source_authorities`.
- Separate boxscore authority for goal, assist, and point streak leaders from
  play-by-play authority for shot-on-goal and shot-attempt streak leaders.
- Render matching authority labels in team streak HTML.
- Cover loaded JSON and missing-cache HTML paths in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router streaks`
- `git diff --check`
