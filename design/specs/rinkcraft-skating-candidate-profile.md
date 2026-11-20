# Skating candidate profile presentation

The compact shared browser renderer can display one seasonal EDGE skating research
candidate in Distinctive Features. The same scoped assessment opens from Skating
Detail for matched skaters who do not qualify or lack sufficient exposure. Goalies
and unmatched source contexts receive no skating assessment control.

Classification remains in Rust. The TypeScript layer binds the payload to the
selected actor, season, regular-season type, position group, GP, TOI and EDGE hash.
It validates measurement domains, unique metric keys, minimum measured peers,
exposure state and the research-status marker. It displays the supplied family
result; it does not classify from rounded text or produce a capability score.

The modal shows the player and selected season, raw peak speed, bursts per 60,
IceLines positional midranks and each metric's measured peer count. It includes
the P90 family rules, rate denominator arithmetic, exposure gate and hashes.
Provider percentiles remain separately labeled in the original EDGE rulebook.
Missing and insufficient states remain distinct from a negative feat.

## Projection and replay

`rinkcraft_edge_candidate_profiles` requires the verified exposure report, its
checksummed candidate report, a frozen EDGE-enriched baseline directory and a new
output directory. It verifies report/catalog/input-profile hash linkage and actor
scope, then adds only `numerical_evidence.edge_skating_candidate` to matched
skater profiles. Original JSON tokens are retained through RawValue. Goalies and
unmatched profiles are copied byte-for-byte. Catalog and EDGE coverage are copied
unchanged. The projection uses the authored 10-GP/100-minute research setting.
No production registry, class inference or simulation parameters are modified.

```text
cargo test -p icelines-sources --example rinkcraft_edge_candidate_profiles --locked
cargo run -p icelines-sources --example rinkcraft_edge_candidate_profiles --locked -- EXPOSURE_JSON CANDIDATE_JSON BASE_DIRECTORY NEW_DIRECTORY
python tools/verify_rinkcraft_candidate_profiles.py BASE_DIRECTORY NEW_DIRECTORY CANDIDATE_JSON NEW_VERIFICATION_JSON
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node icelines-browser/e2e/rinkcraft-skating-candidates.mjs
```

The first working export was stopped after observing repeated hashing of the
unchanged report; that constant hash is now computed once. The completed export
is the second output directory; the partial first directory is not a deliverable.

## Validation

- One Rust preservation test passed (long numeric tokens, null, duplicate guard).
- TypeScript compilation and 30 focused Node tests passed, including three new
  candidate binding/unknown/formula tests.
- Independent projection audit preserved all 3,941 existing profiles, bound 3,561
  research contexts and counted 446 candidate labels across four seasons.
- Browser acceptance passed 20 contexts, actual season-selector changes, all
  three candidate branches, insufficient exposure, goalie exclusion, six mobile
  themes, no page errors and one-page Letter printing.
- Desktop/mobile screenshots were inspected; a spacing issue was corrected and
  browser acceptance rerun. pypdf independently verified one 612×792-point page
  containing the candidate label.

Native Askama/API output has not acquired these new research fields. The shared
browser renderer can consume them when supplied by an exported profile. This is
not a full native or live WASM-engine candidate release. Existing player values,
ability grades and source observations remain unchanged.

## Public release version 4 — 2026-11-19 Pacific

URL: https://rinkcraft-nhl-preview.ivoryorchid34.chatgpt.site
Audience remained public; owner access was verified before publication.
Static source commit: `cb2ab8423116d5f383f02e8c8fd8ffb984143fed`.
Saved version: `appgprj_6aff7eac4c1081919202e36a7730dc2a~appgver_09c2186c6e78819191a294c13802117d`.
Deployment: `appgdep_6affd26c2adc81918682b5a3903085cf` (succeeded).
The deployed site passed the same 20-context acceptance, season control, six
mobile themes, goalie exclusion and Letter checks with no page errors.
A temporary source credential expired during initial synchronization; refreshing
that credential and retrying the same project completed publication. No new Site,
main merge, hosted CI or existing IceLines Pages publication was performed.
