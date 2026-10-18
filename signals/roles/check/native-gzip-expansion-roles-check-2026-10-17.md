---
skill: roles-check
topic: native-gzip-expansion
date: 2026-10-17
roles_used: [hart, keel, forge, wire, bench, edge]
p1_count: 1
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
| FORGE |21 Rust tests and actual fresh WASM checks pass: all75 byte/offset/state comparisons,8 MiB pattern and CRC/truncation/trailing/member/limit refusal. | P3 | Preserve actual-WASM evidence separately from worker mocks. |
| FORGE | Both worker and Rust bound input/output100 MiB; final one-byte probe enforces exact-limit overflow. | P3 | Retain pre-copy type/size guard and CRC read-through. |
| FORGE | Existing locked flate2 version1.1.10; Cargo.lock changes only the WASM dependency edge; actual wasm32 feature tree is rust_backend/miniz_oxide. | P3 | Verify no native/C backend reaches the final wasm32 build. |
| WIRE |13 adapter/worker checks, browser canonical/CRC refusal, six fresh/resident context cases and process-close offline reopening pass. | P3 | Keep previous-good933 rows/exact export/requery and valid905 retry; no new UI cancellation-race claim. |
| WIRE | Transport and canonical digest/size checks remain distinct and precede activation. | P3 | Keep malformed gzip/corrupt wire failures and no permissive fallback. |
| WIRE | Foreground abort/deadline checks run before and after asynchronous expansion and final checksum. | P3 | Preserve delayed-backend tests and acquisition sequence guard. |
| BENCH | Full original pairs are391.4375/393.33594 and392.92578/390.48047 MiB candidate/control; initial reduction reverses sign. | P1 | Do not adopt: no repeated saving;250 MiB remains failed. |
| BENCH | Worker tests replace bindings with a mock and cannot prove native decoding. | P3 | Keep actual-WASM ABI and browser evidence separate. |
| BENCH | Existing OS sampler, normal flags/channel and workloads remain unchanged. | P3 | Never subtract blank baseline or force GC for acceptance. |
| EDGE | Synchronous WASM decoding is bounded but not internally abortable; context checks refuse obsolete results. | P3 | Keep termination/recovery and post-result guards; do not claim cooperative internal cancellation. |
| EDGE | Oversize/malformed requests refuse before native copying and queued stale generation invokes no decoder. | P3 | Preserve protocol regression tests. |
| EDGE |250 MiB and physical Android remain unmet/deferred. | P3 | No full acceptance, adoption or publication claim. |

Roles reviewed:6; P1:1; P2:0; P3:17. Verdict: NEEDS-WORK before adoption.
Amendments applied: pre-copy guards; exact-limit probe/CRC read-through; abort,
deadline and unchanged canonical/context checks after asynchronous decoding.
Completed: full build/check (168 browser units,270 native/WASM parity cases and
required native/Python checks), actual-WASM/native/browser tests above, TS and
pure-Rust wasm32 feature tree. All four normal memory runs complete75/eight/two/
20/restart with every phase and unchanged flags/channel/sampler, no forced GC.
Warm maxima73.4112/84.8653/80.9675/76.7109 ms satisfy governing500 ms on this host.
One CRC fixture timed out using Uint8Array body; corrected to Buffer plus explicit
served-route assertion; original failure/report/source retained. No runtime fix.
Trial closed unadopted/unpublished. No universal causal, resource or Android claim.
Evidence: `context/waves/2026-10-03-browser-wasm/evidence/native-gzip-expansion-20261017-manifest.json`.
