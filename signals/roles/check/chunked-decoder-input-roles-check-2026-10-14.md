---
skill: roles-check
topic: chunked-decoder-input
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Pull-based decoder input review

Single-agent installed-role review, not independent reviewers. BENCH covers
measurement/regression, WIRE integrity/cancellation and KEEL allocation/ownership.
Scope: incremental implementation and local evidence; not full resource/mobile
acceptance. Required PR CI and exact preview/master/public recovery remain pending.

| Role | Finding | Severity | Evidence / condition |
|---|---|---|---|
| BENCH | Two independent opposite-order pairs repeat lower raw peaks. |P3|387.26/394.45 and383.18/389.93 MiB candidate/control.|
| BENCH | Baseline differences limit causal precision. |P3|229.56/232.67 and227.47/230.81 retained; no universal saving or subtraction as acceptance.|
| BENCH | Warm maxima stay below100 ms in all four observations. |P3|97.64/98.59/92.52/84.08; no latency waiver.|
| BENCH | Original complete workload/observer retained. |P3|75/eight/two905/332/20/restart, all sampled phases, original flags, no forced GC.|
| BENCH | Meaningful new regressions plus163 units pass. |P3|Bounded offset writes/cancellation fail before change; incompressible gzip boundary test uses native decoder.|
| WIRE | Transport/raw authentication unchanged. |P3|Both SHA/size checks and catalog/engine validation stay in place.|
| WIRE | Offset input is immutable and zero-copy. |P3|Subview test proves exact backing identity/offset and full unchanged input.|
| WIRE | Cancellation interrupts producer writes. |P3|Abort after first transform forwards only part of payload; reader cleanup/typed error verified.|
| WIRE | Foreground deadline applies to each pull. |P3|Same supplied signal/deadline; no separate retry/fallback after decompression error.|
| WIRE | Corruption and offline/export paths pass real browser. |P3|Corrupt gzip preserves previous-good context; exact raw export/retry and offline reopening pass.|
| KEEL | No new actor/schema/dependency or Rust change. |P3|Only decoder frontend module differs; WASM/template byte-identical to Windows control.|
| KEEL | Producer write bound is narrow. |P3|8 KiB per write does not hard-bound native decompressor allocations or whole-browser peak.|
| KEEL | No disposal or chunk-retention set introduced. |P3|Caller/decoder ownership remains ordinary GC; raw fallback unchanged.|
| KEEL | Player-table functionality is intact. |P3|933 names/ranks,44 mounted,120 forward/reverse Tab, filter835, goalie77, detail/AX/print transitions and390px pass.|
| KEEL | Completion remains unproven. |P3|250 MiB failed including startup; Android deferred; CI/master/public source still to validate.|

Roles reviewed:3; P1:0; P2:0; P3:15.
Verdict: APPROVED-WITH-CONDITIONS for incremental review only.
BENCH/KEEL consensus: repeated lower local peaks justify a candidate, not exact
causal savings or full acceptance. Amendments incorporated: original workload
and baselines retained; narrow allocation-bound language; CI/exact artifact and
publication/recovery conditions remain explicit. No250 MiB or Android approval.

## Final reconciliation after publication

PR102 is merged/published as75e6ba97 after29 PR checks. Master/preview Git trees
match; no automatic native master run exists or is claimed. Nonpublishing master
dispatch38571406174 succeeds, with exact182-file retained artifact11897464859;
all179 non-metadata files match reviewed CI. Actual master corruption/context,
offline/viewport/keyboard/AX and24-page/two-parser933-name/rank print checks pass.
Executing WASM bytecode matches the retained binary. First master101.06 ms warm
max remains in evidence; repeat93.81 is separate. Neither replaces the original
nor waives the resource/latency evidence.

Retained-byte forward/rollback/restore all succeed without rebuild, with273
served files and95 identical documentation hashes in each completed phase.
First rollback static check had one HTTP503; failure remains, a new full check
passes. Original owned profile5ef->75->5ef->75 transitions preserve saved data,
metadata/revisions/settings, exact gzip export and offline fresh queries/detail.
Executing decoder/module/WASM fingerprints and blocked network probes match.

BENCH/WIRE/KEEL conditions for the incremental validation release are satisfied;
records explicitly preserve failures and scope limits. Full acceptance remains
NEEDS-WORK:250 MiB fails and physical Android remains deferred. This review does
not approve a budget waiver, physical-device claim or completed full goal.
