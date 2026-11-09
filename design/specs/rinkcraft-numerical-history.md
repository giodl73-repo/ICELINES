# Annual Rinkcraft numerical evidence — CHG-134

Owner asks to continue numerical differentiation. Add an independent optional
history archive, preserving the existing numerical/style snapshot and class
catalog. Schema `icelines.rinkcraft-numerical-history.v1`, up to 20 unique validated
regular-season catalogs, at most 8 MiB. Each catalog retains original formulas,
components, source fingerprints and annual peer ranks. No pooled percentile.

Core projects `icelines.rinkcraft-history-card.v1` for a player and first/last
season bounds. Sort annual points, preserve missing-player/missing-metric/unranked
states, and retain exposure, situation/group and sources. For each metric count
P >= 80 among ranked seasons only if definition/formula, unit, group and situation
remain compatible. Otherwise withhold the count and keep observations. Missing
years are not zero and are excluded from the ranked-year denominator. The number
of supplied and reported player seasons is displayed separately. High raw ranks
are not always desirable. No persistence, stable-style, ability or class score.

CLI `profile-history --catalog FILE FILE FILE` validates source catalogs then
assembles the archive using RawValue, preserving existing JSON number tokens.
Fetch handles bounded local archive IO. Native optional
`ICELINES_RINKCRAFT_HISTORY` attaches additive profile `numerical_history` only to
regular-season profiles. Apply requested bounds and explicit display-season ceiling
before computing counts; absent last bound uses current season. No later-season
data contributes to an earlier display. Retrospective model vintages remain unknown.

WASM `numerical_history(catalog, player_id, first?, last?)` and worker operation
`numericalHistory` call the same pure projection. Browser optional prepared
`rinkcraft-history.json` is bounded and included in the authenticated offline shell;
active regular-season package supplies its ceiling. This source history is
independent of resident NHL packages and names its own supplied seasons. Existing
stale-dialog guards remain after asynchronous attachment.

Native and browser put a default-closed Evidence over seasons expander within the
primary character card. Annual cells show value/unit, P, measured peers; repetition
column states P >= 80 / ranked years. Coverage and source URL/hash/byte evidence
plus limitations expand separately. MoneyPuck credit is displayed. No asset means
legacy card behavior; historical class absence does not suppress numerical history.

Initial source archive: 25-player cohort, 2023–24 through 2025–26; 74 reported
player-seasons, 75 requested slots, 1,020 ranked measurements. Barkov absent in the
last source is an explicit gap, with no attributed cause. All six positions remain
in scope. Previous 345 primary measurements and 47 style readouts are unchanged.

Acceptance: temporal bounds before counting, duplicate rejection, missingness and
changed groups, input-token preservation, source-component/annual peer evidence,
actual native/WASM projection parity, offline/no-JS/mobile and existing primary/
comparison/history regressions. See matching validation report. No public release
or independent statistical persistence/class validity claim.
