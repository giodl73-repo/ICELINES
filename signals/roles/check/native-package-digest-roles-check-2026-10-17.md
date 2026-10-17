---
skill: roles-check
topic: native-package-digest
date: 2026-10-17
roles_used: [hart, keel, forge, wire, bench, edge]
p1_count: 3
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
| FORGE | Actual-WASM ABI and pre-copy boundary checks are not yet verified. | P1 | Run native/actual-WASM known-vector, offset and size-bound checks. |
| FORGE | Worker rejects oversized/non-ArrayBuffer payload before native copying. | P3 | Keep both worker and Rust bounds. |
| FORGE | No new dependencies; primitive uses existing sha2. | P3 | Avoid a hand-written checksum implementation. |
| WIRE | New backend corruption/context/cancellation preservation not yet proved in browser. | P1 | Run real browser refusal, all context mismatches and offline checks. |
| WIRE | Compressed-wire checksum stays in acquisition adapter. | P3 | Preserve both compressed/canonical byte limits and hashes. |
| WIRE | Asynchronous checksum now checks abort/deadline before returning. | P3 | Keep explicit delayed-backend failure tests. |
| BENCH | No repeated original full-workload memory benefit yet. | P1 | Fresh engine build and matched controls; do not claim proposed copy reduction as peak saving. |
| BENCH | Native build/tests are pending, not passed. | P3 | Retain terminal outcomes and failed attempts. |
| BENCH | Original memory runner and workload remain unchanged. | P3 | Preserve75/eight/two/20/restart and no forced GC. |
| EDGE | Cancelled/stale worker results must not activate data. | P3 | Verify generation/sequence behavior with new operation. |
| EDGE | Raw fallback shares canonical digest; no compression downgrade on failure. | P3 | Preserve raw/no-decoder and corrupt transport cases. |
| EDGE | Fixed250 MiB and physical Android remain open. | P3 | No resource waiver or full-goal completion. |

Roles reviewed:6; P1:3; P2:0; P3:15. Verdict: NEEDS-WORK before adoption.
Review gates remain pending. Source amendments applied: worker pre-copy size/type
guard, no engine creation for digest, unchanged context-before-activation checks,
trusted/default verifier distinction and post-checksum abort/deadline refusal.
