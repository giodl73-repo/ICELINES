# Playing-method comparison validation — 2026-11-08

CHG-133 / IF-RINKCRAFT-NUMBERS-001. Baseline `b0703bc` plus this working tree.
Contract: `design/specs/rinkcraft-style-comparison.md`. Rinkcraft decision 0013
and producer-applied review govern interpretation. No new numerical inputs,
tracking acquisition, player labels or class/chemistry validity claim.

## Inputs and behavior

Reuse the unchanged enriched catalog retained in Rinkcraft at
`outputs/playing-methods/2026-11-08-01/assessments.json`, also the ignored browser
`public/rinkcraft-numbers.json`. It still contains 25 profiles, 47 style readouts
and 345 original numerical measurements. Tracking coverage remains one player.
Native `ICELINES_RINKCRAFT_NUMBERS` points to the same research copy. Seed/trials/
splits N/A for descriptive subtraction and validation.

McDavid/MacKinnon high-danger attempt share differs by 2.705007 percentage points
(display +2.71 pp); recorded contact differs by about +0.41 /60. McDavid-only
tracking rows retain the three measured values and no difference. Reversing the
pair reverses supported signs and one-sided coverage. Fox/Makar has two matched
method rows; Shesterkin/Hellebuyck has no method rows and three shared method gaps.
Saving outcomes remain visible separately. These are narrow descriptive contrasts,
not complete playing styles, class distances, adjusted skill or overall ranking.

## Executed checks

- `cargo test -p icelines-core --lib rinkcraft --locked`: 20 passed. Shared style
  comparisons cover matching, missing sides, changed sources and legacy absence.
  A pre-existing attribute fixture failed because unordered league iteration could
  include a deliberately ineligible player in a purported two-eligible-peer sample.
  The fixture now explicitly excludes that player; no production attribute logic
  changed. The focused suite passed after the correction.
- Native CLI/web build passed. Library Clippy core/web/WASM with warnings denied,
  TypeScript, five focused frontend history/profile tests, format and diff checks
  passed.
- Full `node scripts/build-browser.mjs` passed, including actual WASM/package
  verification. Rust 1.95.0, wasm-bindgen 0.2.118; build metadata parent `b0703bc`
  plus dirty working tree. No post-commit clean-build claim.
- Final JS/asset build and `node scripts/verify-browser-dist.mjs` passed after
  browser-theme correction. 37 shell assets, 75 packages; shell hash prefix
  `915992ad51f6`; summed gzip shell 422,177 bytes; shell plus largest package
  557,362 bytes, within the 10 MiB budget. No publication.
- `node icelines-browser/e2e/rinkcraft-style-comparison.mjs`: four exact actual
  native API/WASM comparison objects (forward pair/reversal, defense, goalie);
  one-sided tracking, source/unit/scope incompatibility with null differences,
  legacy absence, pp formatting, shared gaps, no-JS native, offline browser and
  390 px layouts passed. Computed foreground/background contrast for new method
  headings, gap summaries and player values is at least 4.5:1 in all three themes.
- Existing `rinkcraft-integrated.mjs`: exact three comparison objects, temporal/
  range/playoff guards, unscouted class evidence, offline/mobile passed.
- Existing `rinkcraft-style.mjs`: all 25 actual native/WASM cards equal, all 47
  readouts recompose, and every original metric deep-equals the retained baseline:
  345 unchanged. Representative style rendering/no-JS/offline/mobile passed.
- Existing `rinkcraft-native.mjs`: five no-JS profile cases passed.

Native and browser screenshots inspected; initial browser host theme put light
text on a white comparison surface. Browser comparison now uses the host surface,
text/border palette and readable player colors. Ignored artifacts:
`target/rinkcraft-style-comparison-check/`, prior integrated/style/native acceptance
directories and `target/rinkcraft-style-comparison-browser-build.log`.

## Remaining limits and source feasibility

No new source data acquired. Rinkcraft reinspected the public All Three Zones FAQ;
data-links navigation redirected to login. No account/purchase/authenticated export
was accessed. Source-access findings do not establish unseen workbook schema,
season coverage or completeness. Existing NHL EDGE batch restriction remains.

Matching definition/source frame is a compatibility check, not statistical
exchangeability, uncertainty, completeness or opportunity adjustment. No direction,
carry/dump choice, pursuit success, deception, goalie mechanics or missing-side
ability is inferred. The one-sided values remain unknown on the other side.
