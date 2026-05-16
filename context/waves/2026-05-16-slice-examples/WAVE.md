# Wave: SLICE Examples

## Goal

Show the safe layer for SLICE inside ICELINES: simple prepared player bio/stat
rows only.

## Pulse table

| Pulse | Title | Status | Outcome |
|------:|-------|--------|---------|
| 01 | Simple player row selectors | done | Added dev-only SLICE tests for simple bio/stat row predicates. |

## Success criteria

- ICELINES query UX and IR remain authoritative.
- SLICE is dev/example-only.
- Examples cover simple bio and stat fields only.
- `cargo test -p icelines-query --test slice_simple_selector` passes.
