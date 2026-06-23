# Phase Canadiens Setup Inventory

## Surfaces

| Surface | File | Result |
|---|---|---|
| Setup command | `icelines-cli/src/commands/setup.rs` | Existing `config.toml` now blocks a real setup write unless `--reset` is passed; reset still uses `save_sync()` so non-sync keys survive. |
| CLI help | `icelines-cli/src/cli.rs` | Setup help now states that existing config is left unchanged unless reset is requested. |
| Command reference | `COMMANDS.md` | Documents the existing-config guard, reset scope, and dry-run preview behavior. |
| Plan index | `design/plans/INDEX.md` | Records this as a closed Canadiens setup slice under the active major-stats roadmap. |
| Wave index | `design/waves/PHASES.md` | Records repo-local execution evidence for the slice. |

## Non-Claims

- The first-invocation auto-run hook remains separate from this slice.
- No installer, updater, seeded profile, or public API documentation is added.
- Reset only rewrites the sync section; it is not a full config deletion/reset.

## Validation

```powershell
cargo test -p icelines-cli commands::setup::tests -- --nocapture
git diff --check
```
