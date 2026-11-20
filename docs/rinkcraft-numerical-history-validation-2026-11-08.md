# Annual numerical Rinkcraft validation — 2026-11-08

CHG-134 / IF-RINKCRAFT-NUMBERS-001. Baseline `b8e0da2` plus this working tree.
Contract: `design/specs/rinkcraft-numerical-history.md`. Rinkcraft owns protocol,
decision 0014, source registry and producer-applied ten-lens review. No independent
persistence/class validation or public release.

## Source inputs and generation

Preserved existing 2024–25 catalog, 345 measurements and 47 method readouts.
Four additional publicly listed MoneyPuck regular-season downloads were captured
2026-11-08 into ignored `target/rinkcraft-history-pilot/`:

| Year/type | Bytes | SHA256 |
|---|---:|---|
| 2023 skaters | 3430973 | `ee8d191d3633064c1f4f76d64c76e04f8a8e555528a4b49caa6d0ecb8bac9299` |
| 2023 goalies | 93777 | `9b9ca9dad5f2b4fc9ebb65f3fea09915335fb1bbe3868366f61a7f3075389645` |
| 2025 skaters | 3483132 | `9a7e1f1a2f6bc8bc2bf10c5ba32947b47c4b26d5397287e4b30e9d662c4966e4` |
| 2025 goalies | 93386 | `2fe2a2dbf25822cf1650e2ad00388c94ef303681fbfd8a8c7fdcb065212cd2fc` |

URLs follow the inspected public season-summary paths under
`https://moneypuck.com/moneypuck/playerData/seasonSummary/{year}/regular/{type}.csv`.
Actual URLs/capture dates are in the retained source manifests and Rinkcraft
SRC-105 through SRC-108. MoneyPuck.com noncommercial terms/visible credit apply;
raw/generated inputs stay ignored. Dictionary access was not retried; no new
ambiguous fields or goalie/transport technique measurements.

Existing `profile-numbers --season YYYYZZZZ --skaters FILE --goalies FILE --player
<authored 25 IDs>` generated annual catalogs; `profile-style --numbers FILE
--edge-captures DIR` used an empty capture manifest for the two new years, with
no invented tracking data. Assembly:

```
icelines --no-setup --no-live profile-history --catalog target/rinkcraft-history-pilot/20232024/assessments.json icelines-browser/public/rinkcraft-numbers.json target/rinkcraft-history-pilot/20252026/assessments.json
```

Archive: 865,334 bytes, three requested seasons, 74 reported player-seasons from
75 requested slots, 1,020 ranked measurements. Barkov absent in the 2025–26 source
is an explicit missing-player slot with no attributed cause. Annual eligible peers:
2023–24 F431/D247/G71; 2024–25 F424/D240/G71; 2025–26 F441/D237/G71.

Research retains the archive/source copies in ignored
`outputs/numerical-history/2026-11-08-01/`. Native
`ICELINES_RINKCRAFT_HISTORY` points to its `history.json`; browser optional
`public/rinkcraft-history.json` is the same prepared archive. Seed/trials/splits
N/A. Current class and primary numerical/style catalog remain unchanged.

## Executed acceptance

- Core focused Rinkcraft suite: 21 passed, including new missing-season, bounds
  before counting, incompatible-group count suppression and duplicate-year tests.
- Native CLI/web builds passed; history CLI validates input catalogs and RawValue
  assembly preserves their original JSON number tokens.
- Library Clippy core/fetch/web/WASM with warnings denied, TypeScript, five focused
  frontend history/profile tests, format and diff checks passed.
- Full browser Rust/WASM build passed with actual engine/package verification.
  Rust 1.95.0, wasm-bindgen 0.2.118; retained build metadata parent `b8e0da2` plus
  dirty working tree. Final JS/asset build/verification after theme/wrapping fixes
  passed. Shell: 39 authenticated assets, 75 season packages, hash prefix
  `687f218cda67`; summed gzip shell 490,588 bytes; shell plus largest package
  625,993 bytes, within 10 MiB. No post-commit clean-build claim.
- `rinkcraft-history.mjs`: 25 actual WASM history cards, 1,020 source-matched and
  component-recomposed values/percentiles/reference hashes; all three archive
  catalogs deep-equal original inputs; four exact native API/WASM projections
  (McDavid, Makar, Hellebuyck, Barkov); explicit missing slot and duplicate rejection;
  earlier/display/range/playoff guards; no-JS, offline, default-closed expander,
  390 px layouts including expanded source/limit evidence passed. Its initial
  browser rehearsal requested an unavailable 2026–27 prepared NHL package; the
  successful test uses the available 2025–26 package, which includes all three
  independently prepared history years under its ceiling.
- Existing integrated/style/style-comparison E2Es passed: period/class/numerical
  guards, exact comparisons, all 25 primary cards, unchanged 345 measurements and
  47 style readouts, invalid source/context, legacy and offline/mobile checks.
- Native no-JS five-case regression passed. Existing actual WASM player-history
  acceptance passed four resident windows, competition separation, exact values,
  missing-not-zero, goalie values, offline reads, unchanged active window, stale
  dialog close and 390 px layout (`target/history-browser-Tm9FyA/`).
- Native/browser screenshots inspected. Browser host table stripes/hover and links
  now respect the parchment card; source hashes wrap in narrow views.

Ignored artifacts: `target/rinkcraft-history-check/`, history browser build log,
existing acceptance directories and retained research run manifests/results.

## Findings and limits

MacKinnon's shots/60 is around P98 in all three snapshots. McDavid's primary
assist-rate ranks are P99.9/P99.4/P86.3 despite raw rates 1.53/1.17/0.80 per 60.
Hellebuyck's GSAx-rate upper-quintile count is 2/3, with the latest snapshot P57.0.
These describe observed annual outputs, not stable talent, technique or cause.

P >= 80 is an explicit display threshold. High values are not uniformly desirable.
Annual peer populations differ; no percentile average or pooled career rating.
Provider/model/recording and opportunity/deployment confounding remain. All files
were acquired retrospectively; contemporaneous model vintages and independent
methodological corroboration are unestablished. No directional/carry/dump/pursuit,
goalie-mechanics, assignment or class validity inference follows.
