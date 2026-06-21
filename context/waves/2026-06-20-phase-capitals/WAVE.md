# Phase Capitals

## Scope

Plan and execute the Signals cache promotion gate left by Phase Hurricane and
Phase Rangers. The wave decides whether Signals can safely enter analytics
cache, `StatId`, filters, or leaderboards, or whether they remain durable
`PlayerSignalsView` inspection surfaces.

## Entry Posture

- Phase Hurricane shipped Signals to CLI, TUI player-card, Web player, and
  Markdown export surfaces.
- Phase Rangers shipped `signals-roster` as a team-scoped discovery matrix.
- Signals remain outside `StatId`, `--filter`, public cross-team leaderboards,
  and analytics cache.
- Rangers pulse 05 kept `signals-roster` outside WP-009 until a separate Signals
  cache-promotion gate exists.
- Existing non-claim copy says Signals are descriptive and must not be treated as
  prediction, betting, injury, deployment, player-grade, or autonomous coaching
  recommendations.

## Goals

1. Inventory current Signals surfaces, evidence shapes, and inherited blockers.
2. Decide whether Signals are eligible for WP-009 analytics cache metric keys.
3. Decide whether any Signals are eligible for `StatId`, filters, or leaderboard
   promotion.
4. Preserve unavailable-state and non-claim copy in any promoted surface.
5. Close the phase with exact surface-matrix wording.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Capitals goals | passed; see `CAPITALS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Decide Signals analytics-cache eligibility | passed as not eligible yet; see `CACHE-ELIGIBILITY.md` and `pulses/pulse-02.md` |
| 03 | Decide Signals catalog/filter/leaderboard eligibility | passed as not eligible yet; see `CATALOG-LEADERBOARD-GATE.md` and `pulses/pulse-03.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Cache/catalog/surface changes require focused Rust tests.
- Tests stay offline and fixture-backed.
