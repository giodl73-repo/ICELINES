# Player-specific pressure responses and task specialties

2026-11-18 owner clarification. Under Pressure and Task Skills are variable
repertoires, not a second fixed ability-score grid. The theme directions stay
unchanged. The earlier fictional numeric mockup is superseded in these boxes.

| Area | Meaning | Presentation |
|---|---|---|
| Attributes | Broad execution capabilities | Existing six 1–20 fields; NHL values still unrated |
| Under Pressure | Characteristic response when a named threat/condition arises | Short name and trigger→response description |
| Task Skills | Selected task proficiencies/specialties | Short name and task description, specific to the dossier |
| Signature Methods | Techniques used to execute tasks | Existing source-linked method descriptions |
| Playing Tendencies | Style and recurring approach | Existing descriptive traits; possible future style axes require evidence |
| Measured Record / dossier | Activity, context and task-performance evidence | Scoped rates/percentiles/formulas, not automatic proficiency scores |

Repertoire membership does not itself assert exceptional effectiveness or grant
a game bonus. These are curated selections, not exclusive class requirements or
an exhaustive list of what the player can do. A method can help execute several
tasks; shared source/action links do not create independent evidence credits.
No style dial or new numerical estimator is implemented in this change.

## Catalog contract

Optional `under_pressure` and `task_skills` arrays reuse the source-linked
`CharacterFeature` record in the Rust character catalog and selected sheet.
New entries require nonblank name/detail/evidence type and valid source/date
support. The existing publication-season cutoff applies independently to each
entry. Selected entries participate in the deduplicated source union.

Legacy catalogs omit both arrays; empty fields serialize away, preserving their
canonical catalog fingerprint. Existing traits/methods are never automatically
promoted into a repertoire. New nonempty fields change the catalog fingerprint,
as expected for newly authored interpretation. No original dossiers are silently
rewritten and no real-player specialties are invented by the renderer.

Render the first three pressure responses and five task specialties in authored
order, with the rest in a closed `+N more` disclosure. This is a space budget,
not ranking. Sources/gaps are inspectable separately. An absent repertoire says
not yet catalogued; it is not an inability or poor-performance claim. Additional
entries stay off the compact print sheet, which retains the primary selection.

## Examples and verification

Three neutral fictional design specimens illustrate differentiated F/D/G
repertoires, each with four responses and eight specialties. They remain visibly
fictional, including their separate illustrative attributes and records. Five
retained NHL fixtures keep their source/dates/identity/statistics; new repertoire
entries remain uncurated until independently authored with support.

Focused Rust checks exercise compatibility, source validation, date gating and
source deduplication. Browser checks exercise explicit-only membership, overflow
disclosure and the absence of task scores. Rebuilt WASM verifies the same source/
date/legacy behavior using neutral synthetic fixtures. Native Askama compilation,
theme/mobile checks and independent print checks complete local validation.
