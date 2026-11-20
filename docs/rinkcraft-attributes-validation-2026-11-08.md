# Descriptive attribute validation

CHG-127 / IF-RINKCRAFT-ATTRIBUTES-001 / WP-RINKCRAFT-03, 2026-11-08.
Protocol: `rinkcraft.descriptive-attributes.0.1.0-experimental`. Rinkcraft owns
decision 0007, the measurement protocol and producer-applied role review. These
checks establish implemented arithmetic and presentation, not role/skill validity.

Implemented one pure normal/sparse borrowed-view builder; canonical native
reference loading; additive profile JSON; native/CLI JSON/WASM integration; cards
with measured values, bands, reference counts, GP, formulas and fingerprints;
seasonal step charts and textual history; separate goalie workload/outcome branch.

Local validation:

- `cargo test -p icelines-core --lib rinkcraft --locked`: 10 pass, including
  ties, observed zero/missingness, exposure rates, forward/defense isolation,
  goalie-specific attributes, duplicate identities and undersized references.
- `cargo test -p icelines-wasm --lib rinkcraft --locked` and the same with
  `--features sparse-storage`: one focused profile test passes in each mode;
  explicitly checks attributes and unranked small reference population.
- TypeScript check and five profile/history frontend tests pass.
- `node scripts/build-browser.mjs`: real normal-storage WASM distribution builds;
  32 shell assets and 75 packages pass engine/loading/query verification.
- `node icelines-browser/e2e/rinkcraft-wasm.mjs`: actual exported engine bands
  match the dialog, step paths preserve unmeasured gaps, offline controls and
  390px width pass. Sparse was tested natively, not as a second browser artifact.
- `node icelines-browser/e2e/rinkcraft-native.mjs`: five profile/scope/competition
  cases, API-to-card equality, 7 skater/3 goalie charts, native season controls,
  error responses, no-JavaScript and 390px width pass. Screenshots inspected.
- `python tools/check-rinkcraft-preview.py C:\src\Rinkcraft\outputs\stat-profiles`:
  prior five snapshots still match their original metric/scope/change/source-gap
  fields. New attribute fields are additive, not present in those historical files.
- Native CLI build, `cargo fmt --all -- --check`, `git diff --check`, and
  `cargo clippy -p icelines-core -p icelines-fetch -p icelines-wasm -p icelines-web --lib --locked -- -D warnings` pass.

Actual loaded examples differ: Fox's latest defense cohort has 150 eligible peers,
assist output band 5 and recorded contact band 1; Kartye's forward cohort has 305,
contact band 5; Shesterkin's latest loaded goalie season uses 77 eligible goalies.
These are dated local-source results, not verified current roster/season facts or
an ability ranking. Raw measures, reference coverage and provisional GP policy
are visible on each card. Synthetic boundary tests do not calibrate uncertainty.

No overall level, family sum, archetype assignment, skill model, north–south/
east–west inference, predictive role-change detector or lineup benefit is claimed.
All strengths remain combined and unadjusted. Cohort thresholds and bands are
experimental display conventions; statistical reliability remains untested.
No hosted publication, workflow run, data-cache rewrite or release.
