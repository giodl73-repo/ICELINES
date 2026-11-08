# Playing-method validation — 2026-11-08

CHG-132 / IF-RINKCRAFT-NUMBERS-001. Source baseline `8b74f84` plus this working
tree. Contract: `design/specs/rinkcraft-playing-methods.md`. Research definitions,
ten producer-applied lenses and SRC-102/103 custody live in Rinkcraft. No class
relabeling, raw tracking trajectories, validated style score or public release.

## Retained inputs and reproduction

Original MoneyPuck catalog and 345 measurements from CHG-130 remain preserved.
One public NHL EDGE response was captured at the pinned 2024–25 regular-season
McDavid endpoint: 4,839 bytes, SHA256
`75fb523d2eca4c748f9d085b7287a9b0ac63444fbc3de6fcf4ef6e5cf7d7b44b`.
The first PowerShell request returned 200; web-tool API access failed, and a
subsequent Python batch returned 403 with no retained batch responses. It was
stopped without retry/bypass. No working acquisition helper is claimed.

Ignored capture directory: `target/rinkcraft-style-pilot/20242025/`; contains
`manifest.json` and `8478402.json`. Exact generation command:

```
icelines --no-setup --no-live profile-style --numbers target/rinkcraft-numerical-pilot/20242025/assessments.json --edge-captures target/rinkcraft-style-pilot/20242025
```

Output retained as a separate `assessments.json`, copied to ignored browser
`public/rinkcraft-numbers.json` and Rinkcraft's ignored
`outputs/playing-methods/2026-11-08-01/assessments.json`. Native
`ICELINES_RINKCRAFT_NUMBERS` points to that enriched research copy. Original run
files remain intact. Seed/trials/splits N/A: deterministic descriptive projection.

Coverage: 25 profiles, 22 skaters with two shot/contact readouts, one of those
with three tracking readouts, three goalies with explicit method gaps. Total 47
style readouts. McDavid: 23.969 mph maximum; 442/67 = 6.597 bursts/game;
256.1121/67 = 3.8226 skating miles/game. Five-on-five high-danger attempt share:
McDavid 7.66%, MacKinnon 4.95%, Fox 2.78%, Makar 1.64%. These describe provider
shot categories, not net-front skill, choice quality or a player ranking.

## Executed acceptance

- Core focused Rinkcraft tests: 19 passed. New cases cover movement denominators,
  shot share versus contact rate, explicit gaps, zero denominators, inherited
  card projection and rejecting malformed style components.
- Fetch `rinkcraft_style` test: passed bounded local identity/hash input with
  expected values, changed-byte rejection and incompatible-season rejection.
- CLI/native builds passed. Style enrichment initially exposed last-digit f64
  parse/serialize changes in existing percentiles. The CLI now retains original
  JSON numeric tokens using RawValue and adds only style evidence. The actual
  source regression deep-compares every original metric, including formulas,
  values, percentiles, components and reference fingerprints: all 345 unchanged.
- Library Clippy core/fetch/web/WASM with warnings denied passed; TypeScript,
  five focused frontend profile/history tests, formatting and diff checks passed.
- Full browser Rust/WASM build and actual engine/package verification passed;
  retained build metadata is parent `8b74f84` plus dirty working tree, Rust 1.95.0,
  wasm-bindgen 0.2.118. Native-only RawValue serialization does not change WASM
  arithmetic. JS/asset rebuild after the exact-preservation fix passed.
- Final prepared shell: 37 authenticated assets, 75 packages, hash prefix
  `21e086759749`; gzip shell sum 414,079 bytes; shell plus largest package
  549,266 bytes, within the 10 MiB budget. No post-commit clean-build claim.
- `node icelines-browser/e2e/rinkcraft-style.mjs`: all 25 actual native numerical
  API cards exactly match actual WASM cards, 47 component-recomposed readouts,
  original 345 metrics unchanged, representative native/browser rendering,
  goalie-specific gaps, no-JS, offline and 390 px layout passed. Pure comparison
  endpoints avoid unrelated career hydration for the all-player source check.
- Existing integrated E2E: exact three comparison objects, time/range/playoff
  guards, unscouted-class numerical evidence, offline comparisons and mobile
  passed. Native no-JS five-case profile regression also passed.
- Native/browser screenshots inspected. Browser host-theme summary/link contrast
  was corrected for the new style section and the prepared shell reverified.

Ignored artifacts: `target/rinkcraft-style-check/` and the existing integrated/
native acceptance directories. Research run retains input/output hashes, capture
manifest and copied results. Reviews are producer-applied, not independent.

## Remaining limits

Tracking coverage is one player, not 22. Response has no independently echoed
season; requested endpoint and hash custody establish requested scope. Bursts are
per game, not per skating minute or with-puck opportunity. Peaks depend on exposure.
No capture completeness/trajectory accuracy or peer-eligibility specification was
established; tracking percentiles are omitted. Shot/contact measures remain 5on5;
tracking all strengths. No direction, carry/dump choice, pursuit/recovery success,
passing deception, assignment compliance, goalie mechanics, persistent style,
chemistry or class validity follows from these software checks.
