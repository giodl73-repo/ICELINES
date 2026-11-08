# Character-card validation

CHG-128 / IF-RINKCRAFT-CHARACTER-001 / WP-RINKCRAFT-04, local 2026-11-08.
Owner explicitly corrected the information hierarchy: identity/class first,
historical/statistical exploration behind expanders. Inspected original official
blank and filled Cleric/Wizard sheets plus the official section guide. The UI is
original Rinkcraft CSS/design, not copied D&D artwork or game mechanics.

Generic Rust types/validator/selector consume an optional authored research
catalog. No real-player class records are committed in IceLines. Native server
uses explicit `ICELINES_RINKCRAFT_SHEETS`; browser receives an optional offline
research snapshot through stateless WASM `character_sheet`. Current research pilot
is three producer-authored, source-linked provisional documentary identities.

Checks completed:

- Twelve focused `cargo test -p icelines-core --lib rinkcraft --locked` tests pass;
  character additions verify source support, unsafe URL rejection, unknown early
  periods and later-feature suppression. Existing measurements still pass.
- Focused WASM profile test passes with normal and sparse storage. Actual exported
  normal WASM character binding is exercised by the browser test; sparse was not
  rebuilt as a second browser artifact.
- Native CLI build, production library Clippy for core/fetch/wasm/web, formatting,
  TypeScript and diff checks pass. Five profile/history frontend unit tests pass.
- Browser build produces real WASM and a 35-asset authenticated offline shell.
  The initial verifier rejected the two newly added asset names; its exact
  allowlist was extended for the shared CSS and optional character JSON. Build
  and subsequent `node scripts/verify-browser-dist.mjs` verification pass, including
  actual engine loading/query checks for 75 packages. No wildcard broadening.
- Native five-case acceptance matches API identity/analogy/bands, checks primary
  card visibility and default-closed charts/history, historical unknowns and
  suppressed later multiclass, no-JavaScript behavior and 390px document width.
  Legacy attribute-anchor links retain the new primary character card.
- WASM acceptance matches the independently instantiated exported character
  selector to the visible card and preserves statistical history controls, step
  gaps, offline behavior and 390px dialog width.
- Existing browser-history acceptance passes using local Edge via
  `ICELINES_TEST_BROWSER=msedge`: resident/regular/playoff values, missing records,
  goalie values, no extra season downloads, stale-dialog closing and three themes.
- Original five statistical snapshots still match identity/window/metrics/changes/
  source gaps. Documentary class fields are additive and separate.

Screenshots under ignored `target/rinkcraft-ui-check/` and
`target/rinkcraft-wasm-check/` were visually inspected. Contrast was corrected for
the parchment card inside the dark browser host; card stat period/GP remain
visible independently of source-evidence dates. Historical charts are closed by
default. Local preview endpoints are loopback only.

Class support is curated documentary interpretation, not a statistical classifier,
calibrated proficiency bonus, source-endorsed D&D class, current-role continuity
claim or independent expert validation. The conservative publication-season
cutoff is not a detector of when behavior changed. No hosted publication/release,
source-cache rewrite, private export or game artwork was introduced.
