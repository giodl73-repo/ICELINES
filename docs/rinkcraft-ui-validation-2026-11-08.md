# Rinkcraft player profile UI validation

CHG-126 / IF-RINKCRAFT-UI-001 / WP-RINKCRAFT-02. Local working-tree validation
on 2026-11-08; no publication, release or hosted CI was run.

Native IceLines links player pages to Rinkcraft profiles and exposes the standard
JSON twin. The standalone browser player dialog calls the same core builder via
WASM, over resident prepared packages. Native optional reports and browser package
v1 have different coverage; no equality of source depth or complete-career claim.

Checks completed:

- Six existing core profile tests pass after the borrowed-view refactor.
- All 21 WASM library tests pass with normal storage and all 21 with
  `sparse-storage`, including profile scope, null optional reports, competition
  isolation, nonmutating residency and unload behavior.
- Two focused native profile tests pass: request validation and missing/percentage
  formatting. Two browser profile tests and three existing history tests pass.
- TypeScript check, `cargo fmt --all -- --check`, `git diff --check`, and
  `cargo clippy -p icelines-core -p icelines-web -p icelines-wasm --lib --locked -- -D warnings`
  pass. Native CLI build succeeds.
- `node scripts/build-browser.mjs` builds the normal WASM distribution and verifies
  actual engine loading/query behavior, 31 shell assets and 75 prepared packages.
  Browser-pinned Rust 1.95.0 required installing its WASM target locally; the
  stable toolchain's existing WASM target did not cover that separate toolchain.
- `node icelines-browser/e2e/rinkcraft-wasm.mjs`: actual exported WASM/profile
  values match the dialog, two resident regular seasons exclude loaded playoffs,
  offline controls preserve unavailable metrics, and the dialog fits 390 pixels.
- `node icelines-browser/e2e/rinkcraft-native.mjs`: five native HTML/JSON examples,
  season selection, invalid-parameter/unknown-player responses, JavaScript disabled
  and 390-pixel document width pass in local headless Edge.
- `python tools/check-rinkcraft-preview.py C:\src\Rinkcraft\outputs\stat-profiles`
  matches all five prior CLI snapshots for identity, request, seasons, latest row,
  summary, full timeline, changes and source gaps. Historical snapshots remain
  unmodified. During validation the native adapter was corrected to seed its
  player-local repository with the running app's active row before bundled career
  fan-out, preserving the newer current season.

Screenshots and detailed results are ignored local artifacts under
`target/rinkcraft-ui-check/` and `target/rinkcraft-wasm-check/`. The local preview
servers bind loopback only. Automated visible browser opening was unavailable:
Playwright MCP stalled and automatic approval review blocked the default-browser
launch without a detailed reason. Local Edge headless checks succeeded; visible
opening is available through the reported localhost links.

Review limits: existing CLI semantics remain authoritative. These screens show
statistical evidence, proposed family descriptions and explicitly unmeasured
dimensions, not validated role classes, directional styles, tactical blame,
significant change points or lineup-fit estimates. Full TUI integration and
arbitrary game/date windows remain deferred. Sparse storage passed native WASM
crate tests; the browser acceptance above used the normal-storage WASM build.
