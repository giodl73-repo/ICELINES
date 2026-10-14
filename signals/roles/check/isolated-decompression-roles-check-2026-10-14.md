---
skill: roles-check
topic: isolated-decompression
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 0
p2_count: 4
verdict: APPROVED-WITH-CONDITIONS
---

# Short-lived package decoder — local experiment review

Artifact: package-decoder.ts, package-decoder-worker.ts, optional expander in
package-download.ts, and the main catalog acquisition call. Single-agent review
through five installed role lenses; no independent-agent review claim.

Scope: a local memory experiment, not approval to publish or declare acceptance.
The decoder handles authenticated compressed bytes only. It never queries Rust,
fetches live data, saves records, changes residency, or activates a dataset.
Canonical raw bytes still pass size/SHA, context and Rust validation before use.
The client terminates its dedicated worker on every terminal path. Existing
engine-worker protocol and local-state schemas remain unchanged.

BENCH covers real execution and measured claims; WIRE integrity/error boundaries;
KEEL ownership and offline assets; FORGE bounded owned buffers and cleanup;
EDGE cancellation, stale work and failure behavior. Hockey-model-specific lens
checks are unchanged by this transport-only candidate and are not newly proved.

| Role | Finding | Severity | Evidence / required action |
|---|---|---|---|
| BENCH | Fake-worker tests exercise abort, deadline, crash, post failure and malformed responses. | P3 | Six targeted tests pass; keep actual execution evidence separate. |
| BENCH | Actual WASM/catalog, prior-good corruption refusal and offline restoration are not yet proved for this candidate. | P2 | Run retained-artifact build, refusal and offline browser checks before adoption. |
| BENCH | A new worker may add more memory than it releases. | P2 | Original full workload, matching control, opposite-order repeat if first pair shows saving; no diagnostic totals as acceptance. |
| WIRE | Compressed SHA is checked before transferring its bytes to the decoder. | P3 | downloadPackage authenticates transport before injected expansion. |
| WIRE | Raw size and SHA checks remain after expansion, with no raw downgrade on failure. | P3 | Failed worker/decode propagates through existing acquisition refusal. |
| WIRE | Explicit response length preserves short-output detection despite spare destination capacity. | P3 | Client checks length and maximum buffer capacity; caller checks exact raw size. |
| KEEL | Engine and persistence remain on their current surfaces. | P3 | Decoder owns temporary gzip state; Rust remains in the existing engine worker. |
| KEEL | Two new modules must belong to the exact offline shell. | P2 | Automatic asset enumeration includes them; verify artifact and process-close offline recovery. |
| KEEL | A adopted additional runtime actor changes architecture documentation. | P2 | Before closing an adopted implementation, update controlled design/interface/trace records. An unadopted local prototype does not change the release baseline. |
| FORGE | Byte views with offsets cannot transfer surrounding data. | P3 | Client copies subviews; exact full buffers transfer directly. Targeted sentinel test passes. |
| FORGE | Input and output capacities remain bounded to100 MiB. | P3 | Worker bounds input; shared expander validates maximum; client bounds returned buffer and length. |
| FORGE | Timers, listeners, message handlers and worker lifetime have one terminal cleanup path. | P3 | finish is idempotent and terminates before resolving/rejecting. |
| EDGE | Cancellation terminates decoding and rejects acquisition. | P3 | Abort test passes; existing acquisition sequence still prevents stale activation. |
| EDGE | Worker faults do not cause automatic alternate decoding or activation. | P3 | onerror/onmessageerror/post failures reject; previous engine data stays outside this worker. |
| EDGE | Browsers without global Worker keep the bounded decoder path; browsers without gzip support retain existing catalog fallback. | P3 | Fallback test passes; a launched worker failing is terminal, not a downgrade trigger. |

Roles reviewed:5. P1 blockers:0. P2 conditions:4. P3 notes:11.
Verdict: APPROVED-WITH-CONDITIONS for local experimentation only.
Top condition: measured repeatable benefit is required; expected heap reclamation
is a hypothesis. BENCH and KEEL agree exact offline/runtime evidence is required.

Amendments applied: bound returned buffer capacity; isolate all worker faults and
cleanup; preserve explicit raw output length. The 250 MiB whole-browser requirement
and physical Android gate remain unmet and are not waived by this review.
