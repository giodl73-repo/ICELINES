# WP-005 Pulse 03 - Shift Capability Lock

## Scope

Selected shift-level refusal boundary: per-shift parsing is not implemented, so
the shift capability must remain locked to `off` and attempts to enable it must
fail explicitly before users can request unsupported shift-level data.

## Change

- Recorded existing capability-lock evidence under `CHG-059` and
  `EVID-WP005-SHIFTS-LOCK-L1`.
- No implementation change was required; the existing config boundary already
  rejects `sync.capabilities.shifts = favorites|league` with literal refusal
  copy and keeps the typed capability helper disabled for all favorite states.

## Evidence

```powershell
cargo test -p icelines-cli --test foster_capability_matrix shifts -- --nocapture
cargo test -p icelines-cli --test system_tests l2_foster08_config_set_shifts_favorites_rejected -- --nocapture
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

The selected L1/L2 evidence covers both the in-process capability matrix and the
cross-process CLI config surface. `shifts=off` succeeds, `favorites` and
`league` are refused with spec-pinned copy, and the typed capability helper
blocks shifts for both favorite and non-favorite targets.

## Status

`passed_with_risk`.

Remaining WP-005 work includes fetch failure mocks, data command transcripts, and
partial-fetch resume/flag evidence.
