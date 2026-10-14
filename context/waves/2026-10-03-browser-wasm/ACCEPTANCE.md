# Browser WASM acceptance ledger

Updated: 2026-10-14. Requirement: REQ-BROWSER-001; work packages WP-BW-01..06.
This reconciles implemented features with the plan's remaining acceptance gates.
It does not declare the full plan complete.

## Current release

Public source is75e6ba9709e31e2cdeed633958f923f0ae36c01d (merged PR102).
Authenticated catalog gzip input now uses pull-based writes of at most8 KiB.
Canonical bytes, hashes, context, cancellation/deadlines, raw fallback, eight
residents and engine/schema remain unchanged. This bounds individual writes,
not all native decoder allocations or total browser memory.

All29 PR checks pass. Master/preview Git tree is identical
1bc42ba61dabcdb9b5509e50346bc9736473f717. Automatic master checks did not appear;
nonpublishing dispatch38571406174 succeeds, retaining artifact11897464859.
No native master run is claimed. Exact182 files/75 packages/27 shell assets
verify;179 non-metadata files match reviewed preview3b61f10e. Shell is
e575ec099d635615476610b46ec106b97e83d28b66fc3980bbdcbbf4292c1a36;
engine remains8482c9fd825f773eb3fc0956e42f67f0931caf871d2bab89c76b01a4738aab3a.
163 browser units and native/WASM parity pass in CI. Exact master corruption,
catalog-context, viewport/keyboard/AX, offline reopening and24-page/two-parser
PDF checks pass; every933 name/rank remains complete, with no out-of-page text.
Actual executing WASM fingerprints match; first/last PDF pages look readable.

Independent opposite-order Windows pairs show raw7.18/6.75 MiB lower peaks;
baselines differ3.11/3.34. One Linux-built CI pair shows392.25/394.16 MiB with
228.21/233.79 baselines; it is not independent opposite-order replication or
exact causal saving. Control102.92 ms warm maximum is retained. Exact master
387.24/389.67 MiB has228.74/229.77 baselines. First101.06 ms warm maximum remains
in evidence; repeat93.81 ms is a new observation, not a relabeled pass or waiver.
All75/eight/two905/332 imports/20 queries/restart preserve original flags/sampler/
network observer, no forced GC and every sampled phase.250 MiB still FAILS.

Retained-byte publication38574453158, rollback38575692853 to5ef015c4 and restore
38576162667 succeed without rebuild.273 served files match each completed phase;
all95 documentation hashes are identical. Rollback's first static read had one
HTTP503 (272 matches); the original failure remains, and a new check matches273.
The original owned profile completes consent-driven5ef->75->5ef->75 transitions,
preserving saved season/schedule bytes, revisions, metadata and settings. Gzip
905-player acquisition exports exact raw JSON without requesting its raw route.
Offline process reopening runs a fresh eight-player query/detail with uncached
probes blocked before/after debugger inspection. Executing decoder/module and
WASM hashes match every phase. The schedule remains a frozen fixture, not live
NHL proof. Physical Android remains explicitly deferred.

Evidence: chunked-decoder-input-ci/master/public-* and publication provenance
dated20261014. This is an incremental validation release. Full resource/mobile
acceptance remains incomplete.

## Historical5ef015c4 release

Public source is5ef015c4d47ebbcee8234eb92f1fe83716b45054, merged PR99's wasm32-only
repository stats allocation change. Large SeasonStats rows use unique Box
ownership behind smaller hash slots; native rows remain inline. Public borrowed
types, key shape, normalization, exports, saving policy and residency are unchanged.

PR/master each pass29 checks. Exact retained master artifact11885734369 from
browser38549103604/native38549103699 verifies182 files/75 packages/27 assets.
Shell is2cd7ec75fbdada2e037c0538b702286dff4a31419f383081bd29d82dc1efb02b;
WASM is8482c9fd825f773eb3fc0956e42f67f0931caf871d2bab89c76b01a4738aab3a.
Master modules/engine/catalog match the reviewed CI preview. Core1,182 tests,
native/wasm32 strict Clippy, repo-swap compile-fail,270 native/WASM parity cases
(33,613 rows),160 browser units, full viewport/keyboard/AX/print transitions,
corruption/exact export, catalog-context and process-close offline checks pass.
Actual24-page PDF preserves all933 names/ranks with two parsers and no clipping.

Local Windows-built opposite-order pairs save15.91/5.41 MiB raw private peak;
baselines remain explicit and do not establish universal savings. Linux-built CI
artifact peaks392.33/390.05 MiB against one shared392.86 control, not independent
opposite-order pairs. First CI warm maximum100.81 ms remains in evidence; repeat
is99.40 ms. Exact master full workload peaks395.54 MiB, blank237.10, warm max97.05
ms. All75/eight residents/two905/332 imports/20 queries/restart preserve original
flags/sampler/network observer, no forced GC and every sampled phase.250 MiB still
FAILS; no baseline subtraction or latency/budget waiver declares full acceptance.

Retained-byte publication38551551343, rollback38552043788 to38d2a053 and final
restore38552342417 succeed without rebuild. Each phase matches273 served files
with no failures; all95 staged documentation hashes match across phases. The same
original owned profile completes direct consent-driven38->5ef->38->5ef transitions
with saved season/schedule bytes, metadata/revisions/settings preserved. Gzip
acquisition exports exact905-player raw JSON; offline reopening runs fresh queries
and detail with uncached probes blocked, including after debugger attachment.
Debugger bytecode fingerprints prove the executing engine matches the retained
8482c9fd/2e4c8533 binaries on every online/offline phase. Additional inspector
observations are identity evidence, not normal memory acceptance.

Evidence: boxed-stats-ci/master/publication-* dated20261014. This is an incremental
validation release. Whole-browser250 MiB and physical Android gates remain open;
Android remains deferred by the user. Earlier sections are historical evidence.

## Historical38d2a053 release

Public source is38d2a053b0abc6b139b8f20718377c4df8a57a11, merged PR96's
digest-keyed catalog metadata projection. Whole JSON is authenticated and parsed
during build; known verified digests avoid reparsing player records on the UI
thread. Unknown digests retain the original fatal UTF-8/full JSON parser and
engine validation. No native API, data revision, saving policy or residency change.

PR and master each pass29 checks. Master browser38523902672/native38523902552
produce retained artifact11875720347:182 files,75 packages,27 shell assets.
Shell is3fcb6d9bc9e9b3cbf067aefded629932ced8ba1a5b1d1a19026eff3f0fdfebe2;
WASM remains2e4c853309a32d8366c282afe113626fde5c25b26349bd9194ceafe59123f57b.
Exact master catalog-context, viewport, keyboard, export, corrupt-gzip refusal,
offline and933-name/two-parser PDF checks pass. Typecheck/160 units pass.

Publication38525844284, rollback38526243290 toe58a3bfc and final restore
38526559371 all succeed with retained bytes, no rebuild. Served-file verification
matches273/272/273 files respectively, with no failures. All95 staged documentation
hashes match throughout. The same owned profile completes both consent-driven
directions, preserving saved season/schedule bytes, revisions, metadata and settings.
Online gzip acquisition exports exact original905-player JSON. Executing main,
decoder and new metadata module hashes match retained artifacts; offline process
reopening blocks uncached probes and runs a fresh eight-player query/detail.
Backed-up owned test-download history cleanup handles the known Chromium153
automation issue without changing app storage, user profiles or benchmark flags.

Actual master full-workload private peak is390.42 MiB, baseline226.52 MiB,
maximum warm query92.11 ms. All75 packages/eight residents/two imports/20 queries
and restart complete with every required phase sampled, original flags/sampler/
observer and no forced GC. Earlier opposite-order candidate pairs showed modest
2.69/5.18 MiB raw reductions with baseline variation; no universal saving is claimed.
The250 MiB gate remains FAILED. Physical Android remains deferred by the user.
Evidence: catalog-metadata-index-master-* and publication provenance dated20261014.
This is an incremental validation release, not full resource/mobile acceptance.

## Historical e58a3bfc release

PR93's catalog-context guard is merged as b29ba81c. Its 29 CI checks and exact
master artifact verification, six catalog refusal cases, viewport and offline
reopening pass. PR94's bounded gzip decoder is merged as e58a3bfc after 29 checks
and exact preview a019be7c browser/offline/refusal validation. Master browser run
38511436965 and native run 38511437021 pass: 29 successful checks. Retained
artifact11870427683 verifies181 files,75 packages and26 shell assets. Current
public source is e58a3bfc0f71fe7ba844cf729df166132eff7693, shell
466622feed4318b5ef58118be58cea1d8f91118accdd0ae42773a1ceada80269.
WASM remains2e4c853309a32d8366c282afe113626fde5c25b26349bd9194ceafe59123f57b.

Publication38513149144, rollback38513650760 to2e265236, and final restore
38514090019 all succeed using retained bytes, without rebuilding. Each public
phase matches272 served files (178 runtime/94 documentation), with no failures.
All95 staged documentation-file hashes match across the three phases; the
upload-tar guard includes hidden files. The same owned profile completes both
consent-driven directions, preserving the saved933-player season, frozen
two-game schedule, revisions, bytes, metadata and settings. Each online phase
loads the unsaved905-player package via gzip and exports exact original JSON.
Executing main and decoder module hashes match their retained artifacts.
Each process-close offline reopening performs a fresh eight-player query/detail
with uncached probes blocked, including after debugger attachment.

The owned-profile download-history workaround is backed up before each online
phase. Only its three test-download tables are cleared; all other history tables
and application storage remain unchanged. This handles the matching Chromium153
automation regression, not an app defect or memory-budget waiver. Complete
in-place transitions pass this time; no fresh profile substitutes for them.

Exact master full-workload private peak is397.09 MiB, blank baseline230.64 MiB,
maximum warm query99.19 ms. All75 packages, eight residents, two imports,
20 queries and restart complete with every required phase sampled, original
flags/sampler/observer and no forced GC. The250 MiB gate remains FAILED;
physical Android is deferred by the user. Actual master A4 PDF text verification
passes all933 ranks/names with two parsers,24 pages and no clipped text.
Evidence: stream-expansion-master-* and stream-expansion-publication-provenance.
This is an incremental validation release, not full resource/mobile acceptance.

The PR94 CI preview's full-workload private peak is 388.95 MiB versus retained
master control 406.71 MiB, with blank baselines 227.36/236.32 MiB. This additional
pair uses identical WASM/catalog bytes and unchanged flags, sampler and observer,
all 75 packages, eight residents, both imports, 20 queries and restart, no forced
GC. Local Windows pairs earlier observed 11.29/16.03 MiB reductions; this pair
observes 17.76 MiB. Baseline variation remains a limitation. The 250 MiB gate
still fails; Android remains deferred by the user.

The digest-view experiment is unadopted: one full-workload pair measured
399.05 MiB candidate versus 395.33 MiB control. Its code and reports remain on
the separate local experiment branch, not the merged implementation.

## Historical 2e265236 release

- Validation source: `2e265236f3e71fd2dbe6ecab74e64b296a553465`, merged PR #92.
  Includes PR #91 viewport rendering and authenticated gzip catalog delivery.
- Master browser build `38495299888` and native CI `38495299823` pass; PR and
  master each have 29 successful checks. Retained artifact `11863289024` verifies
  181 files, 75 packages and 26 shell assets. No publication rebuild.
- Shell: `9691890e23471a01accd6b907228e1d03eed268890039c5bb97061a9a26a8aad`.
  WASM: `2e4c853309a32d8366c282afe113626fde5c25b26349bd9194ceafe59123f57b`.
- Publication `38497514039`, rollback `38498071961` to4d1f513e, and final restore
  `38499320777` succeed. Final static verification matches272 served files:
  178 runtime and94 documentation. One initial documentation503 is preserved;
  a focused retry verifies the same file against unchanged preservation hashes.
- Public in-place first update and rollback preserve one saved933-player season,
  one frozen two-game schedule, revisions, bytes, metadata and settings. Executing
  main-module SHA verifies the rollback and final restored code. Public gzip read
  shows905 players and exports the original908625-byte JSON from118304 wire bytes.
- Final profile reopening, fresh eight-player query/detail and strict offline
  controls pass. Export in the reused profile hit a matching Chromium153/CDP
  download-history regression; failed attempts remain. A backed-up workaround
  clears only the owned test profile's three download tables, with other history
  tables and app storage unchanged. Final verification reopens already-restored
  2e265236 after those failures; it is not mislabeled a new in-place transition.
- Exact master full workload peaks388.74 MiB private allocations;20 warm queries
  have median70.45/max83.92 ms on this Windows workstation. The fixed250 MiB
  whole-browser gate FAILS. Physical Android remains explicitly deferred.
- Source/scope and raw reports: `evidence/gzip-delivery-publication-provenance-20261013.json`.
  This is an incremental validation release, not full resource/mobile acceptance.

## Historical e0741daa publication

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
| Exact release publication and documentation preservation | Master38495299888; publication38497514039; final restore38499320777; final static aggregate272 files, original503 retained | Current validation bytes/documentation verified; complete upload guard covers hidden files |
| Application update with saved data | Public one-tab4d1f513e→2e265236 update and2e265236→4d1f513e rollback preserve saved933-player season/frozen schedule; actual gzip execution/source SHA | Selected public path passes; not arbitrary user data or a new two-tab proof |
| Deployed offline reopening and saved query | gzip-delivery-public-final-offline-20261013.json: saved bytes restore, uncached probe fails, navigator offline, fresh eight-player filter/detail and executing-module SHA match | Current desktop path passes; physical mobile separate |
| Previous-version rollback and forward restore | Rollback38498071961 and restore38499320777; exact code/data hashes, final offline/export after backed-up automation-history workaround | Scoped public roundtrip verified; failed export-time browser closures retained and qualified |
| Whole-browser staging/resident peak memory | gzip-delivery-master-memory-20261013.json: all75 packages,8 residents,905/332 imports,20 committed queries/restart, original flags/sampler;388.74 MiB private peak | OPEN: fixed250 MiB target fails; no baseline subtraction, forced GC or workload reduction. Counters are not exact physical RAM |
| Physical mobile, touch/picker and latency | Desktop browser at narrow viewport | OPEN: real mobile browser/device measurements required |
| Compare/history/team/cards/fantasy | Plan WP-BW-07 requires separately scoped contracts and parity | Later waves; not implemented or implicitly accepted by the first slice |

## Remaining acceptance actions

1. Continue allocation optimization until the original250 MiB whole-browser
   target passes the complete workload. Current exact-master results387.24 and
   389.67 MiB fail acceptance; neither is a budget waiver. The earlier388.74 MiB
   value belongs to the historical2e265236 release.
2. Complete physical Android when the user connects the phone. Keep picker,
   touch, TalkBack, latency and device/resource scope separate from desktop proof.

### Warm-filter threshold audit (2026-10-14)

The governing plan section11.C specifies **500 ms** warm single-season filter
latency, separately from250 MiB desktop peak memory and10 MiB compressed starter
size. The recent100 ms comparisons are a stricter observation, not the plan's
acceptance threshold. No requirement is relaxed by this correction.
Exact published-master normal runs retain all20 samples: first maximum101.0567 ms
and repeat93.8081 ms. Both maxima are below the plan's500 ms target on this
workstation and workload. The original101.0567 ms observation remains above100 ms;
it is not deleted or relabeled. This supplies scoped desktop timing evidence,
not universal timing, device acceptance or a memory pass.
Sources: `design/plans/2026-10-03-browser-wasm-workbench.md` section11.C and
`evidence/chunked-decoder-input-master-memory-first-20261014.json` /
`evidence/chunked-decoder-input-master-memory-repeat-20261014.json`.

## Historical completed checks and earlier measurements

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

## Historical viewport candidate — superseded by current release

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

### Historical candidate print/AX follow-up

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

## Historical print-verifier correction

The earlier11 accented-name PDF mismatch came from reading the UTF-8 reference
using Windows CP1252, not from damaged app/PDF names. The original failed bytes
are retained as `.raw.cp1252`; the old JSON is transcoded to validUTF-8 and marked
superseded. Exact published 2e265236 PDF passes all933 ranks/names, including accents,
with two parsers,24 pages and no text outside bounds. The opt-in permanent
`scripts/verify-browser-print.py` reads/writes strictUTF-8. This is machine text
inspection, not physical-printer or screen-reader acceptance.
