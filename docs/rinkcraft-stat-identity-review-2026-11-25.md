# Identity and feat differentiation: first pass — 2026-11-25

Question: does the compact selection hide stronger earned specialties behind
weaker combinations, and can the existing evidence make identities easier to
read? This is an exploratory presentation audit of the frozen 865-player current
cohort, not a new player classification or measurement validation.

The result adds two optional **Stat specialties** below the identity fields and
changes task selection to tier first. Contract:
[Task selection and stat specialties v2](../design/specs/rinkcraft-stat-specialties-v2.md).
The original class, documentary specialty and secondary class remain separate.

## Findings and resulting behavior

The previous novelty-first selector could choose a Bronze compound over a Gold
single-stat task. Beniers' Takeaways and Fox's Assist production are concrete
examples. V2 prioritizes earned tier, then complementary metric coverage, without
changing any qualification. The full library retains all earned/locked entries.

An initial projection also let rare SH credits lead some identity summaries and
repeated block metrics in adjacent highlights. The final authored policy excludes
SH outcome families from the identity strip (they remain in Role Coverage) and
requires disjoint literal metric keys across its two highlights. This does not
establish independence between correlated statistics.

| Player | Final stat specialties |
|---|---|
| Adam Fox | Assist production · Gold; PP assists · Gold |
| Matty Beniers | Takeaways · Gold; Goal production · Silver |
| Tye Kartye | Body contact · Silver; Goal production · Bronze |
| Alex Killorn | Draw penalties · Bronze |
| Adam Larsson | Block shots · Silver; Body contact · Silver |
| Braden Schneider | Body contact · Gold; Shot volume · Bronze |
| Juuse Saros | Busy saves · Silver; Starts & shutouts · Bronze |

These are qualified stat patterns in their own season/GP window, not technical
skill conclusions, new class assignments or fixed coaching responsibilities.
Blank strips remain blank rather than receiving invented awards.

## Same-cohort comparison

- 247 of 865 compact task selections change; distinct unordered task combinations
  increase from 172 to 198. No qualification or tier changes.
- 608 profiles have identity highlights: 494 have two and 114 have one. The other
  257 have no qualifying mapped highlight.
- Forward coverage: 395/514; defense: 196/273; goalies: 17/78.
- The goalie gap has two components: 34 below the existing feat GP minimum and
  27 eligible without a positive identity highlight. Goalies have only three
  positive active feat definitions in the current registry. Broader goalie
  specialties need a separate evidence/calibration pass.
- Curated class dossiers remain 26/865. Expanding them is still open; statistical
  highlights do not pretend to fill the other 839 dossiers.

This is a fixed-cohort software/presentation contrast, not proof that the new
selection predicts outcomes or improves coaching decisions. Named examples do
not constitute a salary-band study or an independent expert assessment.

## Producer-applied review lenses

Applied the Rinkcraft `.roles/ROLE.md` Methods Skeptic, Taxonomy Steward, Defense,
Goaltending and Consumer Boundary lenses. This is self-review, not an independent
panel or credentialed expert review.

| Lens | Finding and disposition |
|---|---|
| Methods | Preserve earned states, thresholds, input cohort and GP. Version the selection change; no new awards or validated trait claim. |
| Taxonomy | Keep Class/Specialty/Secondary and their dates intact. Label computed chips Stat specialties with their own season/GP. |
| Defense | Blocks and contact can be visible independently; no inference of retrieval, gap control, assignment compliance or isolated prevention. |
| Goaltending | Include positive goalie patterns and low-GP/no-award examples. Record the sparse vocabulary and eligibility gaps; do not pad. |
| Boundary | Reusable selection/rendering in IceLines. Research definitions unchanged in Rinkcraft; generated player artifacts remain ignored. |

## Validation and reproduction

- 257 browser tests and seven focused Rust tests passed, covering stronger singles,
  unchanged qualifications, family/metric diversity, caps, stable ordering,
  caution/SH exclusion, old v1 deserialization/rendering and temporal scope.
- Browser, pinned unoptimized WASM and native CLI/Askama builds passed. The final
  native rebuild includes the amended shared print CSS.
- Frozen before/after checks preserve all 865 complete profile payloads after
  removing only `achievements.task_repertoire`. This includes classes, sources,
  feats, scores, pressure, unit links, production and deployment data. The audit
  also checks tier ordering, metric novelty, earned references and header guards.
- All 865 repertoires are produced by the actual Rust/WASM selector. Twelve varied
  F/D/G cards pass exact header/task values, preservation of class and full feat
  library, actual WASM parity, old-v1 fallback and 390-pixel mobile layout.
- Twelve independently checked PDFs are single-page 612×792-point Letter sheets,
  retaining Attributes, Task Skills and earned specialty strips. A print overflow
  found during acceptance was corrected with scoped spacing/name-size adjustments;
  no content was clipped or hidden to force a pass. Beniers' screenshot inspected.

```powershell
node icelines-browser/e2e/rinkcraft-stat-identity.mjs
py target/stat-identity-study/verify.py
```

The frozen pre-change audit, source-profile projections, comparison, checks and
screenshots/PDFs remain ignored in `target/stat-identity-study`. The prepared
WASM projection helper there recalculates existing achievement repertoires; normal
feat evaluation/refresh uses the same selector automatically. The statistical
catalog remains `target/draw-strength-study/enriched/rinkcraft-stat-attributes.json`,
SHA-256 `22c98248e403016b7e9a4e34e140bbb03ff37e2b9d3beb4a480a9744c7500315`.

Preview: [Beniers](http://127.0.0.1:8780/?player=8482665) or
[Fox](http://127.0.0.1:8780/?player=8479323). No source acquisition, hosted
publication, push or merge is included in this pass.
