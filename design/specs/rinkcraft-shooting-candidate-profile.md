# Seasonal shooting candidates on the shared profile

The shared browser renderer presents shooting alongside the seasonal skating
family in Distinctive Features. At most one label from each family appears.
The existing stat-feat lists and earned counts are unchanged; the separate
Seasonal Records group identifies the experimental observation families.

Shooting record opens the measurement readout for matched skaters, including
nonqualifiers, missing peak tracking and insufficient exposure. Goalies receive
no shooting family. Identity, season, regular-season scope, position, GP, TOI,
EDGE lineage and domain/peer checks bind the supplied Rust assessment before
rendering. Candidate classification is not recomputed from rounded UI values.

The modal shows tracked maximum shot speed, official SOG per 60, IceLines
positional midranks, measured peer counts and the exact P90 conjunctions. Conversion
is in a separate audit disclosure using the 25-SOG sensitivity setting; it never
awards this family or a finishing feat. Differing official and EDGE counts remain
visible together with their capture metadata and hashes. Shared SOG evidence with
existing shot-generation feats is explicitly acknowledged; it is not independent
corroboration or an additive simulation bonus.

The page distinguishes the existing stat-feat minimum from the research family's
own exposure rules. This changes wording, not either gate. Missing is not a penalty;
all six capability attributes stay unrated.

## Rust projection

The existing `rinkcraft_edge_candidate_profiles` command accepts an optional final
shooting-report path. Without it, the previous skating-only invocation still works.
The optional report must bind the same exposure inventory/catalog and its own
rules checksum, and contain exactly one 10-GP/100-minute/25-SOG audit scenario.
Each shooting record must match the original profile hash, actor, season, group,
GP and optional EDGE hash. The projector adds only `edge_shooting_candidate` to
matched skater frames while retaining the skating addition. RawValue preserves
original JSON number tokens. Both report digests are computed once outside the
per-player loop; output must be a new directory. Catalog and coverage are unchanged.

The independent verifier accepts the shooting report as an optional last argument
and strips both new fields before comparing every existing profile value. It also
checks every projected shooting record, report/rule hashes and baseline identity.

```text
cargo test -p icelines-sources --example rinkcraft_edge_candidate_profiles --locked
cargo run -p icelines-sources --example rinkcraft_edge_candidate_profiles --locked -- EXPOSURE_JSON SKATING_CANDIDATES_JSON BASE_DIRECTORY NEW_DIRECTORY SHOOTING_REPORT_JSON
python tools/verify_rinkcraft_candidate_profiles.py BASE_DIRECTORY NEW_DIRECTORY SKATING_CANDIDATES_JSON NEW_RECEIPT_JSON SHOOTING_REPORT_JSON
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node icelines-browser/e2e/rinkcraft-shooting-candidates.mjs
node icelines-browser/e2e/rinkcraft-skating-candidates.mjs
```

## Validation

- Rust token/null/duplicate preservation test passed for both fields.
- TypeScript compilation and 33 focused Node tests passed; three additional
  preview tests passed after the eligibility wording clarification.
- Independent audit preserved all 3,941 profile values, 446 skating labels and
  bound 3,565 shooting contexts containing 506 labels across four seasons.
- Shooting browser acceptance passed 30 contexts, all three branches, five source
  count differences, missing tracking, insufficient exposure, actual season changes,
  goalie exclusion, coexistence of both families, six mobile themes and three
  one-page Letter profiles. The existing 20-case skating suite passed as well.
- Desktop and mobile screenshots were inspected. pypdf independently confirmed
  all three Letter exports as one 612×792-point page each.

This is an exported-profile/shared-renderer feature. The native Askama/API and
live WASM computation have not acquired these fields. Production feat awards,
capability scores, documentary classes, finishing models and simulation inputs
remain unchanged. Dated outcomes, source snapshots and publication receipts are
retained in PUCK and the external research packet.

## Public preview version 5 — 2026-11-20 Pacific

URL: https://rinkcraft-nhl-preview.ivoryorchid34.chatgpt.site
Audience remained public; owner access was verified before source preparation.
Static source commit: `04804b5d04cc28c6c580515d861d8ee2b0defdf3`.
Saved version: `appgprj_6aff7eac4c1081919202e36a7730dc2a~appgver_4d06ea87a7188191892cffde7bc58633`.
Deployment: `appgdep_6b0058c40ef88191b36adbc134028f95` (succeeded).
The deployed site passed all 30 shooting contexts, count-difference displays,
coexisting families, actual season changes, goalie exclusion, six mobile themes
and three one-page Letter profiles with no page errors. No main merge, hosted
CI run, new Site or existing IceLines Pages deployment was performed.
