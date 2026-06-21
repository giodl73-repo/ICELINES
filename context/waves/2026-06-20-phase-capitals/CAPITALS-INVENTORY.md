# Phase Capitals Inventory

## Purpose

Confirm the Signals promotion starting point before implementation. Capitals is
not a blanket approval to cache, rank, or catalog every Signal.

## Current Signals Posture

| Area | Current evidence | Capitals posture |
|---|---|---|
| Player Signals surfaces | `PlayerSignalsView` backs CLI text/JSON, TUI player-card, Web player HTML/JSON, and Markdown export. | Reuse the existing ViewModel and methodology copy. Do not duplicate Signal meaning in renderers. |
| Roster discovery | `signals-roster` renders a team-scoped matrix with `signals-roster.v1` JSON and unavailable-state disclosure. | Treat as discovery evidence, not as a public ranking or cache publication. |
| Analytics cache | WP-009 cache consumers use `AnalyticsCacheConsumerView` with metric keys, source-state, invalidation, methodology, disclosures, non-claims, and supported consumer kinds. | Pulse 02 keeps Signals uncached until accepted Signal cache metric keys, source-state, invalidation, and methodology versioning exist. |
| Stat catalog and filters | Signals remain outside `StatId`, `--filter`, and catalog-driven sort paths. | Pulse 03 keeps Signals outside catalog/filter paths until a bounded subset proves stable comparability and copy. |
| Leaderboards | Rangers explicitly rejected cross-team Signal leaderboards for the roster discovery lane. | Pulse 03 keeps public cross-team Signal ranking deferred; `signals-roster` remains a team-scoped inspection matrix. |
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
2. Cache eligibility gate. Result: passed as not eligible yet; Signals remain uncached.
3. Catalog/filter/leaderboard gate. Result: passed as not eligible yet; Signals remain outside `StatId`, filters, and public leaderboards.
4. Promotion or durable deferral implementation. Result: passed; docs/specs record durable no-promotion wording.
5. Closeout and surface-matrix claim.
