# Phase Canadiens Setup Auto Inventory

## Surfaces

| Surface | File | Result |
|---|---|---|
| CLI entrypoint | `icelines-cli/src/main.rs` | Runs setup before dispatch only when config is missing, stdin/stdout are terminals, the command is not `setup`, and `--no-setup` is absent. |
| Setup command | `icelines-cli/src/commands/setup.rs` | Exposes the existing config-file check for the entrypoint gate. |
| CLI help | `icelines-cli/src/cli.rs` | Documents interactive-only auto-setup and script bypass behavior. |
| Command reference | `COMMANDS.md` | Documents first-run auto-prompt boundaries and explicit headless setup command. |
| Plan index | `design/plans/INDEX.md` | Records this as a closed Canadiens setup slice under the active major-stats roadmap. |
| Wave index | `design/waves/PHASES.md` | Records repo-local execution evidence for the slice. |

## Non-Claims

- Non-interactive commands never auto-prompt.
- Existing config still blocks setup writes unless `--reset` is explicit.
- No installer, updater, seeded profile, or public API documentation is added.

## Validation

```powershell
cargo test -p icelines-cli auto_setup -- --nocapture
cargo test -p icelines-cli commands::setup::tests -- --nocapture
git diff --check
```
