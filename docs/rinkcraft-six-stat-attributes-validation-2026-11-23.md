# Six stat attributes: validation

The primary skater profile uses the owner's approved
[simple-stat recipes](../design/specs/rinkcraft-simple-stat-attributes.md) for
Skating, Handling, Passing, Shooting, Reading and Defending. Rust owns extraction,
peer ranking, weights, rounding and snapshot selection. These are estimates;
the separate technical measurement research battery does not gate this version.

Implementation checkpoint: `5e011866` (PR #133). Validation date: 2026-11-23.

## Population audit

The retained projection covers 3,565 skater-season profiles from 2023–24 through
2026–27, including forwards, defensemen and small samples. Every emitted score's
weighted arithmetic, lower-is-better direction, peer minimum and 1–20 conversion
was independently checked against its supplied parts. Duplicate actor/windows
were absent. All 671 small-sample profiles are marked as such.

| Attribute | Scored | Missing |
| --- | ---: | ---: |
| Skating | 3,562 | 3 |
| Handling | 3,565 | 0 |
| Passing | 3,565 | 0 |
| Shooting | 3,565 | 0 |
| Reading | 3,565 | 0 |
| Defending | 3,565 | 0 |

There are 21,387 scored attributes and 3,562 complete six-score profiles.
The skating exceptions are Xavier Bourgault (2026–27, 2 GP), Colin White
(2024–25, 3 GP) and Jacob Gaucher (2024–25, 4 GP). Both the NHL EDGE skater-detail
and skating-speed-detail endpoints returned 404 for these actor/seasons on
2026-11-23. Missing observations remain unknown; a 404 does not establish why
the provider omitted them. No other-season measurements or zero substitutes
were used.

The audited generated artifact is
`target/attribute-inputs/catalog-v9/rinkcraft-stat-attributes.json`, SHA-256
`869260864b22334a78bb0d9f26a775ee7ffaab9709ae17456ad9a39d18a21fd0`.
It remains outside Git. The source projectors and source-selection rules are
documented in the linked specification.

## Runtime evidence

- Native CLI, native Askama HTML and actual WASM selected the same McDavid
  2024–25 estimates: 20, 15, 20, 15, 17, 8. No numerical dossier was required.
- The ordinary browser player dialog passed McDavid and Makar in both 2023–24
  and 2024–25, using the real worker and WASM binding. All six displayed grades
  matched the catalog, and attribute dialogs showed the selected GP and formula.
- Selecting an older season after a newer resident season retained the older
  grades. Offline reopening worked. The 390-pixel dialog had no horizontal
  overflow. A screenshot was inspected.
- Missing, malformed and stale-GP catalogs preserved the player card and showed
  six unrated fields. Invalid and mismatched supplied data produced availability
  notes. A goalie received no skater estimates.
- An explicit native missing-skating fixture showed a dash with **Unrated** and
  retained the other five estimates. This exposed and fixed a native-only label
  bug that had called a missing score an estimate.
- 239 browser unit tests and 15 focused Rust integration tests passed at the
  checkpoint. TypeScript, native CLI/Askama and pinned WASM builds passed.
  Offline distribution verification included the optional catalog asset.
- Both Rust scoring unit tests passed, covering the six recipes, position-peer
  selection, reversed negative stats, ties, measured zero and small samples.

The browser runtime acceptance is repeatable with:

```powershell
$env:ICELINES_WASM_PKG=(Resolve-Path target/attribute-inputs/wasm-pkg).Path
$env:ICELINES_STAT_CATALOG=(Resolve-Path target/attribute-inputs/catalog-v9/rinkcraft-stat-attributes.json).Path
node icelines-browser/e2e/rinkcraft-stat-attributes.mjs
```

Both overrides are optional when `icelines-browser/dist` already contains the
current bindings and catalog. Overrides serve local artifacts without replacing
publication assets. Results and the mobile screenshot go to
`target/attribute-runtime/`. This is local acceptance, not a hosted release or
proof that these stat proxies measure technical skill. Goalie scoring remains
its own attribute family.
