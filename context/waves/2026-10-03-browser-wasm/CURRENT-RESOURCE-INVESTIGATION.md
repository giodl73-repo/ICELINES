# Current resource investigation — 2026-10-13

Published application source is e58a3bfc. Its normal full-workload private peak
remains397.09 MiB against the retained250 MiB target. PR95 merged the validation
records as05396122 after29 checks; its CI artifact c9211a54 has identical
frontend modules, CSS, catalog and WASM to the published release. This records
branch does not imply a new application publication or full acceptance.

## Rejected paint experiment

Local57562a07 makes the virtual table background transparent while preserving
actual row/header surfaces. Complete scrolling, keyboard, export/print and
three-theme text/effective-background equality pass. One original-runner pair
measured407.18 MiB candidate versus395.92 MiB control, blank baselines237.29 and
229.90 MiB. It shows no useful saving and is unadopted on the separate local
table-paint experiment branch. No public style change was made.

## Current read-only attribution

Both diagnostics retain75 packages, eight residents, two archive imports,
20 queries and restart, with no forced GC. Additional inspector observations can
alter timing/collection, so target comparisons are explicitly null. These do
not replace the normal acceptance measurement. Original runner/sampler/phase
classifier hashes remain unchanged. Raw reports and observer hashes are retained
in current-runtime-attribution-* evidence.

The first diagnostic's nearest OS samples show renderer-process private totals
of82.59 MiB at blank,201.93 MiB after eight packages,189.46 MiB after twenty
queries, and156.46 MiB after restart. GPU totals are76.12/96.48/104.45/108.49 MiB
at those checkpoints. These are process aggregates and sequential observations,
not a causal allocation breakdown or synchronized peaks.

WASM linear memory reaches23.875 MiB. Main-context backing storage reaches23.03
MiB at the eight-package checkpoint, then5.85 MiB after imports/queries. Main JS
used heap stays below8 MiB at captured checkpoints; embedder and backing metrics
are separate observations and may overlap other counters. No leak is established.

Supplemental observations find service-worker JS used heap0.66–0.69 MiB,
capacity2 MiB and backing storage1.84–1.85 MiB. This does not support attributing
the large gap to service-worker JS heap growth. Chromium's internal omnibox
browser-UI targets are present even at blank. Their identical heap observations
can overlap; never add per-target heaps or subtract browser UI to waive the gate.

## Next allocation candidate

The catalog load path decodes/parses the complete JSON package on the UI thread
to read six header fields, then the engine decodes the package again for full
validation and normalization. Investigate a bounded read for the repository
producer's known header layout, retaining the full-parser fallback for other
layouts and unchanged worker validation before active-state commit.

Before adoption, compare every catalog header with the full-parser result and
verify Unicode/escaping, reordered or long headers, malformed records beyond the
prefix, duplicate root fields, catalog-context refusal, previous-good results,
resident order, exact raw exports and retry. Do not silently move or weaken a
controlled refusal/cancellation guarantee. Measure the same full workload with
the original runner; keep the candidate local unless saving is demonstrated.

The250 MiB requirement remains unmet. Physical Android remains deferred by the
user; browser flags, residency, workload and measurement scope are unchanged.
