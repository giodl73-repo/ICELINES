# Phase Canadiens Diagnostics Pulse 01 - Data-status JSON

## Result

Passed. `icelines data-status --json` now emits the shared `DataStatusView`
envelope for scriptable freshness diagnostics.

## Evidence

- `icelines-cli/src/cli.rs`
- `icelines-cli/src/commands/data_status.rs`
- `icelines-cli/src/main.rs`
- `COMMANDS.md`
- `design/plans/2026-06-23-phaseCanadiensDiagnostics-data-status-json.md`
- `context/waves/2026-06-23-phase-canadiens-diagnostics/WAVE.md`
- `context/waves/2026-06-23-phase-canadiens-diagnostics/CANADIENS-DIAGNOSTICS-INVENTORY.md`

## Closeout

The JSON envelope includes rows, empty state, source state, warnings, and
authority notes while the existing table remains the default terminal output.
