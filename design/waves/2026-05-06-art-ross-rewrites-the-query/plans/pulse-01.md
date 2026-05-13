---
wave: art-ross-rewrites-the-query
pulse: 01
date: 2026-05-13
status: backfilled
kind: provenance
---

# Pulse 01 - Provenance and Shipped Slices

## Evidence Commits

| Commit | Evidence |
|---|---|
| `e6ae186c` | spec + plan after 8-role review |
| `5d55ef82` | IR + planner + executor skeleton |
| `d6bafc6f` | grammar expansion (`<`, `>`, `!=`, `IN`, `BETWEEN`, `LIKE`) |
| `d6711237` | sliding-window atom grammar and aggregator |
| `ad0185b0` | provider and CLI killer-query wiring |
| `1d58bae0` | historical `EVER` + age slicing |
| `e58857e8` | cross-league career atoms |
| `1c240d80` | v0.20.0 query rewrite release |
| `699d1e17` | web JSON API new-grammar parity |
| `6e769790` | cross-surface result parity |
| `417d864f` | TUI free-form filter overlay |
| `001df809` | `query career --filter` extension |

## Carry Forward

- Scenario corpus classification belongs to `backcheck-the-phases` pulse 05.
- New command surfaces should route through the shared query grammar.
