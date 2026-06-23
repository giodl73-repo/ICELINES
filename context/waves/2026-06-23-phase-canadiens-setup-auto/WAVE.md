# Phase Canadiens Setup Auto

## Mission

Advance the Canadiens production-packaging roadmap by making first-run setup
real for interactive packaged users without risking blocked scripts or piped
commands.

## Scope

- Add the auto-setup dispatch gate.
- Restrict auto-prompting to terminal stdin/stdout.
- Respect `--no-setup` and skip recursive setup command handling.
- Document the interactive-only behavior.

## Pulses

| Pulse | Status | Evidence |
|---|---|---|
| 01 | passed | Interactive-only auto-setup gate implemented, documented, and tested. |

## Closeout

Phase Canadiens Setup Auto is closed for the first-run prompt slice. Installer
UX, update UX, seeded demo profiles, public API docs, and broader diagnostics
remain separate production-packaging work.
