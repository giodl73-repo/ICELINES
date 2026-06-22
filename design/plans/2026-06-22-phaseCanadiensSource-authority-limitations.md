# Phase Canadiens Source - Scoring authority limitations

Status: Closed

## Intent

Prevent scoring authority metadata from being over-read by naming major domains
that cached official NHL play-by-play scoring reports do not cover.

## Scope

- Add `source_authority.limitations` to scoring JSON metadata.
- Mark shift time, expected goals, live fetch status, and uncached games as
  outside this authority contract.
- Keep the existing authority label and covered metric family intact.
- Document the limitations field in the command reference.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router rocket_game_scoring`
- `git diff --check`
