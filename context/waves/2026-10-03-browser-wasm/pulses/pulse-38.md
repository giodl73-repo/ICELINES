# Browser WASM pulse 38 — isolated review checkpoint

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-06. Previous turn made verified
composition progress. Goal remains active.

Ran `node scripts/build-browser.mjs --offline --check` in the original checkout:
completed successfully. Fresh WASM/catalog verification passed 22 shell assets,
75 packages, four count goldens and residency. Native/distributed-WASM parity
passed 270 cases, 33,613 full rows and 90 empty results. Affected Rust suites
(including 12 WASM tests), 18 Python tests and 128 browser tests passed. Shell
identity remains `c6ecb4253273cb8b97a560f70359d68a6869dcd7ee12ea246461dba1eac3943a`;
app plus largest package gzip remains 404,795 bytes. Log:
`C:/src/icelines/target/browser-pulse-38-check.txt`.

Created/attached a managed worktree at
`C:/Users/giodl/.codex/worktrees/browser-wasm/icelines`, from verified base
`935136020140bd5b408d26cbb0777dd6f0fb5ef9` (same HEAD/origin/master).
Created local review branch `codex/browser-wasm-workbench`. Copied 187 selected
files (6,527,917 bytes), with a [path/digest inventory](browser-pulse-38-checkpoint.json).
The original checkout's branch/index/dirty native work was preserved.

Included shared portable loading, count ordering/goalie-floor and schedule
boundaries, browser code/build/workflows and corresponding docs/tests/evidence.
Excluded unrelated fantasy-market, player-fit, game, shot-hands, CLI and native
template/style changes. The copied Cargo.lock retains exact dependency pins,
including resolver differences; these still need review rather than silently
claiming unrelated dependency churn absent. Generated data/bindings/dependencies
were excluded from the source transfer.

Independent checks are underway in the worktree. Its WASM was compiled from the
isolated source, catalog built and distribution verified with the same final
shell identity/size; `cargo fmt --all -- --check` passed. Native/parity/check
portion is still running in terminal handle **27828**, confirmed live this turn,
with log `target/browser-pulse-38-isolated-check.txt`. No complete isolated
pipeline pass or clean commit is claimed yet.

Initial npm installation was denied by local permissions, then the normal
offline cache lacked a package. Approved tool execution installed the two
locked dependencies from existing local tarballs using a workspace cache;
two packages installed with no reported vulnerabilities. No upstream live data
or remote publication was involved. The locally pinned wasm-bindgen executable
was copied as an ignored build tool, not included in the review source.

Next: finish the same independent check handle, fix any failures in the isolated
WASM slice, review/stage/commit that slice, then obtain a clean retained build
before remote validation deployment. Physical devices/resources, complete state
matrix, deployed live/offline/update and remote rollback remain open. Relay
hosting stays deferred until deployed-origin testing. No remote mutation ran.
