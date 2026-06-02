# VTRACE WP-009 Major Analytics Cache Foundation

## Scope

Accept the major analytics cache as the next ICELINES product-direction baseline
for coach/scout/report/card/line/goalie/practice/postgame decision-support
surfaces.

This wave started with a specification baseline and now has partial
implementation evidence for the core contract, strict tempdir-backed store,
first internal downstream consumer ViewModel fixture, first narrow
product-facing named-cache Web report, first coach-specific dashboard route,
first opponent-scout cache report route, first player evidence-card route, first
line-combination explorer route, and first goalie readiness/workload route. It
does not yet claim a full player-card workflow, full line-combination workflow,
full goalie workflow, practice, postgame, agent action, prediction accuracy, or
autonomous coaching authority.

## Entry posture

- ICELINES VTRACE WP-001 through WP-008 are closed, closed with risk, or
  explicitly dispositioned.
- The user selected the major analytics cache direction as the next hockey
  product foundation.
- `docs/vtrace/` remains the controlling specification baseline.

## Exit posture for specification baseline

- CONOPS, mission, requirements, change control, architecture, design,
  interfaces, validation, verification, trace, work packages, code rigor,
  integration, stage execution, and review records all name the cache target.
- VTRACE proof and diff checks pass for the docs/spec baseline.
- The baseline defines cache record/envelope expectations for version, scope,
  source window, provenance, freshness/staleness, quality/completeness, warnings,
  invalidation, methodology, disclosures, and consumer contract behavior.
- Downstream screens/reports are constrained to consume prepared cache evidence
  and must not recompute source-state, confidence, or methodology locally.
- Cache implementation rows remain partial until downstream hockey consumer
  fixtures and product-copy reviews pass.

## Future implementation sequence

1. Schema and storage slice: choose cache storage path, schema compatibility
   policy, metric families, and first fixture set.
2. Builder/read slice: prove local/snapshot-only builds, no-live reads, stale,
   partial, missing, unsupported, invalid-key, and schema-incompatible behavior.
3. Consumer slice: prove one dashboard/report/card-style envelope preserves
   prepared analytics, provenance, freshness, quality, warnings, and disclosures.
   - Status: done for the internal ViewModel fixture in pulse 04; shipped
     product surfaces are limited to the named-cache report in pulse 05, the
     coach dashboard and opponent scout routes in pulses 06-07, and the player
     evidence-card route in pulse 08, the line-combination explorer route in
     pulse 09, and the goalie readiness/workload route in pulse 10.
4. Product report slice: prove one Web HTML report and JSON twin can render an
   existing cache record without recomputing analytics or fetching live data.
   - Status: done for `/reports/analytics-cache` and
     `/api/v1/reports/analytics-cache` in pulse 05; broader hockey screens and
     reports      remain pending.
9. Goalie readiness/workload slice: prove one active-context goalie workload
   route can render a goalie consumer envelope without implying injury certainty
   or deployment advice.
   - Status: done for `/goalies/readiness` and `/api/v1/goalies/readiness` in
     pulse 10; practice/postgame surfaces remain pending.
5. Coach dashboard slice: prove one user-friendly coach route can default to the
   active-season cache key without forcing operators through the generic report
   query contract.
   - Status: done for `/coach/dashboard` and `/api/v1/coach/dashboard` in pulse
     06; broader coach-dashboard expansion remains pending.
6. Opponent scout slice: prove one active-context scout report route can render
   a scout consumer envelope without implying a full scouting suite.
   - Status: done for `/scout/opponent` and `/api/v1/scout/opponent` in pulse
     07; broader scout workflows remain pending.
7. Player evidence-card slice: prove one active-context player card route can
   render a player consumer envelope without implying a full player research or
   deployment workflow.
   - Status: done for `/player/evidence-card` and
     `/api/v1/player/evidence-card` in pulse 08; line/goalie/practice/postgame
     surfaces remain pending.
8. Line-combination explorer slice: prove one active-context line explorer route
   can render a line consumer envelope without implying deployment advice or
   line-chemistry causality.
   - Status: done for `/lines/explorer` and `/api/v1/lines/explorer` in pulse
     09; broader line-combination workflows and goalie/practice/postgame
     surfaces remain pending.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Specification baseline and DCR acceptance | docs_passed; implementation_target_spec_pending |
| 02 | Initial core schema/source/consumer contract | core_cache_contract_partial_passed |
| 03 | Strict fetch-layer cache store/read/invalidation fixtures | store_read_invalidation_partial_passed |
| 04 | Downstream consumer ViewModel fixture | consumer_viewmodel_partial_passed |
| 05 | Product-facing named-cache Web report and JSON twin | web_report_partial_passed |
| 06 | Coach dashboard active-cache Web route and JSON twin | coach_dashboard_partial_passed |
| 07 | Opponent scout active-cache Web route and JSON twin | opponent_scout_partial_passed |
| 08 | Player evidence-card active-cache Web route and JSON twin | player_evidence_card_partial_passed |
| 09 | Line-combination explorer active-cache Web route and JSON twin | line_combination_explorer_partial_passed |
| 10 | Goalie readiness/workload active-cache Web route and JSON twin | goalie_readiness_partial_passed |
