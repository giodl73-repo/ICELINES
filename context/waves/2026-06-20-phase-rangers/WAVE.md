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
| 02 | NYR workflow proof over existing offline surfaces | passed; see `scripts/rangers-workflow.ps1` and `pulses/pulse-02.md` |
| 03 | Signals discovery design gate | passed; see `SIGNALS-DISCOVERY-GATE.md` and `pulses/pulse-03.md` |
| 04 | Signals roster matrix CLI surface | passed; see `signals-roster` and `pulses/pulse-04.md` |
| 05 | Evidence-envelope bridge decision | passed; see `EVIDENCE-BRIDGE.md` and `pulses/pulse-05.md` |
| 06 | Layout persistence hardening proof | passed; see `scripts/rangers-layout-proof.ps1` and `pulses/pulse-06.md` |
| 07 | Lean CLI audit/fence | passed as target-not-met audit; see `scripts/rangers-lean-audit.ps1`, `LEAN-AUDIT.md`, and `pulses/pulse-07.md` |

## Validation posture

- Planning/doc-only edits use VTRACE proof check and `git diff --check`.
- Implementation pulses add focused Rust tests for changed surfaces.
- Tests stay offline and fixture-backed.
