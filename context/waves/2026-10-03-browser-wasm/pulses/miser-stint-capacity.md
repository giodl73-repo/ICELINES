# MISER stint-capacity integration

Candidate: c4232db05168c8f6c14853e84de2ac39ddb52953, based on master
663c4f2909905c9e1d3258e26478d36355d30f4a. One production statement shrinks
finalized stint Vec spare capacity after validation/sorting. Public API and all
serialized/statistical values stay the same.

## Measured engine result

Windows x64, Rust 1.95.0, unchanged 75-package catalog. Three opposite-order native
pairs use actual BrowserEngine code with requested-allocation counters and identical
100 non-consumer dependency records. Source input buffers/I/O are outside scopes.

| Actual native engine metric | Control bytes | Candidate bytes |
|---|---:|---:|
| First eight retained requested bytes | 20,479,341 | 18,565,149 |
| First eight construction peak requested bytes | 21,278,653 | 19,400,413 |
| Full 75 sweep construction peak requested bytes | 23,255,110 | 21,196,438 |

First-eight retained reduction is 1,914,192 bytes (9.35%); allocation/reallocation
calls rise from 247,274 to 252,971. Cumulative requested bytes rise too. These are
requested allocations, excluding allocator metadata/internal realloc copy overlap,
not native process RSS or total browser memory. No speedup is claimed.

Five fresh-process pairs of actual distributed WASM show matching engine revisions,
canonical exports and full query results across all 75 packages. Linear-memory
capacity after the sweep falls from 17,891,328 to 16,711,680 bytes: 1.125 MiB.
Node-hosted WASM capacity is separate from browser process counters.

## Functional validation

- Core/data/WASM selected native suites: 1,237 tests/doc-tests pass.
- Native loader/native parity integration suites: 44 tests pass.
- Fresh native-loader/distributed-WASM parity: 270 cases, 33,613 full rows and 90 empty results.
- Browser unit suite: 163 tests pass.
- Artifact verification: all 75 packages and eight-window limits pass.
- Offline close/reopen passes.
- Viewport/export/keyboard/AX/print/retry checks pass (933 skaters, 77 qualified goalies).

The initial concurrent native test builds hit Rust 1.95 delayed import-resolution
internal compiler errors. A dedicated non-incremental target did not alone resolve
it; serialized compilation passed. No root cause is asserted. Reproduce successful
native checks with CARGO_INCREMENTAL=0 and CARGO_BUILD_JOBS=1. Runtime/browser flags
were not changed. CI/account billing and physical Android remain separate gates.

## Whole browser

Two opposite-order pairs use Chromium 153.0.8010.12 and the existing sampler,
unchanged launch settings, all 75 packages, eight resident windows, the same archive
(905 regular / 332 playoff rows), 20 warm queries and engine restart. All four runs
complete with every required phase sampled; no forced GC or baseline subtraction.

| Order / variant | Sampled group peak private MiB | Sampled group peak working-set MiB |
|---|---:|---:|
| Pair 1 control | 390.3125 | 694.6641 |
| Pair 1 candidate | 382.6094 | 675.7109 |
| Pair 2 candidate | 388.3477 | 692.3125 |
| Pair 2 control | 391.9844 | 696.9844 |

Candidate private peaks are lower by 7.7031 and 3.6367 MiB in these two pairs.
This is a small exploratory sample, not a statistically established general saving.
Both private and summed working-set comparisons still fail the existing 250 MiB
criterion in every run. Working-set sums may count shared pages more than once;
process-counter sample maxima can miss transient spikes. The criterion remains open.
No physical-phone or production-origin resource acceptance is claimed.

## Reproduction

Use Rust 1.95.0 and the committed Cargo lockfile. Build with
`node scripts/build-browser.mjs --offline`, run `node --test` over the browser
`.test.mjs` files, and run `node scripts/verify-browser-native-parity.mjs --offline`.
Selected Rust suites: core/data/WASM; fetch tests `stats_loader` and
`browser_native_parity`. Existing offline/viewport/memory scripts accept a retained
artifact via `--artifact`; memory additionally requires the unchanged official
`data-20242025.tar.gz` archive. No forced GC, workload reduction or launch-flag change.

Detailed native/WASM raw measurements and source/lock hashes live in the owner's
private MISER experiment lab (EXP-003 integration). IceLines builds/tests do not
depend on that repository. This public summary preserves scope and exact byte values.
No release deployment or production-origin proof is claimed by this branch.
