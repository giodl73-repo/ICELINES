# Browser allocation investigation — 2026-10-14

Status: partial diagnostic. No implementation change, publication, budget waiver,
or improved acceptance result. The published 38d2a053 full-workload peak remains
390.42 MiB; the 250 MiB gate remains failed. Physical Android is deferred.

The retained Windows control (`target/catalog-metadata-index-final-artifact`)
completed all 75 packages, eight resident windows, both 905/332-player archive
imports, 20 committed queries and restart. Browser flags, original OS sampler
and network observer stayed unchanged. Additional inspector/tracing observations
make these diagnostic totals unsuitable as acceptance measurements. No forced GC
was requested; allocation dumps explicitly use `deterministic:false`.

LayerTree returned no layers throughout a complete workload. That probe does
not establish whether table compositing explains the peak. An initial allocation
trace filled its buffer with unrelated categories and missed the requested dumps.
A shell quoting failure also caused an unchanged unfiltered repeat; its local
report/log remain retained and it does not support any optimization claim.

The corrected trace excludes unrelated categories and uses a continuous 8 MiB
buffer. It contains detailed allocator events at package 26, after 20 queries,
and after restart. Requested blank/startup dumps are absent despite successful
request responses. The summary explicitly fails its all-checkpoint usability
flag; missing observations must not be reconstructed from later allocations.

Serialized allocator event IDs differ from requested dump GUIDs. The summary
associates records through the exact `GlobalMemoryDump` request GUID and its
begin/end timestamp interval, rather than assuming the event IDs are identical.
Allocator entries form overlapping hierarchies and shared ownership graphs;
their sizes must not be summed as physical or private browser memory.

At the post-query checkpoint, one renderer reports 33.88 MiB under `blink_gc`,
including 31.50 MiB under `blink_gc/main`. The GPU reports 31.34 MiB under `gpu`,
including 24.41 MiB under `gpu/shared_images`. After restart those GPU entries
are 28.46 and 26.91 MiB, respectively. These are category observations with
tracing overhead, not proof of application ownership, a leak, or removable bytes.
WASM linear memory in the separate layer diagnostic plateaus at 23.875 MiB
after package 26 and falls to 1.1875 MiB after restart.

The next investigation should resolve browser-managed Blink allocations and GPU
shared-image ownership before another implementation trial. Any candidate still
needs the original acceptance runner and repeatable opposite-order comparisons;
this diagnostic does not justify a smaller workload or changed browser flags.

Exact report hashes, trace hashes/local locations, missing evidence and scope
are recorded in `evidence/allocation-investigation-provenance-20261014.json`.
The summary and both unsuccessful probe reports are retained alongside it.
