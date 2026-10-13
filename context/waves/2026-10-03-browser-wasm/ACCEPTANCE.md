# Browser WASM acceptance ledger

Updated: 2026-10-13. Requirement: REQ-BROWSER-001; work packages WP-BW-01..06.
This reconciles implemented features with the plan's remaining acceptance gates.
It does not declare the full plan complete.

## Current release

- Validation source: `0e34ce88fb4c9465d66c40821b94658aa19cf722`; includes the merged renderer and dependency updates. The deferred-engine candidate is not deployed.
- Retained master build: `38434732162`; publication `38441650974`; final restore `38450322271`, all successful. Deployment used the reviewed retained bytes and the master workflow with PR #88's archive guard.
- Shell: `058529d3026d453cb2fa8a517f9c2d9e815a4883ca48a7034a22507df6209c02`.
- Retained clean artifact verified: 104 files, 75 public packages. Static runtime/documentation verification matched 195 files after retrying one transient HTTP 503.
- Pages source: Actions workflow. `master` and `gh-pages` branch policies exist;
  the existing workflows publish from master and compose documentation from gh-pages.
- The earlier 696c656b publication's first attempt failed before deployment: master was excluded by the
  old gh-pages-only environment rule. Switching Pages to workflow mode added
  the master rule; retrying only the failed job succeeded.

## Feature and gate disposition

The attached-row optimization is a local candidate, separate from the current
public release. Two corrected candidate memory runs peak at 541.93/539.78 MiB;
the 250 MiB gate remains open. See MEMORY-ATTRIBUTION.md and attached-rows evidence
for matching controls and the real-browser offline/pending/error checks. Physical
Android testing remains deferred by the user.

| Plan requirement | Evidence | Disposition |
|---|---|---|
| Shared Rust normalization and worker-owned WASM | Native/WASM corpus, distributed-WASM verifier, CI browser build | Implemented; scoped parity corpus passes |
| Catalog, regular/playoff selection, historical packages | Clean retained artifact has 75 packages; CI loads every package in actual WASM | Implemented |
| Skater/goalie filters, sort, sample floors and player details | Distributed count-order goldens, native parity corpus, deployed player/goalie walkthroughs | Implemented |
| Session memory versus explicit saving | Library transaction/conflict tests and deployed save/reload plus unsaved-data discard observations | Implemented |
| Imports, archive bounds and JSON/CSV/package export | Resource preflight and archive fixtures; actual file-picker and downloaded-byte evidence in deployed-file-roundtrip-20261006.json | Implemented; phone picker latency unmeasured |
| Live stats/schedule and bounded relay | Cloudflare relay browser refresh evidence; PR #79 stable ordering/coverage guards; 940-skater and eight-leader observations | Implemented; source observation timestamps remain explicitly unknown where upstream omits them |
| Polling, cancellation, previous-good preservation | Polling/acquisition tests and deployed cancellation evidence | Implemented |
| Responsive UI, keyboard, share routes and source disclosures | Selected 360px/desktop tests and deployed export/view checks | Partial acceptance; not a physical-phone or full assistive-technology claim |
| Appearance choices | Night Game, Fresh Ice, Hockey Club; live switching, preference reload and cross-tab observations | Implemented |
| Exact release publication and documentation preservation | Retained artifact, deployment run and deployed-release-20261008.json | Static-byte evidence; not browser/offline proof |
| Application update with saved data | Live two-tab update to 696c656b restored saved season and 49-game schedule; restored p >= 100 returned eight skaters | Selected deployed update passes |
| Deployed offline reopening and saved query | deployed-offline-reopen-20261012.json: isolated Chromium 153 process closed/reopened offline; saved 933-row season restored, unsaved second season absent, fresh 835-row filter and player detail pass; uncached network probe fails | Selected desktop acceptance passes; physical mobile remains separate |
| Previous-version rollback and forward restore | Public rollback run 38449905372 to 696c656b, forward restore 38450322271 to 0e34ce88; browser/offline reports and exact saved-record hashes | Selected close-and-reopen path passes with saved season/schedule bytes unchanged; in-place update activation remains separately incomplete |
| Whole-browser staging/resident peak memory | desktop-process-memory-20261012.json: Windows process-tree samples across all 75 packages, eight residents, regular/playoff imports and warm queries; 590.81 MiB sampled private bytes, 900.45 MiB summed working set, 230.92 MiB blank-browser private baseline | OPEN: provisional 250 MiB target is exceeded; investigate allocations and validate representative devices. Sampled private allocations and shared-page working-set sums are not exact concurrent physical-memory peaks |
| Physical mobile, touch/picker and latency | Desktop browser at narrow viewport | OPEN: real mobile browser/device measurements required |
| Compare/history/team/cards/fantasy | Plan WP-BW-07 requires separately scoped contracts and parity | Later waves; not implemented or implicitly accepted by the first slice |

## Next acceptance actions

1. Desktop deployed offline reopening completed on 2026-10-12 using the opt-in
   `icelines-browser/e2e/offline-reopen.mjs` Playwright runner. The same check also
   passed against retained artifact 6016f77b with its local HTTP server stopped.
   This is desktop Chromium evidence, not physical-phone acceptance.
2. Rehearse a retained previous-good release through `browser-rollback.yml`, then
   restore this exact release using its retained artifact. Compare saved-package
   fingerprints and current documentation before/after. This temporarily changes
   the public site and is not inferred from a local preview rehearsal.
3. Measure browser-process/staging peak separately from WASM memory, with an
   isolated profile and attributed processes. Run the all-package/resident/import
   cases and warm-query measurements; disclose baseline and instrument limitations.
   The 2026-10-12 Windows headless Chromium run now supplies process counters:
   all 75 package phases have samples, median sampling interval 185 ms and maximum
   interval 755 ms. Regular/playoff imports returned 905/332 players; the slowest
   of 20 warm query/render samples was 130.77 ms. The private-allocation peak was
   359.89 MiB above the blank-browser peak. These observations exceed the current
   total-memory target. Do not reinterpret the baseline-subtracted number as a
   passing total-memory budget or claim a leak without allocation evidence.
4. Run the responsive/picker/keyboard flows on a physical phone. Record mobile
   engine, device and timing; do not substitute a desktop viewport capture.
   The executable manual sequence and instrument options are in
   `PHONE-ACCEPTANCE.md`; its status is not run until real-device evidence exists.

The earlier ChatGPT UI browser controller was unavailable. On 2026-10-12 the
VS Code terminal successfully ran a repository-owned, pinned Playwright browser
in an isolated persistent profile, including a new process with offline network
control. The Windows computer-use helper is unnecessary for this test runner.
Windows process-tree instrumentation is now available through
`npm run measure:memory`. The memory-budget disposition and physical-phone
evidence remain open. Measurement completion is distinct from target acceptance.

The allocation follow-up is recorded in `MEMORY-ATTRIBUTION.md` with separate
control, prototype and corrected-renderer evidence. WASM linear memory was
23.88 MiB; forced page GC exposed collectable renderer allocations. The first
row-reuse prototype retained spare DOM rows and was corrected to release them
when results shrink. The corrected pool returns to approximately the control's
post-GC node count. Forced collection and the modest single-run differences do
not close the whole-browser memory gate or substitute for phone measurements.

Prepared rollback selection is recorded in
`evidence/prepared-rollback-20261008.json`: previous artifact 0caaeaeb and current
artifact 696c656b both pass the retained-artifact verifier. Their local composed
outputs preserve identical inventories of all 95 documentation files. This is
preparation, not a remote rehearsal; public rollback approval remains pending.
