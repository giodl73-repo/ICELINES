# Pulse 07: Lean CLI audit/fence

## Goal

Refresh the WP-007 lean CLI blocker evidence for Phase Rangers and add a
reproducible no-claim check without starting broad Cargo feature surgery.

## Scope

Added `scripts/rangers-lean-audit.ps1`, which checks:

- FLETCH remains a workspace git dependency consumed by `icelines-fetch`;
- SLICE remains a workspace git dependency consumed by `icelines-query`;
- FLETCH fetch command surfaces still exist;
- `icelines-query/src/slice_selectors.rs` still uses `slice_core`;
- no `cli` feature boundary exists in the root or CLI manifest.

## Result

Status: passed as target-not-met audit.

Rangers does not claim lean or standalone CLI support. The audit records the
current blockers and keeps implementation work for a future dependency surgery
wave.

## Validation

| Command | Result |
|---|---|
| `powershell -ExecutionPolicy Bypass -File scripts\rangers-lean-audit.ps1` | passed; target-not-met blockers detected |
| `powershell -ExecutionPolicy Bypass -File scripts\rangers-lean-audit.ps1 -Json` | passed; JSON blocker summary emitted |
| `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| `git diff --check` | passed |

## Non-claims

- No dependency was removed.
- No command surface was removed.
- No `cli` feature was added.
- No lean build support is claimed.
