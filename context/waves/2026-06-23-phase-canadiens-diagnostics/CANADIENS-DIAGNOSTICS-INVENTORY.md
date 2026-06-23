# Phase Canadiens Diagnostics Inventory

## Surfaces

| Surface | File | Result |
|---|---|---|
| Data-status CLI | `icelines-cli/src/cli.rs` | Adds `--json` to `icelines data-status`. |
| Data-status command | `icelines-cli/src/commands/data_status.rs` | Serializes the shared `DataStatusView` before text rendering when JSON is requested. |
| CLI dispatch | `icelines-cli/src/main.rs` | Passes the JSON flag into the data-status command. |
| Command reference | `COMMANDS.md` | Documents machine-readable freshness diagnostics. |
| Plan index | `design/plans/INDEX.md` | Records this as a closed Canadiens diagnostics slice under the active major-stats roadmap. |
| Wave index | `design/waves/PHASES.md` | Records repo-local execution evidence for the slice. |

## Non-Claims

- Text output remains the default.
- JSON uses the existing view contract; no new health schema is introduced.
- Freshness TTLs, manifest collection, and authority notes are unchanged.
- No installer, updater, or seeded demo profile is added.

## Validation

```powershell
cargo test -p icelines-cli data_status -- --nocapture
git diff --check
```
