# Phase Canadiens Shifts - TUI deployment alias

Status: Closed

## Intent

Stop the TUI command bar from treating `deployment <player>` as a hidden alias
for `mates <player>`. With shifts locked off, deployment should remain explicit
watch/deployment workflow language, not a shortcut to roster-fallback linemates.

## Scope

- Keep `mates` and `linemates` as aliases for the roster-fallback linemate
  handoff.
- Remove `deployment` as a `mates` alias.
- Update command help copy to say roster fallback and locked shifts.
- Add parser coverage for accepted `linemates` and rejected `deployment`.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli tui::command::tests::l0_profile_parse_player_hub_handoffs`
- `git diff --check`
