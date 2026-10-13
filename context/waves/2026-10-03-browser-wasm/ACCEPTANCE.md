# Browser WASM acceptance ledger

Updated: 2026-10-13. Requirement: REQ-BROWSER-001; work packages WP-BW-01..06.
This reconciles implemented features with the plan's remaining acceptance gates.
It does not declare the full plan complete.

## Current release

- Validation source: `e0741daad56749cc3a3292e3f054b56d9f2605a2`; merged PR #89's deferred engine and attached-row renderer. This is a validation build, not full resource/mobile acceptance.
- Retained master build: `38457012844`; publication `38467985652`; rollback `38469050130` to previous 0e34ce88; final restore `38469247324`, all successful. Deployments reused exact reviewed bytes with the complete upload-tar guard.
- Shell: `273224d69d98ddb8f6d7ea7d6a861b408bc993db807c8c537ec7593133c7e497`.
- Retained clean artifact verified: 104 files, 75 public packages. Final static runtime/documentation verification matched 195 served files, including 94 documentation files, with no failures.
- Pages source: Actions workflow. `master` and `gh-pages` branch policies exist;
  the existing workflows publish from master and compose documentation from gh-pages.
- The earlier 696c656b publication's first attempt failed before deployment: master was excluded by the
  old gh-pages-only environment rule. Switching Pages to workflow mode added
  the master rule; retrying only the failed job succeeded.

The current release uses retained artifact `11845381015`. Linux WASM SHA-256 remains
`2e4c853309a32d8366c282afe113626fde5c25b26349bd9194ceafe59123f57b`.
Its actual-browser offline process-close check passes, including pending rows,
zero common-row reinsertions, detail, failed-query clearing and retry. See
`evidence/merged-optimization-ci-artifact-20261013.json` and its offline report.
The initial master build skipped deployment; publication and the final restore
are now independently verified on Pages. Public in-place update, rollback and
forward restore preserve saved season/schedule bytes, metadata, revisions and
settings. New/legacy renderer behavior is checked in addition to cached build-info
identity. An initial final-restore notice timeout is retained separately; the
same deployment passes the repeated in-place and offline checks. Cause is unproven.
See `evidence/public-optimization-publication-provenance-20261013.json`.

## Feature and gate disposition

The attached-row optimization is merged and published. Two early candidate
memory runs peak at 541.93/539.78 MiB; the strengthened query-commit runner's
existing-renderer control peaks at 548.02 MiB. The early readiness barrier alone
does not independently prove completion of every same-count warm query;
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
| Exact release publication and documentation preservation | Build 38457012844, publication 38467985652, restore 38469247324; deployed-optimization-release-20261013.json matches 195 served files | Selected current-release bytes/documentation verified; upload tar guard covers staged hidden files |
| Application update with saved data | Public one-tab consent-driven 0e34ce88 to e0741daa update; saved 933-player season plus two-game schedule fixture preserved; pending/render behavior proves new application code | Selected current public in-place update passes; two-tab consent passes separately on localhost |
| Deployed offline reopening and saved query | public-optimization-restore-offline-20261013.json: process closed/reopened offline; saved season/schedule restored, uncached probe fails, fresh eight-player filter/detail and new renderer behavior pass | Selected current desktop acceptance passes; earlier CI profile also verifies unsaved discard and 835-row filter; physical mobile remains separate |
| Previous-version rollback and forward restore | Rollback 38469050130 to 0e34ce88, restore 38469247324 to e0741daa; public browser/offline reports and exact stored hashes | Selected current in-place path passes both directions with saved bytes unchanged; earlier notice timeout retained, cause unproven |
| Whole-browser staging/resident peak memory | stable-handler-control-memory-20261013.json: existing renderer, all 75 packages, eight residents, 905/332-player imports, 20 queries with explicit commit barrier and restart; private peak 548.02 MiB | OPEN: retained 250 MiB target is exceeded; continue allocation investigation. Sampled private allocations and shared-page working-set sums are not exact concurrent physical-memory peaks |
| Physical mobile, touch/picker and latency | Desktop browser at narrow viewport | OPEN: real mobile browser/device measurements required |
| Compare/history/team/cards/fantasy | Plan WP-BW-07 requires separately scoped contracts and parity | Later waves; not implemented or implicitly accepted by the first slice |

## Completed checks and remaining acceptance actions

1. Desktop deployed offline reopening completed on 2026-10-12 using the opt-in
   `icelines-browser/e2e/offline-reopen.mjs` Playwright runner. The same check also
   passed against retained artifact 6016f77b with its local HTTP server stopped.
   This is desktop Chromium evidence, not physical-phone acceptance.
2. Public previous-version rollback and forward restore completed on 2026-10-13:
   current-release rollback 38469050130 and restore 38469247324 preserve exact
   saved season and schedule fixture bytes on the selected public in-place path.
   Final browser, offline and documentation verification pass. Local one/two-tab
   update checks also pass. Earlier incomplete notice probes remain recorded;
   the repeated final-restore check passes without a new deployment or code fix.
3. Continue whole-browser allocation optimization. The original isolated Windows
   process sampler covers every package phase, both 905/332-player archive imports,
   20 warm queries and restart. The corrected attached-row candidate shows repeated
   private peaks near 540–542 MiB versus controls near 570–571 MiB. A corrected
   readiness barrier now proves commit of each warm query and measures the
   existing renderer at 548.02 MiB. The original
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

## Local viewport candidate — not published (2026-10-13)

83da9b55 retains complete query results/exports while rendering a bounded viewport.
Actual browser traversal covers all 933 skaters, correct last-player detail, forward
and reverse keyboard windows/terminal boundaries and 77-goalie fallback/export
parity. A saved 933-player season survives process close/reopen offline; a fresh
835-player query and detail pass. Type checking and 149 unit tests pass.
Matching-control private peak is 547.10 MiB; final candidate runs are
446.23 / 442.02 MiB on the unchanged full workload and original sampler.
The 250 MiB requirement remains unmet. Narrow desktop layout is not Android proof.
Current public e0741daa release and its publication/rollback evidence are unchanged.
See viewport-provenance-20261013.json and MEMORY-ATTRIBUTION.md for limits and the
retained failed anchoring/height probes. No claim of full first-release readiness.

### Candidate print/AX follow-up

680a89cb adds actual print preparation/restoration and readable print-only layout.
All 933 player-button names are exposed in engine order in Chromium's AX tree.
An actual 24-page A4 PDF contains all 933 ranks and complete columns; sample pages
are visually reviewed. PDF extraction remains partial for 11 accented names,
with the cause unproven; its report deliberately remains verified:false.
The sampled accents render correctly, while JSON and AX retain exact names.
Physical printer/screen-reader/Android behavior is untested. Updated full-workload
private peak 447.03 MiB still fails 250 MiB. Actual browser regression,
149 unit tests, type checking and offline process reopen pass. See the retained
viewport-print evidence; current public release and full-goal gates are unchanged.

## Viewport source merged — publication pending

PR91 reviewed head58b6941f merged after 29 successful checks (deploy skipped)
as master4d1f513e29869bceba4fec9bf08d51511ad74207. Exact retained PR preview
e21a643e passed actual browser/AX/print preparation/keyboard/detail and offline
process-close checks; temporary PR artifact is not a master publication.
Master runs38482496646 (native CI) and38482496664 (browser artifact) are in progress.
Current public e0741daa release is unchanged. The rejected resident-list runtime
experiment is excluded; post-merge local runtime matches reviewed/merged source.
The 250 MiB target, physical Android and qualified PDF extraction gaps remain open.
