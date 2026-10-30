---
skill: roles-check
topic: fantasy-market-pulse
date: 2026-09-08
roles_used: 9
p1_count: 0
verdict: APPROVED
---

# Fantasy market pulse — role review

## Artifact identification

- Type: versioned provider-evidence contract, pure comparison ViewModel, and CLI renderer.
- Domain: daily fantasy player-market observations and movement explanation.
- Reviewed artifact: `design/specs/fantasy-market-pulse.md` v0.1.

## Role selection

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, and SCOUT apply. GLASS and
BROADCAST are deferred because v0.1 has no TUI/Web visual surface; they become
required when those renderers are added.

## Review

| # | Role | Finding | Severity | Recommendation |
|---|---|---|---|---|
| 1 | HART | Provider demand is observation state, not season statistics. | P2 | Keep it outside `StatsRepository`. |
| 2 | HART | Player names are not unique identity. | P2 | Require a provider-scoped stable player ID. |
| 3 | HART | Provider positions must not rewrite canonical NHL position. | P2 | Treat them as snapshot evidence only. |
| 4 | KEEL | Core comparison must be the single computation path. | P2 | Make CLI a thin renderer over one ViewModel. |
| 5 | KEEL | A CLI-only launch needs an explicit parity debt. | P3 | Track TUI/Web as follow-up renderers. |
| 6 | KEEL | One-shot file comparison needs no long-lived cache. | P3 | Keep state out of CLI globals and TUI App. |
| 7 | TAPE | Missing metrics cannot mean zero. | P2 | Preserve `Option` through input and output. |
| 8 | TAPE | Cross-provider comparisons are invalid. | P2 | Reject unequal normalized provider labels. |
| 9 | TAPE | Duplicate names are expected across hockey history. | P2 | Join exclusively on provider player ID. |
| 10 | FORGE | Core must remain I/O-free. | P2 | Parse files in CLI; compare typed values in core. |
| 11 | FORGE | Invalid floats make ordering unsafe. | P2 | Validate finiteness before comparison. |
| 12 | FORGE | Public error behavior should remain explicit. | P3 | Return actionable validation strings at this pure boundary. |
| 13 | PACE | A synthetic price would hide unsupported weights. | P2 | Emit raw deltas and categorical direction only. |
| 14 | PACE | Rank direction is inverse to percentage direction. | P2 | Define lower XRank/ADP as improvement. |
| 15 | PACE | Sort order is not player-value rank. | P3 | Document deterministic report ordering. |
| 16 | BENCH | Every direction needs a hand-calculated fixture. | P2 | Test rising, falling, mixed, steady, and new. |
| 17 | BENCH | Identity behavior needs regression coverage. | P2 | Test duplicate names and duplicate IDs. |
| 18 | BENCH | Boundary validation is load-bearing. | P2 | Test percentages 0/100 and invalid values. |
| 19 | EDGE | A current player may have no prior row. | P2 | Emit `new_observation`, not a zero delta. |
| 20 | EDGE | A metric may disappear between exports. | P2 | Leave its delta absent and exclude it from direction. |
| 21 | EDGE | The prior file may actually be newer. | P2 | Reject reversed capture chronology. |
| 22 | WIRE | Input schema drift must fail visibly. | P2 | Use a versioned schema and deny unknown fields. |
| 23 | WIRE | stdin is ambiguous with two inputs. | P2 | Permit `-` for at most one of current/previous. |
| 24 | WIRE | v0.1 must not imply authenticated provider access. | P3 | State that it reads caller-supplied JSON only. |
| 25 | SCOUT | Demand movement is not hockey performance. | P2 | Avoid breakout or scoring claims in direction labels. |
| 26 | SCOUT | Adds can be schedule streaming rather than durable value. | P3 | Expose raw evidence for downstream context. |
| 27 | SCOUT | League availability differs from global demand. | P2 | Do not label globally rising players as locally available. |

## Synthesis

Roles reviewed: 9\
P1 blockers: 0 | P2 issues: 21 | P3 notes: 6

Verdict: **APPROVED**

The conditions are incorporated into the v0.1 contract and locked by core and
CLI tests: provider-scoped identity, optional evidence, chronology/provider
gates, transparent direction, stdin safety, and pure shared computation.

## Amendments applied

1. Split Market Pulse into its own versioned snapshot instead of extending the
   standings/status snapshot or `StatsRepository`.
2. Removed the proposed opaque alert score and documented raw evidence plus a
   categorical, reproducible direction rule.
3. Scoped v0.1 to core + CLI files/stdin, documented surface debt, and added
   explicit identity, range, chronology, missing-data, and schema tests.
