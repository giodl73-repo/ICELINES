# Phase Canadiens Setup Auto Pulse 01 - Interactive gate

## Result

Passed. The first-run setup wizard now auto-runs for interactive terminal users
when no config exists, and remains silent for scripts and piped commands.

## Evidence

- `icelines-cli/src/main.rs`
- `icelines-cli/src/commands/setup.rs`
- `icelines-cli/src/cli.rs`
- `COMMANDS.md`
- `design/plans/2026-06-23-phaseCanadiensSetup-auto-prompt.md`
- `context/waves/2026-06-23-phase-canadiens-setup-auto/WAVE.md`
- `context/waves/2026-06-23-phase-canadiens-setup-auto/CANADIENS-SETUP-AUTO-INVENTORY.md`

## Closeout

The entrypoint now checks config presence, command kind, `--no-setup`, and
terminal status before launching setup. Headless callers can still run
`icelines setup --accept-defaults` explicitly.
