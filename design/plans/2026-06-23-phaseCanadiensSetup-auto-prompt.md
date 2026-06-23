# Phase Canadiens Setup - Auto prompt

## Status

Closed - 2026-06-23

## Goal

Make the documented first-run setup wizard actually run for packaged
interactive users while preserving script safety.

## Scope

- Run setup automatically before command dispatch when no config file exists.
- Gate the auto-run to interactive terminal stdin and stdout.
- Respect top-level `--no-setup`.
- Skip the auto-run for the `setup` command itself.
- Keep non-interactive invocations on default config behavior without blocking.
- Cover the gate with focused tests.

## Non-Claims

- This does not add an installer, updater, or seeded demo profile.
- This does not change setup questions or sync defaults.
- This does not auto-prompt headless or piped commands.

## Validation

```powershell
cargo test -p icelines-cli auto_setup -- --nocapture
cargo test -p icelines-cli commands::setup::tests -- --nocapture
git diff --check
```
