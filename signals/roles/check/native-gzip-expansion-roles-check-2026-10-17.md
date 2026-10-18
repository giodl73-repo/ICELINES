---
skill: roles-check
topic: native-gzip-expansion
date: 2026-10-17
roles_used: [hart, keel, forge, wire, bench, edge]
p1_count: 3
verdict: NEEDS-WORK
---

Single-agent local prototype review using installed role lenses; no delegation.
HART covers immutable hockey/revision semantics, KEEL worker/adapter ownership,
FORGE Rust/feature bounds, WIRE authenticated transport/failure behavior, BENCH
discriminating evidence and EDGE cancellation/resource/device limits.

| Role | Finding | Severity | Recommendation |
|---|---|---|---|
| HART | Expansion returns bytes only; native model/query/projection unchanged. | P3 | Require all75 canonical outputs and native/WASM parity. |
| HART | Existing UI/canonical/engine hashes remain, unlike the rejected digest trial. | P3 | Do not combine experiments or change revision/export identity. |
| HART | Normal catalog metadata/context guards still precede activation. | P3 | Retain season/type/source mismatch preservation evidence. |
| KEEL | Version1 adds expandGzip and a stateless free WASM export. | P3 | Build UI/worker/WASM coherently; retain CHG-126. |
| KEEL | New branch uses the existing engine worker, without constructing/selecting an engine for expansion. | P3 | Preserve stateless ABI and worker no-constructor tests. |
| KEEL | Default adapter still supports browser gzip/raw fallback; supplied backend failures never downgrade. | P3 | Keep transport authentication before invoking the backend. |
| FORGE |21 Rust tests pass, including Unicode/offset, CRC, truncation, members, trailing bytes and expansion bounds. Actual WASM not built yet. | P1 | Verify distributed WASM bytes/state/memory and build before adoption. |
| FORGE | Both worker and Rust bound input/output100 MiB; final one-byte probe enforces exact-limit overflow. | P3 | Retain pre-copy type/size guard and CRC read-through. |
| FORGE | Existing locked flate2 version1.1.10; Cargo.lock changes only the WASM dependency edge; actual wasm32 feature tree is rust_backend/miniz_oxide. | P3 | Verify no native/C backend reaches the final wasm32 build. |
| WIRE |13 adapter/worker checks pass; real-browser refusal/context/offline evidence for new backend is pending. | P1 | Verify previous-good full results/export/requery and valid retry. |
| WIRE | Transport and canonical digest/size checks remain distinct and precede activation. | P3 | Keep malformed gzip/corrupt wire failures and no permissive fallback. |
| WIRE | Foreground abort/deadline checks run before and after asynchronous expansion and final checksum. | P3 | Preserve delayed-backend tests and acquisition sequence guard. |
| BENCH | No full-workload saving is measured. Startup probes were reduced diagnostics with null acceptance. | P1 | Run original75/eight/two/20/restart candidate/control; do not infer savings from fewer browser streams. |
| BENCH | Worker tests replace bindings with a mock and cannot prove native decoding. | P3 | Keep actual-WASM ABI and browser evidence separate. |
| BENCH | Existing OS sampler, normal flags/channel and workloads remain unchanged. | P3 | Never subtract blank baseline or force GC for acceptance. |
| EDGE | Synchronous WASM decoding is bounded but not internally abortable; context checks refuse obsolete results. | P3 | Keep termination/recovery and post-result guards; do not claim cooperative internal cancellation. |
| EDGE | Oversize/malformed requests refuse before native copying and queued stale generation invokes no decoder. | P3 | Preserve protocol regression tests. |
| EDGE |250 MiB and physical Android remain unmet/deferred. | P3 | No full acceptance, adoption or publication claim. |

Roles reviewed:6; P1:3; P2:0; P3:15. Verdict: NEEDS-WORK before adoption.
Amendments applied: pre-copy guards; exact-limit probe/CRC read-through; abort,
deadline and unchanged canonical/context checks after asynchronous decoding.
Current checks:21 Rust tests,13 adapter/worker checks, TS noEmit and pure-Rust
wasm32 feature-tree inspection pass. Actual-WASM/browser/memory gates are pending.
