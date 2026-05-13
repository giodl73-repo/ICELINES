---
wave: norris-extracts-the-state
pulse: 01
date: 2026-05-13
status: backfilled
kind: provenance
---

# Pulse 01 - Provenance and Shipped Slices

## Evidence Commits

| Commit | Evidence |
|---|---|
| `1d14ffd2` | spec + plan |
| `4dd4c40e` | apply 8-role review |
| `24e8a5be` | extract `QueriesState` |
| `543eae23` | extract `ScheduleScreenState` |
| `4d6ea03a` | extract `TransactionsState` |
| `b6b88ba2` | extract Goalies/Playoffs/Tonight states |
| `72014687` | extract DatePicker and GroupPicker states |
| `a32f36a8` | v0.21.0 Norris release |

## Carry Forward

- New TUI work should add state to screen-specific structs, not `App`.
