# Pulse 05 - Sticky command-mode follow-up

## Trigger

Post-dry-run user feedback reported confusing focus, weak discoverability, and
friction when issuing another command after a command-bar submission.

## Scope

- Keep the command grammar unchanged.
- Make repeated command use smooth when sticky command focus is active.
- Improve visible command-row guidance for the empty focused prompt.
- Update user-facing command-bar docs.
- Add focused regression coverage in the command-bar and persona harnesses.

## Role lenses

| Role | Check |
|---|---|
| glass | Empty focused command state must look intentional and explain exit keys. |
| bench | Repeated-command behavior needs direct regression tests. |
| wire | Do not change browser or CLI command contracts while fixing TUI focus. |
| forge | Keep the state change local to MDI command-bar handling. |

## Gates

- `cargo test -p icelines-cli --bin icelines l0_adams`
- `cargo test -p icelines-cli --bin icelines persona_jack_adams`
- `cargo fmt --check`

## Result

Done. `:` at an empty focused command bar is a harmless re-entry no-op, successful
commands show a command-row chaining hint, and `COMMANDS.md` documents that
Enter keeps command mode while `Tab`/`Esc` leaves it.
