# Desktop allocation investigation — 2026-10-12

The provisional 250 MiB whole-browser target remains exceeded. Measurements
below are Windows private allocations and summed working sets, not exact
concurrent physical RAM, phone measurements, or acceptance waivers.

## Diagnostic control

`evidence/desktop-memory-attribution-20261012.json` records retained artifact
6016f77b with all 75 packages, eight resident windows, regular/playoff archive
imports and 20 warm queries. The runner was under development during this
capture; its diagnostic implementation was subsequently committed in 6851783d
with an earlier phase boundary to exclude inspector setup from normal samples.

WASM linear memory stayed at 25,034,752 bytes (23.88 MiB). The page's DOM count
fell from 11,311 to 6,103 after forced page GC; private allocations fell from
602,492,928 to 455,667,712 bytes across the corresponding settled phase samples
(140.02 MiB). Worker collection changed allocations much less. This identifies
collectable renderer state as a useful investigation area; it does not establish
a Rust leak or prove that every retired node accounts for the allocation drop.

Forced GC and debugger inspection are diagnostic interventions. No post-GC
sample, including subsequent engine restart, is used to claim that the normal
workload passes its memory target. The production app does not force collection.

## Row reuse experiment

Prototype 6851783d reuses up to 1,024 row node sets across queries, erasing spare
names and callbacks. It still displays every result, releases its pool on unload
and engine recovery, and replaces column shape when changing player kind.

| Normal run | Control 4f13fd70 | Candidate 6851783d |
|---|---:|---:|
| Sampled peak private allocations | 596.61 MiB | 571.31 MiB |
| Blank browser peak private allocations | 242.43 MiB | 231.13 MiB |
| Peak minus blank baseline (diagnostic comparison only) | 354.19 MiB | 340.19 MiB |
| Summed working-set peak | 901.69 MiB | 871.18 MiB |
| Slowest warm query/render | 100.69 ms | 105.55 ms |

Evidence: `desktop-memory-control-20261012.json` and
`desktop-memory-row-reuse-20261012.json`. Both use identical Windows-built WASM
SHA-256 `5b407c14f0a727855114091d66c5ddf363659c36aaa308bf1fc3950c841253de`,
all 75 packages, the same public archive, browser version and measurement runner.
The control distribution was built from a clean detached 4f13fd70 checkout.
Its build metadata records that commit; it was not relabeled as the candidate.

These are single sequential runs, not a statistical performance result. The
25.30 MiB total difference includes an 11.30 MiB baseline difference; warm-query
timings do not demonstrate a speed improvement. Neither run passes 250 MiB.
Do not describe the experiment as solving the memory gate.

Candidate offline reopening also passed: 933 saved players restored after closing
the browser and stopping the server, the unsaved second season was absent, a
fresh offline filter returned 835 players, and Connor McDavid detail opened.
See `row-reuse-offline-reopen-20261012.json`. Its local OS profile path is omitted
from public evidence. This does not establish real-phone or public rollback
acceptance.

The prototype is superseded: `desktop-row-reuse-attribution-20261012.json`
shows 20,737 DOM nodes before collection and 19,140 afterward, versus 6,103
post-collection nodes in the control. It kept unused nodes from larger seasons.
Commit aeda6df5 trims the pool to the current result size and clears callbacks
on every displayed row, including rows beyond the reusable pool. Its tests also
verify that spare rows are released when results shrink. Measurement of that
correction is separate from the prototype results above.

## Corrected pool ownership

The clean 4d90de8b artifact includes the aeda6df5 correction and the experiment
notes, with the same WASM binary as the Windows control. Its diagnostic capture
(`desktop-trimmed-row-attribution-20261012.json`) shows 10,193 DOM nodes before
collection and 6,108 afterward. The pool no longer holds the larger season's
spare rows. WASM linear memory remains 23.88 MiB. Warm-query median (upper middle
of 20 samples) was 67.30 ms; the matching control was 68.25 ms. This does not
demonstrate a material latency improvement.

The pre-intervention private-allocation peak was 575.26 MiB, with a 226.42 MiB
blank-browser peak. These diagnostic-mode numbers exclude debugger setup,
forced GC and post-GC restart from target comparisons. The corrected renderer
prevents repeat-query node replacement and excess pool retention, but these
observations do not establish a significant whole-browser memory improvement.

The normal corrected-artifact run, without debugger attachment or forced GC,
completed every package phase, both archive imports, 20 warm queries and engine
restart (`desktop-trimmed-row-memory-20261012.json`). Peak private allocations
were 573.44 MiB, summed working sets 881.21 MiB, and blank private allocations
233.02 MiB. Peak-minus-blank was 340.41 MiB, versus the control's 354.19 MiB;
this remains a diagnostic comparison, not a changed target definition. Warm
median was 68.63 ms and maximum 95.32 ms; regular/playoff imports returned
905/332 players. No sampled phase was missing. The 250 MiB target still fails.

The corrected artifact also passed the full offline close/reopen sequence
(`trimmed-row-offline-reopen-20261012.json`): saved 933-row season restored,
unsaved season absent, fresh 835-row query and player detail passed with the
server stopped and uncached network probe failing. All 140 browser unit tests
passed. Actual WASM distribution verification covered 75 packages and the
eight-window residency bound. Physical-phone and public rollback gates remain
open; the public app has not been republished for this experiment.

## Deferred engine startup (2026-10-13)

The page previously created its worker immediately and awaited an unused
current-season request before opening the library. The candidate creates the
worker on the first data operation. Explicit restart/recovery still starts it
immediately; initial creation preserves the caller's selected context generation.
Saved stats and saved schedules automatically start it when restoration needs Rust.

Control: clean master 405cfe68. Candidate: clean 393b9c2c. Both Windows-built
artifacts use WASM SHA-256
`4f99584fe3147f05b525dae35c2ac2a49ca23aaa3f84fd961e5cecb9a43f0eee`.
This is distinct from the retained Linux CI binary; no Rust or dependency changes
are part of this optimization.

| Settled empty-library run | Blank browser private MiB | Idle private MiB | Idle minus blank, diagnostic only | Engine workers |
|---|---:|---:|---:|---:|
| Control 1 | 234.31 | 287.58 | 53.27 | 1 |
| Candidate 1 | 230.64 | 275.27 | 44.64 | 0 |
| Candidate 2 | 227.49 | 272.80 | 45.30 | 0 |
| Control 2 | 230.90 | 290.18 | 59.28 | 1 |

The startup-only probe waits for application readiness, completed offline-shell
preparation and five idle seconds. Worker inventory is collected afterward, in a
diagnostic phase. These sequential runs suggest a modest idle saving; neither
total nor baseline-subtracted numbers redefine the 250 MiB whole-browser target.
The standard runner's early startup boundary alone was inconclusive because the
blank-browser baselines varied and it did not wait for full application readiness.

The full workload still fails: control 572.34 MiB private peak and candidate
566.14 MiB, with blank peaks 245.56/228.30 MiB. The baseline difference exceeds
the peak difference, so this does not establish a full-workload memory saving.
Both load all 75 packages, retain at most eight windows, import 905/332 players,
run 20 warm queries and restart. Warm medians were 68.47/69.41 ms and maxima
86.76/101.11 ms; no speed improvement is claimed. A separate public browser
verification overlapped part of the control run, further limiting timing claims.

All 144 browser unit tests and TypeScript checks pass. Candidate offline reopening
restores the saved season, excludes the unsaved season, runs a fresh filter and
opens player detail. A second actual-browser test starts with zero engine workers,
imports/projects a two-game schedule as its first engine operation, observes one
worker, saves it without a stats dataset and restores it offline after process
closure. Evidence files are prefixed demand-engine in this directory.

The user explicitly retained the 250 MiB target and deferred physical Android
testing. This change is an incremental startup optimization, not full resource
acceptance. No forced GC or debugger-buffer change was used in these candidate
and control workload measurements.

## Attached rows across queries (2026-10-13)

The deferred-engine build 393b9c2c is the control for this next experiment.
Corrected candidate 653c876b retains common table rows in the document, updates
changed text in place and only appends/removes the changed tail. During pending
queries, visibility:hidden and inert hide stale results without discarding their
layout. Current query failures still clear the table. Reused focused buttons blur
when their player identity changes. All rows remain available, including results
beyond the 1024-node reuse pool; query/export/domain semantics are unchanged.

The first implementation was ineffective because runQuery still called
clear(false). The actual offline MutationObserver caught 835 added and 933 removed
rows despite reused node identities. Those early memory runs do not exercise the
intended intervention. The caller was corrected before the measurements below.
See attached-rows-provenance-20261013.json for the rejected candidate boundary.

Both clean artifacts share Windows WASM SHA-256
`4f99584fe3147f05b525dae35c2ac2a49ca23aaa3f84fd961e5cecb9a43f0eee`.
The original OS sampler covers all 75 packages, eight resident windows, imports
of 905/332 players, 20 warm queries and engine restart. There is no forced GC,
debugger intervention or browser-flag change. Runs are sequential; the repeat
pair reverses candidate/control order.

| Run | Blank private MiB | Full private peak MiB | Warm median ms (upper middle) | Warm maximum ms |
|---|---:|---:|---:|---:|
| Control 1 | 233.22 | 570.14 | 67.93 | 87.65 |
| Candidate 1 | 228.76 | 541.93 | 56.98 | 122.28 |
| Candidate 2 | 230.23 | 539.78 | 50.35 | 60.67 |
| Control 2 | 232.91 | 570.70 | 68.04 | 87.62 |

The two comparisons observe approximately 28–31 MiB lower private peaks. Blank
baselines differ by approximately 3–4 MiB, smaller than the observed savings.
These are observed Windows process-counter results, not a general speed claim.
Candidate 1's maximum query time is worse than the control. The 250 MiB total
limit still fails; baseline subtraction does not redefine it. Evidence files
are prefixed attached-rows in this directory's evidence/ folder.

TypeScript and all 147 browser unit tests pass. The committed opt-in browser
check can be reproduced with:

```powershell
node icelines-browser/e2e/offline-reopen.mjs --artifact target/wasm-attached-rows-corrected --verify-attached-rows
```

It closes the process and stops the server, restores a saved 933-player season
offline, verifies pending rows are hidden/inert and still attached, then filters
to 835 rows with zero added rows and exactly 98 removed rows. Player detail,
malformed-query clearing and a successful 835-row retry pass. This browser
inspection is separate from normal memory measurements. The candidate is not
deployed; physical Android remains deferred by the user.

## Table layout versus idle collection diagnostic (2026-10-13)

An ad-hoc direct-CDP controller ran the corrected 653c876b artifact through all
75 packages, both archive imports, warm queries and restart in two fresh profiles.
It uses captured Chromium flags with a fresh profile and loopback-port transport,
without Playwright's injected script or Network observer. It is a diagnostic
method, not a substitute normal benchmark; its lower peaks do not revise the
production-runner results or establish acceptance. No forced GC was requested.
Sources and capture hashes are recorded in table-layout-diagnostic-provenance.

After normal queries, the treatment collected counters, waited five seconds,
hid the existing table with display:none for five seconds, then restored it and
repeated the query. The matched idle control left the table displayed throughout.
All three snapshots in both runs retain exactly 280 active rows.

| Diagnostic | Initial DOM nodes | Middle-interval DOM nodes | Initial embedder MiB | Middle-interval embedder MiB | Private MiB after first / middle / final interval |
|---|---:|---:|---:|---:|---|
| Hide table layout | 20,201 | 6,089 | 61.19 | 3.08 | 435.86 / 367.29 / 363.63 |
| Leave table displayed | 20,013 | 6,089 | 58.47 | 10.19 | 435.54 / 373.61 / 391.10 |

The idle control also drops approximately 14,000 DOM nodes and substantial
allocations without hiding the table. The large reduction therefore cannot be
attributed solely to table layout or used to justify pagination as its solution.
Natural collection and debugger inspection effects remain possible. Retired
presentation allocations across repeated large/small season loads are the next
useful investigation area; this pair does not identify a leak or an exact layout
cost. Raw diagnostic_table_hidden is inherited phase nomenclature in the idle
report and does not mean that its table was hidden. The 250 MiB gate stays open.

## Cleared row cache experiment — not adopted (2026-10-13)

Prototype 18fcb68a retained up to 1024 cleared row shells when a large regular
season was followed by a smaller playoff/filter result. Detached names, IDs and
callbacks were cleared; unload and column transitions released the pool. This
tested whether avoiding repeated tail-node creation outweighs bounded retention.
Control was the existing 653c876b renderer that trims spare entries. Both clean
Windows artifacts share the same WASM bytes. The normal OS-process sampler ran
all 75 packages, both archive imports, 20 warm queries and restart, sequentially
candidate then control, without debugger intervention or forced GC.

| Run | Blank private MiB | Full private peak MiB | Warm median ms (upper middle) | Warm maximum ms |
|---|---:|---:|---:|---:|
| Cache prototype | 234.07 | 537.18 | 50.09 | 66.16 |
| Fresh trimming control | 226.30 | 529.66 | 51.49 | 67.38 |

The total difference is approximately the blank-baseline difference. This pair
does not establish meaningful peak savings or a speed improvement. No further
repeat was warranted to promote the cache; it is **not adopted**. The stable
source and PR #89 still trim spare pool entries. All 147 unit tests pass for the
prototype. Its separate actual-browser check confirms 28 cleared tail nodes,
reused nodes after regrowth, correct tail-player detail and full offline,
pending-query, failure-clearing and retry behavior. Functional correctness does
not establish a memory benefit. Evidence is prefixed cleared-row-cache.

The next candidate is eliminating the redundant full catalog-package JSON
decode on the main thread. Catalog context still must be checked before the
engine replaces or selects active data; removing that guard is not an acceptable
optimization. This is a hypothesis for the next experiment, not measured savings.
The 250 MiB target remains exceeded and unchanged.

## Catalog decoding and interaction experiment — not adopted (2026-10-13)

Prototype 7e069abe replaces the page's full catalog-package JSON decode with a
small expected-context payload. Rust validates season/type/source before any
fresh replacement or cached selection, then returns its validated metadata.
Generic imports remain supported. The page's cached shortcut also checks its
already validated metadata. Combined prototype 47e7ab29 adds the cleared-row
cache described above. Neither runtime experiment is adopted into stable code.

| Run (sequential) | Blank private MiB | Full private peak MiB | Warm median ms (upper middle) | Warm maximum ms |
|---|---:|---:|---:|---:|
| Catalog prototype | 229.72 | 568.04 | 50.54 | 62.86 |
| Fresh existing-renderer control | 235.50 | 551.60 | 50.98 | 62.68 |
| Catalog plus cleared-row cache | 225.86 | 549.45 | 51.48 | 66.95 |

The original process sampler covers all 75 packages, eight residents, both
905/332-player archive imports, 20 warm queries and engine restart. No debugger
intervention, browser-flag change or forced GC was used. One 120 ms targeted
unit-test invocation overlapped an early standalone package phase; timing
claims are not made. The combined run's blank baseline is substantially lower
than the control's, so its slightly lower total is not evidence of a useful
saving. The standalone does not improve the observed peak. This set supplies no
basis for promoting either prototype or claiming a general speed improvement.

Candidate/combined Windows WASM SHA-256 is
`d91f75da1725094ead601aa1024caecf83a7da737c18251e0e9d9996e6356d58`,
different from the existing-renderer control's
`4f99584fe3147f05b525dae35c2ac2a49ca23aaa3f84fd961e5cecb9a43f0eee`.
The new context-validation binding intentionally changes the Rust/WASM adapter;
these are not identical-binary comparisons. Domain projection code, data and
dependencies are unchanged. The standalone starter gzip sum is 413,337 bytes
versus the prior 410,619-byte local control, still within the separate 10 MiB gate.

All 20 native Rust tests and 148 browser unit tests pass for the prototypes.
Actual-WASM distribution verification loads all 75 packages through the new
binding, checks generic-import revision parity and rejects a mismatched resident
context without reordering or changing active selection. The actual browser
fault injection rejects a wrong source while retaining the saved 933-player
table and one resident window; a subsequent 835-player query and detail pass.
Separate offline process-close, pending/error/retry and combined tail-row reuse
checks pass. These functional checks do not establish a memory saving. Raw
evidence is prefixed catalog-context; prototype source branches remain separate.

PR #89's reviewed head a7a57dc4 passed 29 CI checks, with deployment skipped,
and was squash-merged as e0741daa. That merge contains the previously measured
deferred startup and attached-row improvements, not these new prototypes.
At that capture, the public site was the separately verified 0e34ce88 release;
the later publication/rollback/restore evidence verifies current e0741daa. The 250 MiB
whole-browser target remains exceeded; physical Android is still deferred.

## Stable handler experiment and query-completion correction (2026-10-13)

Prototype dc39dd31 gives each attached player button one handler that reads its
current presentation record. Removed and cleared rows lose both the handler and
record reference. Pool trimming, full results, focus identity, headers and domain
projection are unchanged. The hypothesis was lower listener/closure retirement
across repeated queries. This runtime prototype is **not adopted**.

During verification, the normal runner's row-count/enabled-button barrier was
found insufficient for attached rows: the old count remains while a request is
pending, and the submit button stays enabled. The retained correction additionally
waits for the existing tbody.inert pending marker to clear before proceeding.
It preserves the original workload, OS sampler, browser flags and 250 MiB target.
Earlier attached-row report completion/timing claims used the weaker barrier;
they must not independently establish that every same-count query had committed.
The corrected pair below explicitly waits for each of the 20 warm queries.

| Corrected run | Blank private MiB | Full private peak MiB |
|---|---:|---:|
| Stable-handler prototype | 227.93 | 548.75 |
| Existing-renderer control | 231.02 | 548.02 |

Both complete all 75 package phases, eight-window residency checks, 905/332-player
imports and restart without forced GC or debugger intervention. Windows WASM
bytes match at 4f99584f. There is no useful whole-browser saving established by
this pair and no general speed claim. The 250 MiB target still fails.

The prototype passes TypeScript and 148 unit tests, including updated-record
selection and inability of an externally retained handler to open cleared data.
An actual process-close offline test restores 933 players, filters to 835, changes
goal sorting while retaining the same handler functions, and opens the correct
reordered player's detail. Failure clearing and retry also pass. These functional
results are separate from memory measurements. Evidence is prefixed stable-handler.
Stable application source remains merged PR #89; only the benchmark completion
correction is retained. Phone testing remains open. A later current-release
public in-place update, rollback and forward restore pass with unchanged saved
bytes; see public-optimization-publication-provenance-20261013.json. Earlier
incomplete notice waits remain separately recorded and their cause is unproven.

## Pending opacity experiment — not adopted (2026-10-13)

Prototype 8fc3ee9a tests opacity:0 plus inert during pending queries instead of
visibility:hidden plus inert. CSS defines opacity as non-inherited and visibility
as inherited ([CSS Color](https://www.w3.org/TR/css-color-4/#transparency),
[CSS Display](https://www.w3.org/TR/css-display-3/#visibility)). Avoiding descendant
restyling was a hypothesis, not an established performance property.

The original process sampler and strengthened commit barrier complete all 75
packages, eight-window residency, both 905/332-player imports, 20 warm queries and
restart. Matching Windows WASM bytes remain 4f99584f; no forced GC or debugger
intervention. Candidate private peak is 545.82 MiB with 227.37 MiB blank peak;
fresh existing-renderer control is 544.31 MiB with 230.95 MiB blank peak. This
pair establishes no useful total-memory saving. The prototype is **not adopted**.

TypeScript and 147 unit tests pass. A separate actual-browser offline check
restores 933 players, verifies pending computed opacity zero and inert true,
filters to 835 without readding common rows, opens detail, clears failed-query
rows and retries. Functional correctness does not prove a memory benefit.
Evidence is prefixed pending-opacity. The stable source and public e0741daa
release retain visibility:hidden. The 250 MiB target is unchanged and unmet.

## Raw package storage representation sizing (2026-10-13)

The engine retains typed imported DTOs alongside each normalized repository,
partly to support its existing export binding. Before replacing DTOs with
serialized canonical export bytes, the diagnostic native example compares their
storage shapes across all 75 retained CI packages:

```powershell
cargo +1.95.0 run -p icelines-wasm --example package_storage_sizes --locked --offline -- target/wasm-validation-e0741daa
```

Canonical JSON totals 34.66 MiB across the corpus; native DTO lower bounds total
28.68 MiB. Canonical bytes exceed the lower bound in 74 of 75 packages. The first
933-player package is 771,060 canonical bytes versus a 543,631-byte lower bound.
This does not establish a saving from serialized JSON storage; no representation
change is adopted. These are sums over the entire corpus, not simultaneous
resident allocations. The current eight-window bound remains unchanged.

The lower bound includes native vector capacities/inline structs and UTF-8 value
string lengths, excluding spare String capacity and allocator bookkeeping.
Pointer width is eight bytes. Native layouts are not WASM32 layouts, and neither
number measures browser/private/linear-memory peaks. The diagnostic deliberately
does not claim budget acceptance or an exact resident-heap comparison. Raw report:
package-storage-sizes-native-20261013.json. Domain code and export behavior remain
unchanged; the 250 MiB whole-browser requirement stays open.

## Blank-browser target attribution

`blank-browser-targets-20261012.json` records a fresh isolated Chrome 153 process
with no IceLines page loaded. Its CDP target inventory includes `about:blank`
and two `browser_ui` targets for Chrome's omnibox popup. OS and CDP snapshots
show three renderer processes plus browser, GPU, utilities and a crashpad helper.
This helps explain why the blank baseline is substantial. The snapshots are
sequential and do not map target URLs to individual renderer PIDs; do not assign
a particular renderer's bytes to the address bar without further evidence.
Browser UI and GPU allocations remain included in whole-browser measurements.
This small debugger-attached snapshot is attribution evidence, not a substitute
for the full normal workload or a rationale to relabel the 250 MiB target.

## Exact PR CI artifact

Completed browser run 38288081921 produced artifact 11765060462 at temporary
PR merge commit bc337886 (parents: master 4f13fd70 and reviewed head 2cf0f0d6).
`ci-review-artifact-20261012.json` records that provenance, exact shell/WASM
hashes, 104 verified files and 75 verified packages. This is a review artifact;
it is not a master publication. Its WASM hash matches the retained public CI
binary rather than the Windows-local build used for matching memory controls.

The exact CI distribution passed offline close/reopen with the local HTTP server
stopped: saved 933-row season restored, second unsaved season not persisted,
fresh filter returned 835 rows and player detail opened. See
`ci-review-offline-reopen-20261012.json`. This confirms that the renderer works
with the CI-produced WASM binary; it does not add phone, memory or rollback proof.

## Startup controls and full-workload layout isolation (2026-10-13)

PR #90 merged as ede05b79 after 29 successful checks (deployment skipped).
The public release remains the independently verified e0741daa artifact.
New diagnostics use those exact retained CI bytes, original Chromium flags and
OS sampler, fresh owned browser processes, and no forced GC.

Startup-only controls measured minimal HTML at 239.34 MiB and the empty offline-ready
workbench at 276.95 MiB, both with zero engine workers. Blank-browser baselines
were 222.72 and 233.15 MiB respectively. One run per configuration cannot assign
the entire raw difference to app code. These are diagnostics, not workload acceptance.
See startup-minimal-html, startup-empty-workbench and startup-fixed-cost-provenance
JSON evidence dated 20261013.

A stronger layout experiment runs the complete 75-package workload, both official
archive imports (905/332 skaters), 20 committed warm queries and engine restart.
Every result and DOM row is still created; the sole treatment hides the table
container with display:none from startup. A visible control is bracketed by two
hidden runs. Private-byte peaks: visible 550.60 MiB;
hidden 465.41 / 461.78 MiB.
All runs complete, sample every required phase and keep at most eight residents.
The observed 85.19–88.82 MiB difference supports investigating
viewport rendering. It is not a measured saving from virtual scrolling: DOM creation
still occurs, and removing all table layout is a different, unusable treatment.
One visible control and two treatments on one workstation do not establish a
universal saving. No hidden-table change is adopted or published.

Evidence: full-workload-table-hidden-first, full-workload-table-visible,
full-workload-table-hidden-repeat and full-workload-layout-provenance JSON files
dated 20261013. Provenance records runner/sampler hashes and raw baselines.
The 250 MiB whole-browser requirement remains unmet; Android is deferred at the
user's request. The next renderer candidate must keep every result accessible,
correct global ranks, keyboard/detail behavior and complete exports, and must
be measured with actual visible scrolling before any performance claim.

## Viewport renderer prototype (2026-10-13)

Local candidate 83da9b55 keeps the complete engine result, but mounts at most 44
player rows at 1280×800. Scrolling covers all ranks. The full-table option supports
browser Find, printing and assistive technology; exports always use every matching
player. Presentation ranks retain their global offsets. No hockey rules, inputs,
eight-window residency, saving policy or Rust engine behavior change.

Type checking and 149 browser unit tests pass. The permanent opt-in regression is:

`node icelines-browser/e2e/viewport.mjs --artifact target/viewport-artifact-terminal`

Its real browser walks all 933 skaters, compares ranks/names with the complete JSON
export, opens the last player, traverses 120 forward/reverse Tab steps, checks both
terminal Tab boundaries, exercises the 77-goalie full-table/export path and checks
failure clearing/retry and a 390px viewport. Offline process-close/reopen restores
saved 933-player data with 42 mounted rows; offline fresh filtering returns 835,
all-mode names match the full fixture expectation, and player detail works.
These do not establish real Android or screen-reader acceptance.

Stronger repeated probes caught page scroll anchoring and focused-node height
clamping. The prototype disables anchoring while virtual rendering is active and
preserves container height while recycling rows. Terminal rows stay mounted for
keyboard entry/exit from adjacent controls. Failed probes are retained; initial
sleep-based traversal probes were too weak and are not the final regression.

Matching Windows WASM SHA-256 is 4f99584fe3147f05b525dae35c2ac2a49ca23aaa3f84fd961e5cecb9a43f0eee. Control peak: 547.10 MiB;
final candidate runs: 446.23 / 442.02 MiB.
The observed 100.87–105.08 MiB reduction is about
18–19%. All runs keep 75 packages, both 905/332-player archive imports, 20 committed
warm queries, eight resident windows and restart. Browser flags, original sampler
and network observation remain unchanged, with no forced GC. Baselines and source
hashes are in viewport-provenance-20261013.json. The final repeat uses the reusable
memory runner's viewport-aware complete-result checks; it does not replace engine
query size with the mounted-row count. Warm-query expectations come from the full
playoff fixture, applying the same GP/points thresholds as the legacy cell check.

This is a measured local candidate, not a publication or full validation pass.
Both 250 MiB comparisons still fail. Android remains deferred. Raw browser,
offline, candidate/control memory and failed-probe reports use the viewport- prefix.

## Print/accessibility follow-up (2026-10-13)

Candidate application 680a89cb fixes an actual print defect: the original
full-table A4 output clipped the pace column and printed faint names. Print CSS
now fits every column with black text, repeated headers and compact unsplit rows.
Before printing, the renderer mounts the complete result; after printing it
restores viewport mode. The permanent browser regression checks those transitions
and all 933 accessible player-button names in engine order. The actual A4 PDF has
24 pages and all 933 ranks; first, second and last pages are rendered and reviewed.

Exact PDF text preservation is still only partial: two PDF parsers extract
replacement characters for 11 accented names, although sampled raster glyphs
show the accent correctly. A Segoe UI probe reproduces the same mismatch; no font
change is adopted and the producer/mapping cause is unproven. The PDF report keeps
verified:false with layout_verified:true. AX names and canonical JSON preserve
all original names. This is not a physical screen-reader/printer/Android claim.
Raw failure, layout-partial, browser, offline and memory reports plus the exact PDF
and source hashes are retained under the viewport-print prefix.

Type checking and 149 unit tests pass. The actual browser regression repeats
all ranks, last detail, keyboard boundaries, filter/failure/retry and goalie
fallback checks. Offline process reopening/fresh query/detail passes. Updated
normal full-workload private peak is 447.03 MiB against the prior matching 547.10 MiB
control, with all 75 packages/two imports/20 queries/restart and original sampler.
The 250 MiB requirement and real Android gate remain open. The public release
remains e0741daa; PR91 is a draft candidate, not deployed readiness.

## Current gzip release reconciliation (supersedes earlier current-state notes)

PR91 and PR92 are merged. The public validation release is2e265236. Exact master
artifact181 files/75 packages passes actual-WASM, full viewport/keyboard/AX/print,
offline, corrupt-transfer and saved-data update checks. Historical references to
unpublished PR91/e0741daa are scoped to their earlier investigation dates.

Gzip catalog transfers retain canonical raw files/revisions/exports and validate
compressed and expanded byte counts/SHA before activation. All75 transfers fall
from41006884 to4960231 bytes (87.9%). Two opposite-order prototypes peak405.8/389.5
versus controls449.6/434.6 MiB. Exact final Linux-built master on this Windows
workstation peaks388.742 MiB private (blank227.957), with all75 packages,8 windows,
two905/332 imports,20 queries and restart under original flags/sampler, no forced
GC. Warm median70.454/max83.920 ms; no mobile inference.250 MiB still fails.

Public first publication, rollback and final restore succeed; final272-file
verification preserves the original documentation503 and targeted recovery.
Owned saved season/schedule byte hashes and metadata/settings survive. Reused
Chromium153 automation crashes during package export match reported upstream
issue https://github.com/microsoft/playwright/issues/42506. A reversible backup
and removal of three owned-test download-history tables resolves the repeated
crash without changing app IndexedDB/cookies/preferences or benchmark settings.
Final executing main-module SHA, byte-exact export and offline probes pass.
Initial failed transition/export attempts remain; final successful check starts
from already-restored2e265236. This is not a new uninterrupted transition claim.

The earlier accented PDF-name discrepancy is a verifier CP1252/UTF-8 error.
Original raw failure bytes and corrected provenance are retained; current exact
master PDF passes all933 ranks/names with both parsers. The application font/data
needed no correction. Physical printer, screen reader and Android remain unproved.
