---
skill: roles-check
topic: native-symbol-followup
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 1
verdict: APPROVED-WITH-CONDITIONS
---

Single agent applying installed lenses; approval for diagnostic retention only.
[Evidence](evidence/native-symbol-followup-provenance-20261014.json).

| # | Role | Finding | Severity | Action |
|---|---|---|---|---|
| 1 | BENCH | Six exact uncompressed/compressed/pointer requests return404; no transient errors are relabeled. | P3 | Retain individual URLs/statuses. |
| 2 | BENCH | Debug GUID/age and image sizes match both historical module descriptors. | P3 | Reject mismatched binaries before inspecting addresses. |
| 3 | BENCH | The historical profile lacks whole-binary SHA. | P2 | Do not claim historical byte-for-byte identity. |
| 4 | WIRE | HTTPS public symbol requests follow documented server/storage formats. | P3 | No credentials, broad crawling or arbitrary downloads. |
| 5 | WIRE | No existing network/body/application data behavior is changed. | P3 | Keep production intact. |
| 6 | WIRE |404 means absence at the exact URL now, not universal symbol unavailability. | P3 | Preserve availability scope. |
| 7 | KEEL | Current application/official runner/engine are unchanged. | P3 | No protocol/design promotion. |
| 8 | KEEL | Native profiling is not rerun while the same stacks would remain unresolved. | P3 | Avoid redundant observations. |
| 9 | KEEL | Future callsite fingerprints are proposed, not completed ownership evidence. | P3 | Require controlled measurements before interpretation. |
| 10 | FORGE | PE inspection validates MZ/PE/AMD64/PE32+, section-backed RVAs and GUID/image identity before mapping. | P3 | Preserve read-only guards. |
| 11 | FORGE | Export names label only matching runtime-function starts; no nearest-symbol guess. | P2 | Keep880 internal DLL frame identities unresolved except ChromeMain. |
| 12 | FORGE | Unwind ranges identify boundaries, not internal routine purpose or object ownership. | P2 | No allocation-type inference from ranges alone. |
| 13 | EDGE | Files are mapped read-only; binaries are neither loaded nor executed. | P3 | No SDK installs, environment mutation or process intervention. |
| 14 | EDGE | Probe/inspection and exact recipes are archived with sibling hashes. | P3 | Verify Git/checkout custody. |
| 15 | EDGE |250 MiB and deferred Android still lack completion evidence. | P1 | Keep validation goal open. |

Five roles;1 P1,3 P2,11 P3. BENCH/FORGE agree that metadata identity and boundaries
cannot substitute for names/ownership. Amendments: test compressed/pointer forms,
require matching debug IDs/image sizes, and preserve unresolved/no-acceptance
limitations. All implemented. No PR, publication or completion claim.
