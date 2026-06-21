# Phase Capitals Inventory

## Purpose

Confirm the Signals promotion starting point before implementation. Capitals is
not a blanket approval to cache, rank, or catalog every Signal.

## Current Signals Posture

| Area | Current evidence | Capitals posture |
|---|---|---|
| Player Signals surfaces | `PlayerSignalsView` backs CLI text/JSON, TUI player-card, Web player HTML/JSON, and Markdown export. | Reuse the existing ViewModel and methodology copy. Do not duplicate Signal meaning in renderers. |
| Roster discovery | `signals-roster` renders a team-scoped matrix with `signals-roster.v1` JSON and unavailable-state disclosure. | Treat as discovery evidence, not as a public ranking or cache publication. |
| Analytics cache | WP-009 cache consumers use `AnalyticsCacheConsumerView` with metric keys, source-state, invalidation, methodology, disclosures, non-claims, and supported consumer kinds. | Decide whether Signals can define cache metric keys and invalidation semantics, or keep them uncached. |
| Stat catalog and filters | Signals remain outside `StatId`, `--filter`, and catalog-driven sort paths. | Decide whether any Signal is stable and comparable enough for catalog/filter promotion. Default is no promotion. |
| Leaderboards | Rangers explicitly rejected cross-team Signal leaderboards for the roster discovery lane. | Decide whether any ranking is safe after product-copy review. Default is no public cross-team leaderboard. |
| Product claims | Existing copy says Signals are descriptive and unavailable evidence is not zero-value truth. | Preserve non-claim copy in every accepted surface. |

## Blockers Inherited From Hurricane/Rangers

- Signals cache/catalog/filter/leaderboard promotion needs product-copy and
  evidence review.
- Cache publication needs stable metric keys, source-state, invalidation, and
  methodology versioning.
- Signal ranking risks implying player-quality, prediction, deployment, injury,
  betting, or coaching recommendations.
- Missing Signal inputs must remain unavailable, never zero-filled.
- MoneyPuck deployment expansion, goalie GSAx/high-danger save percentage, and
  team confidence bands remain separate blocked source-contract work.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Cache eligibility gate.
3. Catalog/filter/leaderboard gate.
4. Promotion or durable deferral implementation.
5. Closeout and surface-matrix claim.
