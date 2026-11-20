# Numerical Rinkcraft validation — 2026-11-08

CHG-130 / IF-RINKCRAFT-NUMBERS-001. Source baseline `abd60e6` plus the numerical
assessment working tree. Pure model in core; local CSV ingestion in fetch; thin
CLI/native surfaces. No network/cache mutation by the calculation command.

Actual source inputs: MoneyPuck.com 2024 regular season summary files:

- Skaters: 3,410,678 bytes; SHA256
  `467260ae6b411621fc3e736ed005588f75dd4470610416626584f2878a463fd4`.
- Goalies: 97,451 bytes; SHA256
  `a718782ffbeb8d0d05eb6921f4cbea300dbcaa86be8e068930e71be4a22a9d8d`.

Source credit/terms: `https://moneypuck.com/data.htm`; metric definitions:
`https://moneypuck.com/glossary.htm`. Dictionary fetch was unavailable; no unseen
dictionary claim or rebound/freeze technique metric is introduced. Current source
columns differ from the legacy MoneyPuck adapter's expected names, so this bounded
assessment adapter validates the inspected schema explicitly rather than silently
substituting fields or modifying existing cached summaries.

Command: `icelines --no-setup --no-live profile-numbers --season 20242025 --skaters
target/rinkcraft-numerical-pilot/20242025/skaters.csv --goalies
target/rinkcraft-numerical-pilot/20242025/goalies.csv --player <25 IDs>`.
IDs come from Rinkcraft's authored featured cohort. Results are ignored in
`target/rinkcraft-numerical-pilot/20242025/assessments.json`; Rinkcraft retains its
own ignored run copy, inputs and source/hash manifest.

Observed result: all 25 selected IDs, zero missing; 345 ranked measurements;
424 forward, 240 defense and 71 goalie eligible references. Skaters are five-on-
five, goalies all strengths. Primary-assist source boundary check: McDavid 23
primary assists / 70,524 seconds. Goalie boundary: Hellebuyck GSAx 39.59 /
224,482 seconds. No quote/class fields enter these calculations.

Executed checks:

- Three focused core tests: rates/full league reference despite selected output,
  ties, zero/missing values, negative residual, duplicate IDs, zero/low exposure,
  missing player and goalie time/shot denominators.
- One focused fetch test: current schema, exact season and empty-cell guards.
- Full Rinkcraft core regression slice: 16 passed after the addition.
- Library Clippy for core/fetch/web passed; format and diff checks passed.
- `cargo +1.95.0 check -p icelines-wasm --target wasm32-unknown-unknown --locked`
  passed, confirming compatibility of the added pure core module.
- `node icelines-browser/e2e/rinkcraft-numbers.mjs`: all 25 native profiles and
  345 displayed formula/value/percentile cells; source hashes, scope, peer counts,
  no-JS and 390px layout passed. Desktop/mobile screenshots visually inspected.
- Existing native five-case character/profile acceptance passed after the final
  navigation/layout rebuild, including historical and goalie/playoff cases.

Native `/rinkcraft/numbers` and `/api/v1/rinkcraft/numbers` use the explicit
`ICELINES_RINKCRAFT_NUMBERS` JSON input. The operator supplies the generated file.
This new adapter/page is CLI/native only; no new offline numerical browser
artifact or WASM input adapter is claimed. Compiling the shared core for WASM
checks compatibility, not new feature availability there.

This verifies software and source-based descriptive arithmetic. Exposure filters
are exploratory; there is no uncertainty/persistence model, causal individual
defensive attribution, automatic class assignment, overall skill score or
validated numerical-to-role mapping. Research protocol, source registry and
producer-applied .roles review live in Rinkcraft. No public release or source-file
redistribution was performed. Seed/trials: not applicable.
