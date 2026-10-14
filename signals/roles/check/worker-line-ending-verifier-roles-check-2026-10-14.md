---
skill: roles-check
topic: worker-line-ending-verifier
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Exact worker-byte verification

Single-agent installed-role review; no independent-reviewer claim. BENCH covers
regression/refusal, WIRE authentication and KEEL build/retained-byte boundaries.
Scope: verify-browser-artifact.py and two regression tests, separate from the
unadopted duplicate-checksum experiment. No runtime/browser artifact code change.

| Role | Finding | Severity | Evidence / action |
|---|---|---|---|
| BENCH | CRLF/mixed regression fails before fix. |P3|Two errors retained in single-catalog-auth-verifier-regression-before.log.|
| BENCH | All11 artifact tests pass after fix. |P3|LF/CRLF/mixed acceptance and unsealed line-ending refusal tested.|
| BENCH | Existing commit/dirty/extra-file/integrity refusals still pass. |P3|Full artifact suite, not only new cases.|
| BENCH | Actual clean Windows-built182-file artifact now verifies. |P3|Candidate refusal/context/offline/viewport runs reach browser after verifier fix.|
| BENCH | CI remains required before merge. |P3|Run retained-artifact suite through PR checks; do not infer Linux CI from local fixtures.|
| WIRE | UTF-8 decoding remains strict. |P3|read_bytes().decode('utf-8') rejects invalid encoding.|
| WIRE | Hash preserves original line-ending bytes. |P3|No newline normalization; template reconstructed from original decoded string.|
| WIRE | Manifest regex handles optional CR at line end. |P3|Manifest JSON still must equal sealed shell; template SHA still authenticates all bytes.|
| WIRE | Line-ending tampering is refused. |P3|Changing sealed CRLF to LF without resealing fails worker template hash.|
| WIRE | Payload/transport checks remain unchanged. |P3|No checksum, size, path or gzip-expansion relaxation.|
| KEEL | Builder already hashes exact template bytes. |P3|Verifier now agrees with buildOfflineShell instead of text-file normalization.|
| KEEL | Retained artifacts are not rebuilt or rewritten. |P3|Verifier changes only reading/recognition; original candidate bytes preserved.|
| KEEL | Source identity and shell identity remain enforced. |P3|Clean reviewed full commit and manifest hash tests continue passing.|
| KEEL | Memory optimization is separate. |P3|Extract verifier-only commit; no duplicate-digest change in its PR.|
| KEEL | Acceptance scope unchanged. |P3|250 MiB failure and deferred Android remain open; this is a verification repair.|

Roles reviewed:3; P1:0; P2:0; P3:15.
Verdict: APPROVED-WITH-CONDITIONS; merge only after required checks pass.
BENCH/WIRE/KEEL consensus: exact-byte authentication fixes the Windows false
refusal while preserving tamper rejection. Amendments: meaningful LF/CRLF/mixed
and tamper regressions included; keep retained bytes unchanged; isolate verifier
repair from the performance experiment. Public runtime publication is not needed.
