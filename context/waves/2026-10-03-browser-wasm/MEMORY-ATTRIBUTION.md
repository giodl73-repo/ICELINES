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
