# IceCast Game-Prediction Edge — Implementation Plan

**Date:** 2026-07-30
**Status:** Implemented; v1 prospectively registered, v5 opponent-adjusted xG accepted retrospectively
**Parent:** Team Season Forecast

## Objective

Build and historically validate the full game-prediction stack described in
`design/specs/team-game-prediction-edge.md`, then use its enhanced probabilities
inside the existing all-32 season simulator.

## Delivery sequence

1. **Edge contract and overlay**
   - Add vintage, evidence, model, factor-attribution, validation, and sealed
     output contracts in core.
   - Blend Elo with the existing IceLines probability, apply bounded log-odds
     contributions, and preserve a simulation-ready enhanced forecast.
   - Prove missing evidence remains unavailable and late evidence is refused.

2. **Dated source package**
   - Build fetch-owned JSON packages for roster/availability, goalie state,
     trailing xG form, special teams, and matchup evidence.
   - Reuse existing official caches and MoneyPuck snapshots; retain source
     cutoffs and fingerprints.
   - Keep unsupported goalie xGA/high-danger and true shift claims blocked.

3. **Forecast vintages**
   - Expose preseason, game-morning, and confirmed-pregame builds.
   - Add a UI-neutral comparison card measuring probability movement between
     sealed vintages without treating later information as available earlier.
   - Run the 32-team season directly from the selected vintage document.

4. **Training and feature selection**
   - Fit a regularized logistic ensemble on typed feature vectors.
   - Run season-forward rolling origins, probability calibration, feature
     ablations, coefficient stability checks, and automatic pruning.
   - Score against IceLines, home-only, standings-only, and chronological Elo.

5. **Goalie and availability depth**
   - Reconstruct historical dressed lineups, scratches, injuries, transactions,
     and starter evidence as known at each forecast boundary.
   - Separate goalie quality from team defense and shrink small samples.
   - Grade the incremental value of roster and goalie information.

6. **xG and matchup depth**
   - Add score-adjusted trailing team xG features with source-completeness gates.
   - Add PP-vs-PK and bounded opponent-style interactions.
   - Remove or shrink features that fail rolling holdouts.

7. **CLI and reusable outputs**
   - Add `icecast edge`, `edge-train`, and selected-vintage season inputs.
   - Emit text, JSON, schemas, examples, and UI-neutral card projections from
     shared documents.

8. **Promotion and release**
   - Run at least six completed seasons where source coverage permits.
   - Require the full promotion gate; otherwise ship as an evaluation
     challenger with exact failed checks.
   - Regenerate the 2026-27 Rangers/Kraken and all-32 forecast and attribute the
     delta from the July baseline.

## Current historical result

### 2026-08-06 preseason refresh

- A fresh seeded 10,000-trial baseline retained all 1,344 games, all 32 teams,
  and exactly 84 games with a 42/42 home-road split for every team. The run
  emitted zero warnings and selected the authoritative July 29 official-roster
  snapshot with 32/32 verified clubs and player-value effects enabled.
- The frozen baseline projects NYR at 94.76 points (84/95/106 P10/P50/P90),
  49.72% playoffs, and 2.52% Stanley Cup; SEA is 89.54 points
  (78/90/101), 39.68% playoffs, and 1.16% Stanley Cup.
- Same-seed five-player internal-breakout branches move NYR to 99.26 points,
  69.13% playoffs, and 5.85% Stanley Cup, while SEA moves to 94.03 points,
  59.24% playoffs, and 3.30% Stanley Cup. These are authored ceiling branches,
  not unconditional forecasts.
- The retained UI-neutral baseline cards are
  `examples/season-simulation-card-alp-baseline-2026-27.json` and
  `examples/season-simulation-card-brv-baseline-2026-27.json`. Both preserve
  full-league run fingerprint `1afdc479e23aaa46f7bfdb4e84f600c1c479a5e42b9291118d9b0aa63bd6c8a0`;
  the richer development-variance showcase cards remain separate.

- 2019-20 through 2025-26: 8,510 official regular-season results.
- Five season-forward holdouts (6,560 scored games): pooled Brier gain 0.001028
  and log-loss gain 0.002113 versus chronological Elo; four holdouts improve.
- xG is the strongest incremental feature; opening-roster value is positive;
  the current special-teams term is small and must remain shrinkable.
- Confirmed reconstruction reaches 100% starter coverage and at least 98.8%
  availability coverage per season. The temperature-calibrated challenger has
  pooled Brier gain 0.001398, log-loss gain 0.002907, ECE 0.010126 versus Elo
  0.010369, and improves four of five holdouts.
- Every retrospective statistical gate passes. Production remains blocked by
  design until the sealed 2026-27 prospective registration can be scored after
  2027-04-11; retrospective tuning cannot grant itself authority.
- The fitted evaluation model is durable and directly reusable. `edge-card`
  owns same-model vintage deltas and factor explanations, while
  `season-simulate` consumes the selected enhanced forecast without
  recomputing game probabilities.
- A second seven-season replay added confirmed-starter trailing GSAx form and
  workload at roughly 99.5% side coverage. The unrestricted candidate fell to
  a 0.001050 pooled Brier gain and only three improved holdouts; a stronger
  sparsity rule still improved only three. The default therefore retains the
  preregistered `edge-core-v1` feature set, while the rejected
  `edge-core-v2-goalie-form` configuration remains reproducible for research.
- Replacement-adjusted dressed-lineup value (`edge-core-v3`) was retained as
  transparent evidence but trained to zero: pooled Brier was effectively
  unchanged and its ablation gain was 0.000000.
- Goalie-quality under schedule load (`edge-core-v4`) produced only 0.000003
  feature-specific Brier gain. The new 0.000050 candidate-feature gate rejects
  it despite the overall ensemble passing the older pooled checks.
- Strict-prior opponent-adjusted xG (`edge-core-v5`) is the first material new
  candidate: on the same 6,560 scored games it records Brier 0.238101 versus
  Elo 0.239969 (gain 0.001869), log-loss gain 0.003896, ECE 0.004542 versus
  0.010369, four of five improved holdouts, and 0.000781 feature-specific
  ablation gain. It passes leave-one-season and leave-one-team stability.
  It remains evaluation-only because the sealed 2026-27 registration binds
  `edge-core-v1`; retrospective success cannot rewrite that authority.
- Prediction cards now expose active-feature coverage, weight-adjusted
  evidence confidence, and an explicitly non-statistical evidence-stability
  range. Zero-weight research factors no longer inflate coverage.
- The released 2026-27 schedule contains 1,344 games and exactly 84 games for
  each of 32 teams. Authoritative opening-roster strengths now survive as
  typed forecast evidence and feed a reusable preseason package.
- A paired 10,000-trial v1 evaluation run moves NYR from 99.13 to 102.31
  expected points, 66.39% to 74.74% playoff probability, and 5.09% to 5.40%
  Cup probability. SEA moves from 89.30 to 86.43 points, 40.55% to 31.41%
  playoffs, and 1.26% to 0.93% Cup. These are evaluation-challenger deltas,
  not production promotion claims.

## Validation matrix

- L0: contract validation, shrinkage, contribution reconciliation, training,
  pruning, calibration, and promotion gates.
- L1: dated cache/source packages with no live network.
- L2: CLI build/train/replay and season bridge.
- Historical: rolling origins with same-date freezes and roster/goalie/xG
  ablations.
- Cross-platform: canonical model and forecast fingerprints.

## Completion

The source adapters, all three vintages, trained ensemble, rolling validation,
promotion decision, CLI/season bridge, documentation, and regenerated all-32
forecast are implemented and verified. Production authority remains gated only
by the prospectively registered 2026-27 holdout after 2027-04-11.
