# Skater attribute depth — 2026-11-23

Added a shared Rust skater context, source parsers for credited assist types and
shot quality, refresh integration, native/browser records and per-attribute detail.
The six scoring recipes remain the baseline for later comparison.

## Evidence and results

- 787 current skater contexts, with 25 measurement rows each.
- All 787 primary/secondary assist totals reconcile with NHL summary assists,
  using all 377 captured finalized games. Zero credited assists remains zero.
- 776 players have attributed goalie-in-net shot-quality records. The other
  11 retain unavailable values rather than invented zeroes.
- All 3,948 supplied profiles retain identical six-attribute grades. There
  are 786 complete current skater estimates; the prior single missing skating
  estimate remains unavailable.
- MacKinnon and Aho both have 20 assists, with primary/secondary splits of
  13/7 and 7/13. Their primary-assist rates are 1.93 and 0.89 per60 respectively.
  This distinguishes credited involvement beyond total assists, without claiming
  to measure every pass or isolate playmaking skill.
- Fox's 15 primary and nine secondary assists are shown alongside separate EV
  and PP assist rates and PK deployment. Strength rates use their own time totals.

## Validation

- Three new Rust source tests pass: official assist order, shootout/actor guards,
  consistent shot populations, misses, empty nets, strength-specific denominators,
  recorded zero and mismatched assist coverage.
- Three existing refresh integration tests pass: exact replay, prior-window
  preservation, checksum checks, archive handling and EDGE cache precedence.
- Native CLI build passes, including the new Askama includes. Rust 1.95.0 WASM
  build and matching bindings pass.
- Browser build and all 245 browser unit tests pass.
- All 787 cards were projected through actual WASM with exact current GP matching.
  Existing documentary sheets retain their original evidence dates. Current feat
  profiles were evaluated by the existing Rust rules over complete cached reports
  for 865 players; no new feat recipe was introduced.
- Committed browser acceptance passes McDavid, Aho and Fox: 18 attribute dialogs,
  shared worker attachment, 390px overflow checks and no page errors. Screenshots
  were inspected; feat/disclosure colors now remain readable on paper cards within
  the dark app theme.
- Three independently inspected PDFs contain Attribute Record on one 612×792-point
  Letter page. Scoped print rules compact skater spacing and interactive feat
  controls while preserving measured rows and grades.
- The shared game-capture refactor reproduces the original goalie puck ledger
  byte for byte, SHA-256
  `fa95ea65da0676bbad186c6dc93af7b0bab8e7753a6261f84b8452c53a9b4123`.

## Reproduction

From the owned worktree, using existing raw source archives read-only:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-refresh --season 20262027 --kind skaters --cache-dir target/goalie-refresh/cache --game-cache-dir target/goalie-puck-study/cache --edge-captures F:/src/ICELINES-worktrees/rinkcraft-sheet/target/compact-sheet/edge-cohort-20261119 --output-dir target/skater-attribute-study/refreshed-with-edge --previous-catalog target/goalie-shot-context/refreshed/rinkcraft-stat-attributes.json
```

An independent run into `replay-with-edge` produces identical catalog bytes:
`0baf6827ac4b667eef0c8da2105178e40f421a8e3b8e958ec5c487e339706731`.
Matching skater-only season-package SHA-256:
`4e6b2b070c029e327f0bb2a0d3888d7a792ecca0d2082106db5f74f731e2c34a`.

An initial run omitted the retained EDGE archive and lost skating inputs for many
players. It was not used for the preview. Including the original source-bound
captures restored baseline coverage and all grades; no source window checks were
relaxed and no grades were copied into missing data.

The loopback preview at `127.0.0.1:8780` serves generated profiles from
`target/skater-attribute-study/site/`, with matching actual worker/WASM. Prepared
package v1 still omits some optional legacy record fields; the new attribute
records use the complete source assemblies. This is not a full native runtime or
hosted deployment. The goalie preview on 8777 remains separate.

Ignored artifacts include the refreshed catalogs/packages, raw-source report,
projection script/results, screenshots, PDFs, quiet server log and browser results.
The prepared-preview acceptance command is:

```powershell
node icelines-browser/e2e/rinkcraft-skater-context.mjs
```

Override `ICELINES_SKATER_CONTEXT_URL` for another prepared preview. The test does
not acquire data or start a server.
