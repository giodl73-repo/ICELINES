# Phase Canadiens Shifts - Policy lock

Status: Closed

## Intent

Reaffirm historical shift data as a locked-off capability until ICELINES has a
verified source, bundle, fetch, fixture, and join policy. This keeps
strength-state and deployment work from accidentally treating boxscore shift
counts or legacy `ShiftProfile` projections as true per-shift evidence.

## Scope

- Keep `sync.capabilities.shifts=off` as the only valid mode.
- Expose the locked policy in `icelines config list` while preserving raw
  `config get sync.capabilities.shifts` output for scripts.
- Reword Tier 3 shift docs as a candidate-source policy, not an active feature.
- Name promotion requirements before any future shift capability unlock.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli --test system_tests config_list_labels_shifts_locked`
- `git diff --check`
