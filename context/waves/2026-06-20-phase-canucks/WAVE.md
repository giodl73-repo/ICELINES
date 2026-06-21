# Phase Canucks

## Scope

Plan and execute the agent-evidence analytics workflow promotion gate. The wave
decides whether the WP-009 agent-evidence Web/API first-route evidence can
become a bounded prepared-cache agent evidence summary claim, or whether it
should remain only first-route evidence.

## Entry Posture

- Phase Predators promoted only postgame review and adjustments to bounded
  prepared-cache postgame report claims.
- The active surface matrix still keeps agent evidence as WP-009 first-route
  evidence.
- `/agents/evidence` and `/api/v1/agents/evidence` default to
  `agent_evidence:<season>:<type>` and render through
  `AnalyticsCacheConsumerView`.
- Existing L2 tests cover ready and unavailable route behavior.

## Goals

1. Inventory agent-evidence route evidence, product copy, and promotion
   blockers.
2. Decide whether agent evidence can be promoted to a bounded prepared-cache
   agent evidence summary claim.
3. Preserve explicit non-claims: no autonomous agent action, no recommendation
   authority, no workflow completion, no live recomputation, and no prediction
   certainty.
4. Preserve no-cache-creation-on-GET and prepared-cache read behavior.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Canucks goals | passed; see `CANUCKS-INVENTORY.md` and `pulses/pulse-01.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes use focused `icelines-web` analytics-cache tests.
- No live network dependency in tests.
