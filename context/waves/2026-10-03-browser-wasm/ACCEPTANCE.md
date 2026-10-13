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

The next reviewed publication candidate is merged master `e0741daa` (PR #89).
Its exact retained CI artifact from run `38457012844`, artifact `11845381015`,
verifies 104 files and 75 packages. Shell:
`273224d69d98ddb8f6d7ea7d6a861b408bc993db807c8c537ec7593133c7e497`;
Linux WASM SHA-256 remains
`2e4c853309a32d8366c282afe113626fde5c25b26349bd9194ceafe59123f57b`.
Its actual-browser offline process-close check passes, including pending rows,
zero common-row reinsertions, detail, failed-query clearing and retry. See
`evidence/merged-optimization-ci-artifact-20261013.json` and its offline report.
Deployment was skipped; a cache-busted public identity check still returns
0e34ce88. This is publication preparation, not a claim of deployed acceptance.

## Feature and gate disposition

The attached-row optimization is merged in master, separate from the current
public release. Two corrected candidate memory runs peak at 541.93/539.78 MiB;
the 250 MiB gate remains open. See MEMORY-ATTRIBUTION.md and attached-rows evidence
for matching controls and the real-browser offline/pending/error checks. Physical
Android testing remains deferred by the user.

The subsequent catalog-decoding and cleared-row-cache runtime experiments are
not adopted. Their functional/native/WASM checks pass, but the normal memory
comparisons establish no useful saving. They remain separate prototype branches;
the stable runtime still matches merged PR #89. Their evidence and review are
recorded separately in MEMORY-ATTRIBUTION.md and the catalog-context artifacts.

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
| Exact release publication and documentation preservation | Master artifact build 38434732162, publication 38441650974 and final restore 38450322271; deployed-validation-release-20261013.json matches 195 files | Selected release bytes and documentation verified; browser/offline proof recorded separately |
| Application update with saved data | Live two-tab update to 696c656b restored saved season and 49-game schedule; restored p >= 100 returned eight skaters | Selected deployed update passes |
| Deployed offline reopening and saved query | deployed-offline-reopen-20261012.json: isolated Chromium 153 process closed/reopened offline; saved 933-row season restored, unsaved second season absent, fresh 835-row filter and player detail pass; uncached network probe fails | Selected desktop acceptance passes; physical mobile remains separate |
| Previous-version rollback and forward restore | Public rollback run 38449905372 to 696c656b, forward restore 38450322271 to 0e34ce88; browser/offline reports and exact saved-record hashes | Selected close-and-reopen path passes with saved season/schedule bytes unchanged; in-place update activation remains separately incomplete |
| Whole-browser staging/resident peak memory | attached-rows candidate/control and repeat memory reports: all 75 packages, eight residents, regular/playoff imports, 20 queries and restart; candidate peaks 541.93/539.78 MiB, controls 570.14/570.70 MiB | OPEN: retained 250 MiB target is exceeded; continue allocation investigation. Sampled private allocations and shared-page working-set sums are not exact concurrent physical-memory peaks |
| Physical mobile, touch/picker and latency | Desktop browser at narrow viewport | OPEN: real mobile browser/device measurements required |
| Compare/history/team/cards/fantasy | Plan WP-BW-07 requires separately scoped contracts and parity | Later waves; not implemented or implicitly accepted by the first slice |

## Completed checks and remaining acceptance actions

1. Desktop deployed offline reopening completed on 2026-10-12 using the opt-in
   `icelines-browser/e2e/offline-reopen.mjs` Playwright runner. The same check also
   passed against retained artifact 6016f77b with its local HTTP server stopped.
   This is desktop Chromium evidence, not physical-phone acceptance.
2. Public previous-version rollback and forward restore completed on 2026-10-13:
   rollback 38449905372 and restore 38450322271 preserve exact saved season and
   schedule fixture bytes on the selected close-and-reopen path. Final browser,
   offline and documentation verification pass. In-place activation remains
   separately incomplete and needs investigation; no second public rollback is
   required merely to repeat the already verified close-and-reopen path.
3. Continue whole-browser allocation optimization. The original isolated Windows
   process sampler covers every package phase, both 905/332-player archive imports,
   20 warm queries and restart. The corrected attached-row candidate shows repeated
   private peaks near 540–542 MiB versus controls near 570–571 MiB. The original
   total 250 MiB target is retained. Baseline-subtracted or forced-GC measurements
   cannot establish acceptance, and memory growth alone does not establish a leak.
4. Physical Android testing is deferred by the user until the phone is connected.
   Then run the responsive/picker/keyboard flows and record mobile
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
when results shrink. The latest attached-row candidate avoids common-row
detach/reinsert operations and records reverse-order repeated measurements.
Forced collection and the observed savings do not close the whole-browser memory
gate or substitute for phone measurements.

Prepared rollback selection is recorded in
`evidence/prepared-rollback-20261008.json`: previous artifact 0caaeaeb and current
artifact 696c656b both pass the retained-artifact verifier. Their local composed
outputs preserve identical inventories of all 95 documentation files. This is
historical preparation. The authorized public rehearsal and final restore are
now recorded in `evidence/public-rehearsal-provenance-20261013.json`; approval is
no longer pending for that completed rehearsal.
