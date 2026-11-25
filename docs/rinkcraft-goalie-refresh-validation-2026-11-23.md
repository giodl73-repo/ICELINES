# Goalie estimates and routine refresh: validation

Validated locally on 2026-11-23. The recipe contract and replay instructions are
in [Goalie stat estimates and routine refresh](../design/specs/rinkcraft-goalie-stat-attributes.md).

The four-season catalog contains 377 goalie-season profiles. All six estimates
are available for 362; the remaining historical profiles lack relevant shot
opportunities. All 78 current-season goalies have all six numbers. The latest
combined current-season refresh covers 787 skaters and 78 goalies, with 864 of
865 profiles complete; the remaining exception lacks NHL EDGE skating data.

The merged four-season catalog contains 3,948 profiles, 3,930 complete profiles
and 23,660 scored attributes. Remaining unknowns stay unknown rather than zero.
Puck Play is intentionally coarse: 65 of 78 current goalies share score 8 from
tied zero-assist rates. This identifies puck-touch and pass opportunities as the
first deeper measurement priority, not a finding that those players have the
same underlying puck-handling skill.

## Checks

- 20 focused Rust integration tests passed: six-recipe goalie calibration,
  separate F/D/G references, small recipients, source joins, shots-on-goal-only
  save denominators, rebound/cross-slot conjunctions, source disagreement,
  zero/missing separation, corruption, archive member bounds and cache replay.
- 21 WASM library tests passed after moving the existing season-package envelope
  into the shared source schema. Its public WASM type remains re-exported.
- 242 browser unit tests passed. Native CLI/Askama, TypeScript and pinned Rust
  1.95.0 WASM builds passed. Offline distribution verification passed.
- Actual WASM reproduced native grades and stat points for all 1,164 observations
  in the three historical goalie refreshes and combined current-season refresh:
  6,984 attribute comparisons, with tolerance 1e-8 for serialized floating values.
- The ordinary browser app passed two skaters and two goalies across two seasons,
  source/formula selection, history, mobile, offline reopening and missing,
  malformed and stale catalogs. Importing the generated current-season package
  passed Shesterkin, Hellebuyck and Bobrovsky, with matching GP and known minutes.
- Native historical Shesterkin HTML displayed six goalie estimates and known
  workload. Current offline native data was not present in the bundled player
  window; the generated package supplies the fresh data to the browser instead.
- The 78-goalie local preview passed three player selections, formula dialogs,
  390-pixel layout and no page errors. Existing source-bound documentary sheets
  are reused where available, retaining their original dates and hypotheses.

## Reproduction artifacts

Generated artifacts remain outside Git under `target/goalie-refresh/`:

- `all-current-compact/rinkcraft-stat-attributes.json`: SHA-256
  `902a4ef5ed633d56e70f8f448cac5fb939144628930d6cc6901934415dd91926`
- `all-current-compact/season-package.json`: SHA-256
  `ba6417951ab67114b46b09fc72351646eb60ad67358e33523a5427331a0291cd`
- `all-current-compact-replay/`: byte-identical catalog and package, using global
  `--no-live` and the checksum-verified source cache.
- `all-current-final/` and `all-current-final-replay/`: a final online run and
  global-`--no-live` replay also matched byte for byte. The catalog retained the
  hash above; the newly fetched season package has SHA-256
  `3536ed9cf5a43e669109444e391db354e8059ee9eb577630dde4696957b08f0c`.
- `all-current-v2/`: same catalog values in the earlier pretty-printed artifact,
  plus its matching package and the online source report. Compact catalog output
  reduces 27,028,827 bytes to 15,621,601 and enforces the reader's 32 MiB bound.
- `wasm-parity.json`, `coverage.json`, `browser-tests-final.txt`, and preview images.
- Ordinary browser acceptance results/screenshots are in
  `target/attribute-runtime/`; its script accepts `ICELINES_REFRESH_PACKAGE`
  alongside the catalog and WASM-package overrides.

Online runs refresh NHL bulk reports and the advertised MoneyPuck downloads.
An EDGE detail is reused only if it matches the selected actor, season and GP.
Offline replay prioritizes a matching refreshed cache over an older retained
capture. That precedence has an explicit regression test. The exporter emits a
matching typed season package so fresh attributes can be used with fresh player
rows. It does not publish, deploy or schedule anything. No hosted or release
build was run.
