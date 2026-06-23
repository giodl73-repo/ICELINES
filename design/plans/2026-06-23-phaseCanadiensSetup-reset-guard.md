# Phase Canadiens Setup - Reset guard

## Status

Closed - 2026-06-23

## Goal

Make the first-run setup command honor its existing-config boundary so
packaged/scripted installs do not silently rewrite sync settings unless the
operator explicitly requests a reset.

## Scope

- Keep `icelines setup` from writing over an existing `config.toml` by default.
- Keep `icelines setup --reset` as the explicit path for re-running setup.
- Preserve non-sync config keys when reset rewrites the sync block.
- Keep `--dry-run` as a write-free preview mode.
- Cover the boundary with focused setup command tests.

## Non-Claims

- This does not add the auto-run first-invocation setup hook.
- This does not add an installer, updater, or seeded demo profile.
- This does not change the sync capability defaults.

## Validation

```powershell
cargo test -p icelines-cli commands::setup::tests -- --nocapture
git diff --check
```
