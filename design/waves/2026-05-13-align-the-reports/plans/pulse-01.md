# Pulse 01 - Report surface inventory and CLI catalog

## Goal

Give users one obvious command that answers "what report can I generate, and
which CLI door should I use?"

## Deliverables

- Add `icelines report list`.
- Support `icelines report list --json` as a machine-readable catalog.
- Document query/x/export/report roles in `COMMANDS.md`.
- Record the current surface map in `REPORT-SURFACE-INVENTORY.md`.

## Gates

- `cargo test -p icelines-cli --test system_tests l2_report_list`
- `cargo fmt --check`

## Result

Done. The catalog lists current report families and marks symmetric records as a
planned first-class family.
