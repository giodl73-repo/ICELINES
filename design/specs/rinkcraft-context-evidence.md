# Possession and environment evidence — CHG-140

WP-RINKCRAFT-16 / IF-RINKCRAFT-NUMBERS-001. Research authority: Rinkcraft decision
0020 and possession-context protocol 0.1. Existing source/attribute/contrast rules
remain. No raw-input or underlying statistical-definition change.

Add optional EvidenceCard `context_evidence`, schema
`icelines.rinkcraft-context-evidence.v1`, with title/note, declared/available/ranked
counts and fixed groups of ContextFacet {key,label,optional original Measurement,
explanation}. Skaters: two recorded possession events, two penalty rates and two
on-ice xG rates. Goalies: cumulative GSAx only. Context inherits parent season,
position group, situation, exposure, reference universe and source custody.

Shared Rust copies existing measures, retains absent slots, counts genuine zero
as available, separates unranked/exposure failure and provides direct limits.
No percentile inversion, composite, substitution or individual defensive grade.
Optional field defaults absent for legacy cards. Existing projection/worker routes
transport it; no new operation. Primary six/four facets, leading signals, contrast
headlines, history and class assignments are unchanged.

Native/browser display compact context groups with evidence expanders, plus a
separate comparison table. Subtractions reuse existing matched ComparisonRows;
unknown sides/ranks remain unknown. Comparison has no additional headline pool
or dumbbell axes. Defender duty/coverage and goalie mechanics remain unmeasured.

Acceptance: exact copies and all-position missing/zero/unranked cases; 25 actual
WASM cards, native/WASM parity, contextual rows without primary-pool changes,
legacy/no-JS/offline/mobile/theme checks and existing workflow regressions.
Evidence: `docs/rinkcraft-context-validation-2026-11-09.md`.
