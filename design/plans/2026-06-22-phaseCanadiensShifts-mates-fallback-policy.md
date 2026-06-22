# Phase Canadiens Shifts - Mates fallback policy

Status: Closed

## Intent

Make `icelines mates` honest about the locked shift-data policy when no legacy
precomputed `ShiftProfile` exists. The command should guide users to the roster
fallback, not to an unsupported `fetch shifts` workflow.

## Scope

- Print `sync.capabilities.shifts=off` and the missing source/bundle/fetch
  policy in the table fallback path.
- Prove the fallback does not mention `fetch shifts` as a recovery.
- Update COMMANDS and player-analysis docs to separate current roster fallback
  behavior from future Tier 3 shift-backed linemates.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --test system_tests mates_fallback_reports_shift_policy_lock`
- `git diff --check`
