# Readable matched contrasts validation — 2026-11-09

CHG-138 / WP-RINKCRAFT-14 / IF-RINKCRAFT-NUMBERS-001. Baseline `22e9fed`
plus working tree. Contract: `design/specs/rinkcraft-contrast-summary.md`.
Research authority: Rinkcraft decision 0018 / contrast protocol 0.1 and producer-
applied ten-lens review. No independent panel, new source or empirical class validity.

## Change and inputs

Optional NumericalComparison `contrast` selects up to three nonzero absolute rank
gaps from the fixed six/four primary facets, preserves signs/raw values and
ComparisonRow objects, and reports ranked/total coverage, equal ranks and omissions.
Definitions/scale now join existing matched comparison guards. Primary facet pool
is shared with card construction; missing facets cannot silently change the pool.
Native/browser summaries precede the full fixed-order table; limits stay beside
each result, and method/tracking comparison remains separate. Legacy comparisons
without the optional field keep their table. No overall score or rank pooling.

Unchanged retained inputs: Rinkcraft three-season archive (74 player-seasons,
1,020 measurements), original 2024–25 source/style catalog and class catalog.
Latest numerical source is 2025–26, not a live-current-season guarantee. No input,
value/rank, player-class, history policy or source-count changes. No provider IO.

## Executed checks

- Focused core Rinkcraft: 28 passed. New checks cover fixed candidate pool, stable
  exact ties, signed reversal, equal values without fake highlights, missing and
  unranked omissions, and formula/scale refusals. A test initially assumed player
  order by array index; corrected the fixture lookup to use player ID. No product
  behavior was changed for that test failure.
- Core/fetch/web/WASM library Clippy with warnings denied passed. TypeScript,
  five frontend tests, native CLI/web build and full browser build passed.
- Actual browser distribution verification: 40 shell assets and 75 packages;
  shell hash prefix `c08ce4382a2a`; summed gzip shell 512,400 bytes, shell plus
  largest package 647,919 bytes, within unchanged 10 MiB budget. Rust 1.95.0 /
  wasm-bindgen 0.2.118; metadata records parent `22e9fed` plus dirty working tree.
  No final-commit clean rebuild, publication or hosted CI claim.
- `rinkcraft-contrast.mjs`: all 133 same-group latest-source pairs have valid
  fixed-pool/ranked/nonzero/signed/sorted summaries. Nine full native/WASM objects
  match exactly across forwards, defenders and goalies. Frozen source-derived
  highlighted keys and representative +31.519274/+40.845070 pp gaps match; the
  nearly tied final goalie pair checks membership without inventing a rounded tie.
- Definition/scale changes refuse; missing and unranked primary facets are omitted
  explicitly. Three summary cards show coverage, signed/raw readouts and exact
  per-attribute limits in native no-JS pages. Legacy optional-field rendering,
  offline browser goalie switching, three-theme 4.5:1 text contrast and native/
  browser 390 px overflow checks passed.
- Native Fox/Makar and browser McDavid/MacKinnon/goalie screenshots inspected.
  Names, period, sign direction, raw values, ranks and measurement limits remain
  readable. Large residual or workload gaps do not imply ability or technique.

Ignored results/screenshots: `target/rinkcraft-contrast-check/`; logs:
`target/rinkcraft-contrast-*`. Research retains input hashes, commands, outputs and
build/source provenance. Seed/trials/predictive split N/A: deterministic reading
order, not empirical role/style/skill prediction. Related shooting facets can
share underlying events; rank gaps are not effect sizes or hockey importance.

Final regressions passed: integrated numerical cards/three exact matched pairs,
four exact method-comparison pairs with missing/incompatible/legacy guards, and
attributes-first acceptance (25 preserved vectors, nine exact native/WASM cards,
first-screen placement, feature links, unsupported transport, legacy/no-JS/
offline/mobile). No source or class values changed. Final previews remain managed
and loopback-only on 8765 and 8766.
