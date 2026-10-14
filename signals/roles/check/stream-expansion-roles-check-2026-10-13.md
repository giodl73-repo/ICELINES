---
skill: roles-check
topic: stream-expansion
date: 2026-10-13
roles_used: 5
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Single-buffer gzip expansion

One agent applies the installed BENCH, WIRE, KEEL, FORGE and EDGE lenses,
covering measurement, reliability, architecture, ownership and malformed input.
This scoped review leaves the full 250 MiB and physical Android gates open.

| Role / ID | Finding | Severity | Evidence | Recommendation |
|---|---|---|---|---|
| BENCH B1 | Both opposite-order pairs show lower candidate peaks | P3 | Four memory reports | Claim local observations only |
| BENCH B2 | Baselines differ 4.27–6.98 MiB; raw reductions 11.29–16.03 MiB | P3 | Provenance | Never subtract baselines to pass acceptance |
| BENCH B3 | Exact reviewed-head CI artifact remains untested | P2 | Local artifacts only | Finish CI artifact checks before merge |
| WIRE W1 | Wire hash precedes decode; raw hash/size still precede activation | P3 | downloadPackage unchanged | Keep both checks |
| WIRE W2 | Corruption preserves context, result and exact export; retry returns 905 | P3 | Refusal report | Retain recovery evidence |
| WIRE W3 | No gzip support uses authenticated raw JSON | P3 | Existing test | Preserve fallback |
| KEEL K1 | WASM and catalog match between artifacts | P3 | Provenance assertions | Keep this in transport decoding |
| KEEL K2 | Resident cap, save policy, schema and revision unchanged | P3 | Runtime diff | Exclude native experiments |
| KEEL K3 | Saved season survives offline process reopening and fresh query/detail | P3 | Offline report | Check CI bytes too |
| FORGE F1 | Direct input respects view offset and length | P3 | New multi-chunk test | Never enqueue entire backing buffer |
| FORGE F2 | Ordinary input borrowed, shared-backed input copied for BufferSource | P3 | Narrowing and typecheck | Avoid unsafe casts |
| FORGE F3 | Destination bounded to advertised maximum, at most 100 MiB | P3 | MAX_BYTES guard | Do not claim all decoder/engine copies removed |
| EDGE E1 | Over-bound output refused before writing; short output refused by caller | P3 | Unit cases | Keep maximum and exact-size checks |
| EDGE E2 | Abort/deadline checked before allocation and around reads | P3 | Code and cancellation tests | Keep reader cleanup |
| EDGE E3 | Unicode, truncated gzip and expansion bomb checks pass | P3 | 155 units | Never return partial output after decode failure |

Roles: 5; P1: 0; P2: 1; P3: 14. APPROVED-WITH-CONDITIONS for implementation.
Top finding: finish exact reviewed-head CI artifact checks before merge.
Consensus: BENCH/KEEL require honest identity and open resource gates;
WIRE/EDGE require complete integrity checks before replacing active data.

Amendments:
1. Retain all four reports, runner hashes and baseline caveats.
2. Validate retained CI bytes through browser/offline/refusal checks before merge.
3. Keep desktop acceptance failing and Android deferred; this saving is not readiness.
