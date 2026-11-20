# Playing-method comparison — CHG-133

Add optional `style` to `NumericalComparison`, carrying projection schema
`icelines.rinkcraft-style-comparison.v1`, rows and shared gaps. Existing numeric
rows, measurement/catalog schemas and input protocols remain unchanged. Both
legacy styles absent => None. One absent => available rows retain one-sided
values without subtraction. Both present but no measured fields => shared gaps.

Core owns ordered union of measured keys and matched/left_only/right_only/
incompatible states. Match same known definition/unit, season and position group.
Existing catalog validation and numeric comparison position/situation/reference
guards precede style comparison. Shot/contact fields require their exact 5on5
scope and identical sets of source URLs/hashes. Tracking fields require expected
units, declared all-strength scope and pinned per-player/per-season NHL EDGE
regular-season URL. Player-specific tracking file hashes naturally differ.
Unknown definitions, unit or source/scope mismatch => no difference. Known matched
values => full-precision left minus right. Missing is never zero or low ability.

Native existing comparison API and stateless WASM numerical comparison expose the
same additive projection. Native and browser show individual values/scopes, raw
difference, evidence coverage and shared tactical gaps. Fraction differences use
percentage points; other units remain explicit. No style percentile, overall score,
class distance, opportunity adjustment or equivalence-of-samples claim. Legacy
catalogs retain the old view. Goalies keep saving/workload above and method gaps
below, with no skater methods assigned.

No new sources/data ingestion or mathematical metric definitions. Prepared input
remains the 25-player, 47-style-readout, 345-numerical-measurement artifact. Existing
offline shell pipeline authenticates UI/WASM changes. Acceptance: sign reversal,
missing-side and source/unit/scope mismatch, legacy, goalie gaps, actual API/WASM
parity, no-JS/offline/mobile and prior period/class/numeric comparison regression.
See `docs/rinkcraft-style-comparison-validation-2026-11-08.md`.
