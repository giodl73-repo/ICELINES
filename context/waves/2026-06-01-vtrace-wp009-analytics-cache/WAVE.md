# VTRACE WP-009 Major Analytics Cache Foundation

## Scope

Accept the major analytics cache as the next ICELINES product-direction baseline
for coach/scout/report/card/line/goalie/practice/postgame decision-support
surfaces.

This wave started with a specification baseline and now has partial
implementation evidence for the core contract and strict tempdir-backed store.
It does not yet claim a dashboard/report surface, agent action, prediction
accuracy, or autonomous coaching authority.

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

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Specification baseline and DCR acceptance | docs_passed; implementation_target_spec_pending |
| 02 | Initial core schema/source/consumer contract | core_cache_contract_partial_passed |
| 03 | Strict fetch-layer cache store/read/invalidation fixtures | store_read_invalidation_partial_passed |
