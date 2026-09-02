# Phase Rangers

## Scope

Plan and execute the post-Hurricane organization round for ICELINES. This wave
turns shipped analytics into discoverable, evidence-preserving, persistent, and
leaner workflows without making new unverified source claims.

## Entry posture

- Phase Hurricane is wrapped as of 2026-06-20.
- Signals have CLI, TUI player-card, Web player, and Markdown export surfaces.
- MoneyPuck deployment expansion, goalie GSAx/high-danger SV%, team confidence,
  and broader Signals cache/catalog/filter/leaderboard work remain gated by new
  evidence or review.
- REQ-WB-003, REQ-DEP-001, REQ-LEAN-001, and broad REQ-CACHE targets remain open
  targets in VTRACE.

## Goals

1. Signals discovery lane with product-copy and evidence review before any broad
   catalog/filter/leaderboard promotion.
2. Reuse or explicitly disposition the existing WP-009 evidence-card/cache
   envelope for Rangers surfaces.
3. Harden or use the existing WP-002 named workbench layout persistence path.
4. Lean offline CLI feature/dependency audit and reproducible check.
5. NYR workflow proof across existing team, roster/depth, player Signals, goalie
   workload, and export/report surfaces.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Rangers goals | passed; see `RANGERS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | NYR workflow proof over existing offline surfaces | passed; see `scripts/team-workflow.ps1` and `pulses/pulse-02.md` |
| 03 | Signals discovery design gate | passed; see `SIGNALS-DISCOVERY-GATE.md` and `pulses/pulse-03.md` |
| 04 | Signals roster matrix CLI surface | passed; see `signals-roster` and `pulses/pulse-04.md` |
| 05 | Evidence-envelope bridge decision | passed; see `EVIDENCE-BRIDGE.md` and `pulses/pulse-05.md` |
| 06 | Layout persistence hardening proof | passed; see `scripts/layout-proof.ps1` and `pulses/pulse-06.md` |
| 07 | Lean CLI audit/fence | passed as target-not-met audit; see `scripts/lean-audit.ps1`, `LEAN-AUDIT.md`, and `pulses/pulse-07.md` |

## Validation posture

- Planning/doc-only edits use VTRACE proof check and `git diff --check`.
- Implementation pulses add focused Rust tests for changed surfaces.
- Tests stay offline and fixture-backed.

## Phase Rangers closeout (2026-06-20)

Phase Rangers is wrapped. The phase delivered the planned post-Hurricane
organization round: a repeatable NYR workflow proof, a Signals discovery gate,
a team-scoped Signals roster matrix, an explicit evidence-bridge decision, an
isolated named-layout persistence proof, and a lean CLI dependency audit that
records the current target-not-met posture without claiming lean support.

No active Rangers implementation pulse remains. Future work requires new waves:

- Signals cache, catalog, filter, leaderboard, or stable `StatId` promotion
  requires a separate Signals cache-promotion gate.
- Lean or standalone CLI support requires dependency surgery and a passing
  feature/build boundary.
- Live browser or interactive TUI proof remains outside Rangers.
- MoneyPuck deployment columns, GSAx/high-danger save percentage, and team
  confidence bands remain blocked by the evidence/source contracts recorded
  during Phase Hurricane.
