# Phase Canadiens Shifts - Command doc sources

Status: Closed

## Intent

Align the generated command-reference source tables with the locked shift-data
policy so docs do not merely say shift bundles are parked; they name
`sync.capabilities.shifts=off` and the absence of a supported `fetch shifts`
recovery.

## Scope

- Update `src/data/commands.md`.
- Update `src/data/command-map.md`.
- Keep the change docs-only.

## Validation

- `git diff --check`
