---
skill: roles-check
topic: native-package-digest
date: 2026-10-17
roles_used: [hart, keel, forge, wire, bench, edge]
p1_count: 2
verdict: NEEDS-WORK
---

Single-agent source/proposal review using installed role lenses. This is a local
prototype; no adoption, publication, measured saving or full validation claim.

| Role | Finding | Severity | Recommendation |
|---|---|---|---|
| HART | Digest shares existing SHA-256 helper with native package admission. | P3 | Preserve exact canonical revision identity. |
| HART | Stateless export receives no repository/engine reference. | P3 | Verify actual-WASM query/residency remain unchanged around digest calls. |
| HART | Catalog context still checked before activation. | P3 | Preserve season/type/source and schema refusal evidence. |
| KEEL | Version1 worker adds digest operation and WASM free export. | P3 | Retain CHG-124 and coherent UI/worker/WASM assets. |
| KEEL | Old callers retain default WebCrypto validation. | P3 | No permissive fallback if native operation fails. |
| KEEL | Cached bytes still require exact requested saved revision and full validation. | P3 | Preserve loadSaved guards and cached context tests. |
| FORGE | Actual fresh WASM passes four independent SHA vectors/offsets, oversize refusal and query/residency preservation; worker boundary tests pass. | P3 | Keep evidence scoped to ABI/state checks, not memory saving. |
| FORGE | Worker rejects oversized/non-ArrayBuffer payload before native copying. | P3 | Keep both worker and Rust bounds. |
| FORGE | No new dependencies; primitive uses existing sha2. | P3 | Avoid a hand-written checksum implementation. |
| WIRE | Browser transport/canonical refusal, six resident/fresh context mismatches and offline reopening pass; delayed cancellation is covered by adapter/worker units, not a new browser race test. | P1 | Additional browser cancellation evidence remains required before adoption. |
| WIRE | Compressed-wire checksum stays in acquisition adapter. | P3 | Preserve both compressed/canonical byte limits and hashes. |
| WIRE | Asynchronous checksum now checks abort/deadline before returning. | P3 | Keep explicit delayed-backend failure tests. |
| BENCH | Original full workload peaks at401.28125 MiB candidate versus384.5859375 MiB control; no saving demonstrated. | P1 | Keep prototype unadopted; fixed250 MiB gate remains failed. |
| BENCH | Full build/check exits0:165 browser tests,270 native/WASM parity cases and required native/Python checks; three additional worker tests pass separately. | P3 | Test-only addition does not change frozen runtime identity. |
| BENCH | Original memory runner and workload remain unchanged. | P3 | Preserve75/eight/two/20/restart and no forced GC. |
| EDGE | Cancelled/stale worker results must not activate data. | P3 | Verify generation/sequence behavior with new operation. |
| EDGE | Raw fallback shares canonical digest; no compression downgrade on failure. | P3 | Preserve raw/no-decoder and corrupt transport cases. |
| EDGE | Fixed250 MiB and physical Android remain open. | P3 | No resource waiver or full-goal completion. |

Roles reviewed:6; P1:2; P2:0; P3:16. Verdict: NEEDS-WORK before adoption.
Trial closed without adoption or publication; no reverse pair is justified by
this negative first comparison. One sequential candidate-first pair is not exact
causal attribution; retained control includes intervening source-history differences.
Both use the same pinned Windows compiler/profile and original runner, flags,
sampler and75/eight/two/20/restart workload. Maximum warm queries84.2258/83.5140 ms
are below the governing500 ms target. Whole-browser250 MiB remains failed.
Evidence manifest: `context/waves/2026-10-03-browser-wasm/evidence/native-package-digest-20261017-manifest.json`.
Source amendments applied: worker pre-copy size/type
guard, no engine creation for digest, unchanged context-before-activation checks,
trusted/default verifier distinction and post-checksum abort/deadline refusal.
