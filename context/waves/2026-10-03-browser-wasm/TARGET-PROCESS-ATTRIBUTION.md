# Renderer target attribution — 2026-10-14

Published5ef015c4 remains unchanged. Two supplemental runs map browser targets
to Windows process counters using unique console timestamp trace markers.
Every marker executes in its separately attached target; each matching trace
event supplies a PID. Mapped PIDs are present in SystemInfo as renderers and in
the sampled peak. The observer runs after the measured phases, without forced
GC or altered launch flags. This establishes process membership, not removable
allocations or a per-target partition within a shared process.

| Process group at each sampled group peak | Startup MiB | Full workload MiB |
|---|---:|---:|
| Workbench page and service worker; full run also engine worker |46.17|155.41|
| Chromium omnibox renderer, two browser_ui targets |44.65|42.49|
| GPU |90.11|104.70|
| Browser |46.78|49.94|
| Renderer without a mapped target |16.78|16.49|
| Network service |16.96|17.20|
| Storage service |9.86|9.91|
| Unmapped owned child |2.91|2.91|
| Whole owned process group |274.21|399.05|

Each column comes from one actual sampled group peak, not a sum of individual
process lifetime peaks. They are separate runs; their difference is not a
paired causal estimate. Private bytes describe private allocations, not exact
resident RAM. The two omnibox targets share one PID; the page, service worker
and engine worker share another. Never count their process allocation once per
target. The unmapped renderer's role is not established; it is not excluded.

The full diagnostic retains75 packages, eight residents,905/332-player imports,
20 committed queries and restart. Its pre-observer peak occurs during
package_20202021-regular, with warm max90.46 ms. Mapping occurs before restart;
subsequent restart counters are not normal controls. Both reports explicitly
set acceptance target flags to null. Existing uninstrumented master evidence
still governs the250 MiB failure. This is not physical-phone acceptance.

Next investigate JavaScript heap, ArrayBuffer/external backing, WASM linear
memory and native allocations inside the identified application renderer at
peak-package checkpoints. Its155 MiB is not all WASM and is not all removable.
Do not exclude Chromium UI/GPU from the whole-browser gate or disable offline
capability. Android remains deferred. Full validation remains incomplete.

Reports, traces and sibling hashes are retained in
evidence/target-process-provenance-20261014.json. Local helper/sample hashes are
recorded separately. No production source, PR or publication changes.
