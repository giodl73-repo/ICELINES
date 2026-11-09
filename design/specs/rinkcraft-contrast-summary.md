# Readable matched numerical contrasts — CHG-138

WP-RINKCRAFT-14 / IF-RINKCRAFT-NUMBERS-001. Authority: Rinkcraft decision 0018 /
contrast protocol 0.1. Existing source/measurement/class/history contracts remain.

Add optional NumericalComparison `contrast`, schema
`icelines.rinkcraft-contrast-summary.v1`, containing fixed facet count, ranked
coverage, equal-rank count, explicit omissions, selection note and up to three
highlights. Each highlight holds facet label, unchanged ComparisonRow and limit.
Shared core chooses only the fixed six/four primary facets, requires ranked/raw/
percentile/shared peer coverage, excludes zero gaps, sorts by absolute full-
precision rank difference and breaks exact ties by facet order. Candidate pool
is shared with card construction, including explicit missing-primary omissions.

Require matching formula and scale in addition to existing same-source/group/
situation/unit/reference checks. No new worker/API operation: existing comparison
transports additive summary. Legacy comparison objects omit it safely. Source
JSON, values/ranks, documentary classes and historical selection are unchanged.

Native/browser show a compact signed/raw summary with coverage and limits before
the full fixed-order table. No scoring, gap pooling, effect-size/significance or
class/style claims. Correlated primary facets can appear together; this is reading
order, not independent skills. Existing method comparison remains separate.

Acceptance: fixed pool/ties/signs/zero/unranked/missing and definition guards;
all same-group latest-source WASM pairs; exact native/WASM contrasting pairs;
original measurements preserved; legacy/no-JS/offline/mobile/theme contrast.
Evidence: `docs/rinkcraft-contrast-validation-2026-11-09.md`.
