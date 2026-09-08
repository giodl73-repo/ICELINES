---
skill: roles-check
topic: fantasy-today-performance
date: 2026-09-07
roles_used: 5
p1_count: 0
verdict: APPROVED
---

# Fantasy today performance — role review

## Artifact identification

- Type: Rust performance correction with no schema or scoring-policy change.
- Domain: saved fantasy-state assembly, exact lineup assignment, pickup-sequence search, and CLI latency.
- Reviewed artifacts: `fantasy_assistant.rs`, `fantasy_pickup_sequence.rs`, and `fantasy_today_service.rs`.

## Role selection

- FORGE: reviews ownership, allocation, panic, and crate-boundary behavior.
- PACE: checks the complexity diagnosis and measured latency claim.
- BENCH: checks semantic regression coverage and real-command evidence.
- EDGE: checks malformed rules, roster/candidate confusion, and cache scope.
- KEEL: checks that the shared engine remains authoritative across CLI and Puck.

## Review

| # | Role | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|---|
| 1 | FORGE | Resolved `PlayerView` values are scored directly instead of being converted to names and fuzzily resolved again. | P3 | fantasy-today score bootstrap | Keep fuzzy lookup at unresolved text boundaries only. |
| 2 | FORGE | Compact assignment states remove per-transition heap allocation while preserving owned output at the function boundary. | P3 | `maximum_weight_assignment` | Retain the fixed, validated active-slot bound. |
| 3 | FORGE | The daily-lineup cache is request-local and cannot become stale across repository swaps or later commands. | P3 | pickup-sequence builder | Do not promote it to process-global state. |
| 4 | PACE | The removed player-rate bootstrap was quadratic in the player count and allocated normalized strings inside the repeated fuzzy scan. | P3 | fantasy-today assembly | Document direct scoring as the linear resolved-view path. |
| 5 | PACE | The remaining pickup search is intentionally bounded by candidates, moves, beam width, and the weekly acquisition limit. | P3 | sequence search | Keep these bounds visible in output metadata. |
| 6 | PACE | The measured saved-league debug run improved from CPU-bound beyond 20 minutes to 44.91 seconds. | P3 | real CLI benchmark | Treat this as measured local evidence, not a universal SLA. |
| 7 | BENCH | Eight pickup-sequence L0 tests pass after memoization and assignment-state changes. | P3 | core regression suite | Preserve deterministic and multi-move fixtures. |
| 8 | BENCH | Eight fantasy-today service tests pass after direct score assembly and transition filtering. | P3 | fetch regression suite | Keep saved-state and offline paths covered. |
| 9 | BENCH | The exact Week 1 command produced valid `fantasy_today.v2` JSON and the Puck HTML/PDF/JSON brief regenerated successfully. | P3 | integration proof | Continue using this saved fixture as the realistic benchmark. |
| 10 | EDGE | Drop transitions now enumerate only initially modeled roster players, not free-agent candidates that legality checks would later reject. | P3 | week-plan transitions | Preserve the roster/candidate set distinction. |
| 11 | EDGE | Rules with more than 32 active slots return a validation error before compact assignment rather than panicking in bitmask construction. | P3 | rules validation | Retain the explicit boundary test. |
| 12 | EDGE | Cache keys contain both local date and the complete sorted roster, so the same roster on different days cannot reuse schedule state. | P3 | daily-lineup cache | Keep player keys canonical and roster order deterministic. |
| 13 | KEEL | All optimizations remain behind the shared core/fetch fantasy engine; no renderer gained independent lineup logic. | P3 | architecture | Keep CLI, web, and Puck consuming the same result contract. |
| 14 | KEEL | No public JSON schema or fantasy scoring formula changed. | P3 | surface contract | Avoid a schema bump for implementation-only performance work. |
| 15 | KEEL | Puck consumes the repaired CLI output through the existing one-shot JSON boundary and regenerated every report format. | P3 | cross-repository integration | Keep personal exports and rendered briefs private. |

## Synthesis

Roles reviewed: 5
P1 blockers: 0 | P2 issues: 0 | P3 notes: 15

Verdict: **APPROVED**

Top finding: already-resolved player views must never be sent back through full-pool fuzzy identity lookup.

Cross-role consensus: FORGE, PACE, BENCH, and KEEL agree that the fix preserves the shared exact model while removing redundant identity and lineup work.

## Amendments applied

1. Replaced quadratic name re-resolution with direct resolved-view scoring.
2. Memoized exact daily lineup evaluations, compacted assignment state, and restricted drops to rostered players.
3. Added explicit active-slot capacity validation, a boundary test, saved-fixture timing evidence, and the recurring pitfall record.
