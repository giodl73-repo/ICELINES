# Native renderer allocation checkpoints — 2026-10-14

Published5ef015c4 is unchanged. This supplemental diagnostic adds native
allocation dumps to nine exact target/PID checkpoints. Full75/eight/two905/332
imports/20 queries/restart complete with original flags/sampler and no forced
GC. Repeated observers affect later counters; acceptance flags are null.

| Application renderer category, MiB | Eight packages | Ten packages |26 packages |75 packages | After queries | After restart |
|---|---:|---:|---:|---:|---:|---:|
| ArrayBuffer partition size |31.10|14.32|27.82|8.92|8.68|5.94|
| Blink heap size |27.95|33.20|33.26|33.44|33.50|33.50|
| Blink main allocated objects |24.26|2.39|7.72|5.18|2.59|3.06|
| malloc size |38.37|36.63|38.38|26.18|25.45|26.05|

These are overlapping hierarchical/shared allocator records, not a partition
of OS private bytes. Do not sum the rows, assume size equals effective_size,
or infer exact reclaimable bytes. Nonmonotonic ArrayBuffer/live-object counters
and retained Blink capacity suggest allocation churn worth investigating; they
do not demonstrate an unbounded leak or an implementation saving.

The named malloc metadata_fragmentation_caches category is about0.035 MiB after
loading. That does not establish that every allocator cache or fragmentation
source is small. Dump traces prove categories, not individual package ownership.

Each manual dump is identified by its returned GUID and exact GlobalMemoryDump
begin/end interval; unique target timestamps identify renderer PIDs. Initial
summary assumed one allocator event per PID and failed because Chrome emits a
separate global ownership graph under an omnibox PID. That failure is retained.
Corrected summary excludes global graph entries and asserts disjoint local
category names. All nine original checkpoints verify without a browser rerun.

Original traces are archived with lossless deterministic gzip; both raw and
compressed hashes are checked. Runtime local filenames remain unchanged and
provenance explicitly maps each archival trace. Summary/report descriptors are
exact sibling references; runtime local trace references remain external to the
flat archival audit. See renderer-allocations-provenance-20261014.json.

Next inspect ownership of temporary gzip transport/decompression chunk buffers
before testing explicit disposal. Keep canonical raw bytes, integrity checks,
deadlines/cancellation and unsupported-browser fallback intact. No production
change or saving is approved by this evidence.250 MiB remains FAILED; physical
Android stays deferred. PR101's merged verifier repair is separate.
