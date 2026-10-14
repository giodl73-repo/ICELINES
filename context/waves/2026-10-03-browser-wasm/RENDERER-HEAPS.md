# Renderer heap checkpoints — 2026-10-14

Published5ef015c4 remains unchanged. This supplemental full-workload diagnostic
captures nine per-target Runtime heap counters, trace PID markers and executing
engine WASM linear-memory lengths. It preserves75 packages, eight residents,
905/332-player imports,20 queries and restart with original flags/sampler and
no forced GC. Repeated observers can affect later measurements. Null acceptance
target flags make this explicitly separate from uninstrumented acceptance.

| Checkpoint | WASM linear MiB | Page buffer/external backing MiB | Worker backing MiB | Page embedder heap MiB |
|---|---:|---:|---:|---:|
| Ready before engine |—|0.41|—|2.80|
| First package |4.13|4.20|1.10|6.39|
| Eight packages |14.63|23.14|6.41|24.86|
| Ten packages |15.56|27.86|7.77|3.01|
| Eleven packages |15.56|5.78|8.09|5.63|
|26 packages |16.75|13.63|18.52|2.87|
|75 packages |16.75|8.61|2.20|2.80|
| After20 queries |16.75|5.86|2.49|2.94|
| After restart, fresh worker |1.19|5.86|0.02|3.43|

These are separate counters, not a partition of renderer private bytes. Do not
sum them or count shared target/PID observations twice. The page, engine and
service worker share a renderer. Backing storage includes ArrayBuffers and
external strings; the measurements do not identify individual objects or exact
reclaimable bytes. Nonmonotonic values do not prove an unbounded leak. No GC was
forced. Reading existing initialized WASM bindings is a diagnostic intervention,
not an engine-load acceptance check.

Page allocated JavaScript heap after queries/restart is14.87 MiB, with4.04/4.34
used. This alone does not establish a leak or a savings opportunity. Diagnostic
whole-browser peak400.51 MiB and warm max96.34 ms are not a replacement for the
normal master report or evidence of an improvement.250 MiB remains FAILED and
physical Android remains explicitly deferred.

Next review the duplicate raw-byte hash in loadCatalog: both downloadPackage
and loadSaved already authenticate their returned bytes. Any experiment must
preserve transport/raw integrity, exact catalog context, corrupted-data refusal,
saved-data validation and original full-workload verification. No change or
memory saving is established by these checkpoints.

Byte-preserved report and nine traces are mapped with hashes in
evidence/renderer-heaps-provenance-20261014.json. Runtime trace filenames remain
local references; archival names are explicitly mapped there. Helpers/sample
hashes are separate. No production source, PR or publication changes.
