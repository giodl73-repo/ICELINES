# Phase Canadiens Setup Pulse 01 - Reset guard

## Result

Passed. `icelines setup` now has an explicit existing-config guard and covered
reset behavior.

## Evidence

- `icelines-cli/src/commands/setup.rs`
- `icelines-cli/src/cli.rs`
- `COMMANDS.md`
- `design/plans/2026-06-23-phaseCanadiensSetup-reset-guard.md`
- `context/waves/2026-06-23-phase-canadiens-setup/WAVE.md`
- `context/waves/2026-06-23-phase-canadiens-setup/CANADIENS-SETUP-INVENTORY.md`

## Closeout

Repeat setup runs leave `~/.icelines/config.toml` unchanged unless `--reset` is
passed. Reset updates the sync block while preserving non-sync config keys, and
`--dry-run` remains a write-free preview mode.
