# Integrated numerical Rinkcraft validation — 2026-11-08

CHG-131 / IF-RINKCRAFT-NUMBERS-001. Baseline `61c054c` plus this working tree.
Research decision 0011 and producer-applied review live in the Rinkcraft repo.
No new external sources, formulas or player class labels. Existing MoneyPuck
2024–25 inputs produce the unchanged 25-player, 345-measurement numerical catalog.
Raw/generated inputs remain ignored; no publication or remote CI was performed.

## Executed checks

- `cargo test -p icelines-core --lib rinkcraft --locked`: 17 passed, including
  fixed projections, selection, missing data, ties, exposure, goalie denominators,
  future ceiling, same-reference differences, invalid pair and payload checks.
- Focused Clippy for core/fetch/WASM/web libraries with warnings denied passed.
- Browser TypeScript check and five focused profile/history frontend tests passed.
- Native build and full `node scripts/build-browser.mjs` passed. Actual WASM
  load/query verification passed; Rust 1.95.0, wasm-bindgen 0.2.118. Prepared shell:
  37 assets, 75 season packages; shell hash prefix `f0d3f0fbcc96`; gzip shell
  403,855 bytes, shell plus largest package 539,063 bytes, within 10 MiB budget.
  Build metadata truthfully records parent `61c054c` and dirty working tree;
  committing source does not rebuild or mislabel this retained artifact.
- `node icelines-browser/e2e/rinkcraft-integrated.mjs`: passed actual WASM
  projection for all 25 players; exact native/WASM comparison object parity for
  McDavid/MacKinnon, Fox/Makar and Shesterkin/Hellebuyck; native period/range/
  competition guards; numerical evidence with an unscouted class; invalid pairs;
  browser card-to-comparison navigation; offline goalie comparison; 390 px layout.
- `node icelines-browser/e2e/rinkcraft-atlas.mjs`: all 25 native cards passed
  identity/class and numerical facets/ranks; configured atlas and browser cards
  passed. Existing Kartye fallback remains separate from the 25-player dataset.
- `node icelines-browser/e2e/rinkcraft-native.mjs`: five no-JS native cases,
  including historical range and playoff profiles, passed.
- `node icelines-browser/e2e/rinkcraft-wasm.mjs`: engine/dialog parity, offline
  selectors, historical gaps and mobile passed. Its old all-table selectors were
  narrowed to the history table after the added numerical table exposed them.
- `ICELINES_TEST_BROWSER=msedge node icelines-browser/e2e/player-history.mjs`:
  four resident windows, regular/playoff isolation, exact values, missing-not-zero,
  goalie history, offline reads, stale-dialog close and 390 px layout passed.

Ignored artifacts: `target/rinkcraft-integrated-check/`,
`target/rinkcraft-atlas-check/`, `target/rinkcraft-ui-check/`, browser E2E output
directories and `target/history-browser-TJaB3b/`. Visual review of the native
primary card, native comparison and WASM card confirmed readable scope, raw rates,
percentiles and distinct class/method/numerical sections.

## Interpretation limits

P is a raw same-position peer rank, not an ability grade. Leading signals use a
fixed editorial shortlist and may be correlated. Differences use unrounded values;
the final digit may differ from subtracting rounded labels. Payload checks verify
structure and compatibility, not a fresh recomputation of its source statistics.
No uncertainty, persistence, opportunity adjustment, causal defense, chemistry,
transport direction, dump/carry method, assignment compliance or goalie mechanics
is inferred. UI parity is software evidence, not validation of hockey classes.
