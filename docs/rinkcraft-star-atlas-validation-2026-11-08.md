# Rinkcraft star atlas validation — 2026-11-08

CHG-129 extends the optional documentary catalog and character card. Authoritative
research input: `C:/src/Rinkcraft/assessments/2026-11-08-stars-25.json`, protocol
`rinkcraft.documentary-class.0.2.0-experimental`. Source baseline is local commit
`c88c9b9469311584821d565ae858b76ac9e3755a` plus the CHG-129 working tree. The
browser artifact records its actual source state; it is not rebuilt merely to
replace dirty metadata after the local commit. No release or hosted publication.

Local previews: native `http://127.0.0.1:8765/rinkcraft`, standalone browser
`http://127.0.0.1:8766`. Set `ICELINES_RINKCRAFT_SHEETS` to the authored input for
native. Explicitly copy it into ignored browser public `rinkcraft-characters.json`
before `node scripts/build-browser.mjs`. External catalog remains below 128 KiB.

Meaningful checks: focused core catalog/attribute/profile tests; source precision,
unknown/duplicate feature keys and temporal suppression; TypeScript and existing
five frontend regressions; native no-JS card tests; actual WASM class selection;
all 25 configured native cards/stat lines/percentiles and 26 atlas entries;
skater/goalie opening through browser atlas; mobile overflow; existing WASM
offline/history and player-history flows. Commands and results are recorded below
after execution. Ignored acceptance output: `target/rinkcraft-atlas-check/`.

Limits: this verifies software behavior and catalog coverage, not role validity,
source representativeness, current-role continuity or complete career coverage.
Ranking membership, documentary identity, measured involvement and execution
quality are separate. Seeds/trials are not applicable to deterministic rendering.

Executed results:

- `cargo test -p icelines-core --lib rinkcraft --locked`: 13 passed.
- `cargo test -p icelines-web --lib rinkcraft --locked`: two passed, including
  fractional-percentile display rounding. The expanded check exposed Rust's
  ties-to-even versus JavaScript's half-up formatting; compact P labels now use
  the same scaled half-up rule. Underlying ranks were unchanged.
- `cargo test -p icelines-wasm --lib rinkcraft --locked`, repeated with
  `--features sparse-storage`: one focused test passed in each mode.
- Library Clippy for core/fetch/WASM/web passed; web Clippy passed again after
  the formatting correction. `cargo fmt --all -- --check` and diff checks passed.
- `npm run check --prefix icelines-browser` and five selected frontend tests passed.
- `node scripts/build-browser.mjs` and `node scripts/verify-browser-dist.mjs`
  passed with actual Rust/WASM: 35 shell assets, 75 packages; final JS-only build
  shell `f16b6e389b64`, app plus largest package 498,418 gzip bytes.
- `node icelines-browser/e2e/rinkcraft-native.mjs`: five no-JS cases passed,
  including earlier source suppression, period controls and narrow layout.
- `node icelines-browser/e2e/rinkcraft-atlas.mjs`: all 25 native class/stat cards,
  raw values and percentiles, 26 atlas entries, mobile overflow, all 25 actual
  WASM class selections, browser skater/goalie atlas opening passed.
- `node icelines-browser/e2e/rinkcraft-wasm.mjs` and
  `ICELINES_TEST_BROWSER=msedge node icelines-browser/e2e/player-history.mjs` passed:
  actual engine/rendering, resident-window isolation, offline controls, missing
  values, stale dialogs and themes. No complete-career coverage is implied.
- McDavid desktop/mobile, Fox and Hellebuyck screenshots visually inspected.

Authored input SHA256:
`fceb3052e48d95236411691c9fae817b4e336ab07c8341652db58f06315cffc6`.
Acceptance retains 25 API JSON snapshots and `results.json` under the ignored
output directory, including normalized catalog fingerprints and actual loaded
season/evidence dates. Rinkcraft stores a copied deterministic run record under
its ignored `outputs/stars/2026-11-08-01/`. Final native rebuild includes the
Connector crest correction; this does not change classification or statistics.
