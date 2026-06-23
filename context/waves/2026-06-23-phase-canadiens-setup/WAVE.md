# Phase Canadiens Setup

## Mission

Advance the Canadiens production-packaging roadmap by making the first-run setup
command safe for repeat invocations: existing local config is preserved unless
the operator passes the explicit `--reset` flag.

## Scope

- Implement the existing-config guard in `icelines setup`.
- Preserve the documented `--reset` recovery path.
- Keep `--dry-run` write-free.
- Document the reset boundary in command help/reference.

## Pulses

| Pulse | Status | Evidence |
|---|---|---|
| 01 | passed | Setup reset guard implemented, documented, and covered with focused tests. |

## Closeout

Phase Canadiens Setup is closed for the reset-guard slice. The broader
production-packaging roadmap still needs future installer/update UX, seeded demo
profiles, public API docs, and release diagnostics.
