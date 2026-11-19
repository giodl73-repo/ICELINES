# Whole-profile scope and experimental seasonal patterns

Owner correction: a profile represents one selected time window, such as a
season, career or current period. Individual achievements must not independently
choose unrelated time windows on the same primary sheet.

## Implemented local preview

Five retained2024–25 regular-season profiles gain an explicit Season scope control
and three experimental candidate patterns bound to the same actor/year/game type/
position group. The whole-profile header states the season. Career and Current
are listed as unavailable: the loaded fixtures are not verified complete careers
or dated2026–27 season-to-date records. No latest-cached fallback is presented as
current. Source accounts and biographical facts retain their dates and meanings.

Shooting Conversion and PP Production Rate appear in Task Skills; Whistle Balance
appears in Playing Tendencies. Earned experimental patterns share the existing
compact display allowance; other active feats and candidate states remain in
disclosures. A failed candidate threshold is not missing data or absent skill.
Goalie cards get the same scope control and retain their separate evidence battery;
no skater candidate is assigned to a goalie.

Disclosures expose opportunities, reference population, threshold, contributing
values/percentiles, source links and report/screen input hashes. Earlier candidate
seasons are bounded by the selected year. They do not silently replace the primary
scope, and2025–26 results do not appear as evidence on a2024–25 card. Primary
candidate ranks use that selected season's eligible peers, not frozen development
norms; the retrospective longitudinal study remains separate.

## Ownership and replay

Rust example `rinkcraft_scoped_preview` validates the exact selected window and
produces only additive scope/pattern patches. Existing numeric profile payloads
do not round-trip through Rust serialization: `project_rinkcraft_scope.mjs` merges
the two optional fields and checks exact preservation of every prior field. This
avoids otherwise observable floating-point parse/serialize changes. TypeScript
renders supplied states and percentiles; it does not pool career totals or assign
ability grades. Native/WASM data-loader and backend-scope support are not part of
this increment. The shared browser card can consume these optional fields.

```text
cargo test -p icelines-sources --example rinkcraft_scoped_preview --locked
cargo run -p icelines-sources --example rinkcraft_scoped_preview --locked -- BASE_FIXTURES LONGITUDINAL_INPUT_MANIFEST OUTPUT_PATCHES
node tools/project_rinkcraft_scope.mjs BASE_FIXTURES OUTPUT_PATCHES OUTPUT_FIXTURES
node tools/prepare_rinkcraft_theme_studio.mjs OUTPUT_FIXTURES SITE_DIRECTORY
node icelines-browser/e2e/rinkcraft-scoped-profile.mjs
```

## Validation and boundaries

Two Rust tests, TypeScript compilation and20 focused Node tests pass. Five-profile
browser checks cover the global scope, disabled career/current choices, candidate
placement/opportunities, six unrated attributes each, bounded history, no page
errors and390px layout. Independent checks verify48 candidate value/rank scalars,
four rejected actor/year/group/future-history patch cases, exact prior-field
preservation and five one-page612x792 Letter exports with scope/record/footer.
All six themes pass48 desktop/24 mobile cases and six fictional print exports.
Fox screenshot inspected. No hosted or full native/WASM runtime claim.

Career/current aggregation and comparison policy remain unimplemented. Career
rates must pool compatible numerators/denominators; seasonal percentiles must not
be averaged. Current requires an explicit operational window and data-as-of date.
Retired/active/era/career-stage reference comparability needs its own protocol.
The existing34-rule live registry, documentary class logic,1–20 capabilities and
simulator are unchanged. These three remain visibly experimental candidates.
