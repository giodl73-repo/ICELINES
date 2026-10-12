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
