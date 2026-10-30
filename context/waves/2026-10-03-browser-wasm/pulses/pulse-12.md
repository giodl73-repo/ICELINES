# Browser WASM pulse 12 — measured engine and transfer budgets

Date: 2026-10-03. Parent: WP-BW-05 / WP-BW-06 / REQ-BROWSER-001 / CHG-119.

## Measurement implementation

`scripts/prepare-browser-benchmark-preview.mjs` prepares a local-only harness
outside the publishable distribution. It runs the actual production EngineClient,
worker module and generated WASM, with a wrapper reporting allocated WASM linear
memory and worker response timing. Its initial queue retains messages while the
instrumented worker initializes. No benchmark operations or metadata are added
to the production worker protocol or application UI.

The harness loads all 75 digest-verified public packages sequentially, replacing
one active context at a time. For each it measures package download plus integrity,
load round trip, first post-load query, and 20 warm queries of
`gp>=10 AND p>=20` sorted by points. Round trips include worker transport and
JSON projection; they exclude analysis-table rendering. Worker times include
queuing/execution before response transport. Instrumented startup includes the
harness and localhost delivery, not deployed-network startup.

The distribution verifier now recomputes each asset's byte/gzip measurements,
instead of trusting manifest size fields. It includes service-worker and shell
manifest gzip bytes, adds each individual public package, and enforces the
plan's provisional 10 MiB application-plus-single-package gzip budget. This is
a reproducible gzip sum; it does not assert actual Pages content encoding.

## Actual browser results

Browser: Chrome 154, Windows user agent, Codex in-app browser. Origin:
`http://127.0.0.1:8054/benchmark.html`. Physical device/CPU specifications were
not measured. Both runs completed all 75 packages and 1,500 warm queries.

| Metric | First run | Repeat run |
|---|---|---|
| Instrumented startup | 126.0 ms | 149.2 ms |
| Worst per-package warm round-trip p95 | 3.0 ms | 5.3 ms |
| Peak allocated worker WASM linear memory | 8.75 MiB | 8.75 MiB |
| Sampled main-page JS heap maximum | 44,350,054 bytes | 45,883,560 bytes |

The cached shell asset gzip sum is 245,395 bytes (0.234 MiB). Distribution
verification measured the largest runtime application plus any single catalog
package at 377,496 gzip bytes (0.360 MiB), below the provisional 10 MiB bound.
The verifier also passed all 16 shell hashes, four count-order goldens and 75
actual WASM package loads/queries. Whitespace checks passed.

Machine-readable reports:
[first run](browser-pulse-12-benchmark.json),
[repeat run](browser-pulse-12-benchmark-repeat.json).
Each records observed timestamps, browser user agent, shell build identity,
per-package counts/bytes/timings, sample statistics and the explicit limitations.
[Screenshot](browser-pulse-12.png) shows the repeat result.

## Limits and remaining acceptance

Allocated WASM linear memory is a high-water mark for the worker's linear heap;
it is not total browser memory. `performance.memory` main-page samples are
browser-specific, exclude worker JavaScript/browser overhead, and can miss
transient peaks. The 250 MiB full desktop memory gate therefore remains
unproven. Neither adding those two numbers nor a fast isolated query can close
the full application resource gate.

These measurements cover one active package, not eight resident windows,
archive decompression/staging, a large saved library, rendering full tables,
adversarial maximum-size imports, or mobile hardware. The production saved
library currently reads full saved records with `getAll`; its scale needs
measurement and a bounded approach before claiming lazy historical residency.
The warm engine-query and gzip checks have concrete evidence, but the broader
performance/UX gate remains open. Deployed live access, scores/schedule,
storage/lifecycle acceptance, offline reopening/upgrades, Pages deployment and
rollback also remain required by the plan.
