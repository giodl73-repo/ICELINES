---
skill: roles-check
topic: decoder-buffer-release
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL]
p1_count: 2
p2_count: 1
verdict: NEEDS-WORK
---

# Completed decoder-buffer disposal experiment

Single-agent installed-role review. BENCH covers controlled measurement, WIRE
byte integrity/stream lifetime and KEEL ownership/compatibility. Scope: local
unadopted package-download experiment and archived evidence, not release approval.

| Role | Finding | Severity | Evidence / action |
|---|---|---|---|
| BENCH | Peak saving does not repeat. |P1|386.88/395.00 first pair,404.51/382.25 reverse candidate/control. Block adoption.|
| BENCH | Latency observations exceed target. |P1|103.44 control and140.55 candidate retained; no waiver or replacement run.|
| BENCH | Baselines vary. |P3|225.81/232.57 and231.53/231.23 recorded; no baseline-subtracted acceptance.|
| BENCH | Matching engine/template and full workload. |P3|Only decoder module differs; all75/eight/two/20/restart and sampled phases complete.|
| BENCH | Tests expose baseline and assertion-swallowing risk. |P3|New cases fail before implementation; final163 pass; disposal timing asserted outside fallback catch.|
| WIRE | Queued aliases must survive until EOF. |P3|Shared-buffer queued views test preserves all output before disposal.|
| WIRE | Caller input and canonical output stay intact. |P3|Offset input view and exact output tests; raw fallback never detached.|
| WIRE | Optional disposal failures preserve acquisition. |P3|Missing/throwing method tests pass exact output and checksum refusal.|
| WIRE | Integrity/cancellation contracts unchanged. |P3|Transport/raw SHA, size limits, abort/deadline/bomb tests remain; browser corrupt transport preserves previous-good data.|
| WIRE | Real offline/export behavior passes. |P3|Process-close/offline reopening and byte-exact raw package export pass; no public-host claim.|
| KEEL | Holding chunks can increase temporary retention. |P2|Set retains buffers until EOF even without optional disposal; correct before any adoption. Data fallback does not prove memory fallback.|
| KEEL | Disposal is buffer management, not GC intervention. |P3|Optional transfer(0) only; no collectGarbage call or launch-flag change.|
| KEEL | No new actor, public schema or Rust ownership change. |P3|Single decoder module/private helper; WASM byte-identical.|
| KEEL | Validation is separate from current publication. |P3|Public5ef unchanged; merged verifier repair3906 is separate.|
| KEEL | Acceptance remains incomplete. |P3|All memory250 flags false; physical Android deferred.|

Roles reviewed:3; P1:2; P2:1; P3:12.
Verdict: NEEDS-WORK for adoption. BENCH/KEEL consensus: keep experiment out of
master; neither first-pair saving nor intact fallback data establishes benefit.
Amendments: preserve both complete pairs/latency failures; retain local branch
without PR/publication; review producer chunking/temporary ownership before a
different experiment. No further saving is proven.
