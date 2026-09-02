# Pulse 03: Signals discovery design gate

## Goal

Decide whether Phase Rangers can implement a Signals discovery lane without
violating the Phase Hurricane/WP-010 promotion rule.

## Result

Status: passed.

The accepted next implementation shape is a roster discovery matrix. It may show
Signals evidence across one requested roster, but it must not behave as a public
Signal leaderboard, stable stat-catalog promotion, filter key, or analytics-cache
metric family.

## Evidence

| Evidence | Result |
|---|---|
| `design/specs/icelines-signals.md` inspection | Promotion rule requires copy review, source/completeness disclosure, parity evidence when applicable, cache methodology when cached, and explicit non-claims. |
| `context/waves/2026-06-20-phase-rangers/SIGNALS-DISCOVERY-GATE.md` | Records the approved roster-matrix shape, required fields, required copy, required tests, and explicit non-promotions. |
| `scripts/team-workflow.ps1` from pulse 02 | Provides the NYR product thread for the first implementation target. |

## Next pulse

Implement the smallest offline CLI roster discovery surface for NYR-compatible
team workflows, preferably:

```powershell
icelines signals roster --team NYR
icelines signals roster --team NYR --json
```

If preserving the existing positional `signals "<player>"` command requires a
separate additive command name, use that instead. Do not break the existing
player command.
