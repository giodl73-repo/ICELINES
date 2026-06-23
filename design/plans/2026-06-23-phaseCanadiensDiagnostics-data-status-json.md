# Phase Canadiens Diagnostics - Data status JSON

## Status

Closed - 2026-06-23

## Goal

Make release-data freshness diagnostics scriptable from the CLI by exposing the
same `DataStatusView` contract already used by Web admin JSON.

## Scope

- Add `icelines data-status --json`.
- Serialize the shared `DataStatusView`, including rows, empty state, warnings,
  source state, and authority notes.
- Keep the existing text table as the default output.
- Document the JSON diagnostic path in the command reference.
- Cover the JSON envelope with a focused CLI command test.

## Non-Claims

- This does not add a new data health command.
- This does not change manifest collection, freshness TTLs, or source authority
  semantics.
- This does not add installer/update UX or seeded demo profiles.

## Validation

```powershell
cargo test -p icelines-cli data_status -- --nocapture
git diff --check
```
