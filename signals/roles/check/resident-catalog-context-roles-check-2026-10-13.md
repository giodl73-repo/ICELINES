---
skill: roles-check
topic: resident-catalog-context
date: 2026-10-13
roles_used: 5
p1_count: 0
verdict: APPROVED
---

# Cached catalog header guard

Scope: reject season/type/source mismatch before selecting an already-resident
revision. Existing downloaded-byte context checks remain in place. One agent
applies five installed role lenses; no independent-agent review is claimed.
BENCH covers regression fidelity; WIRE transport/cache reliability; KEEL boundary
consistency; FORGE ownership/change scope; EDGE adversarial header cases.
This scoped code review does not close the full 250 MiB or Android gates.

| Role / ID | Finding | Severity | Evidence | Recommendation |
|---|---|---|---|---|
| BENCH B1 | Published baseline selects resident data despite a forged catalog header | P3 | baseline-browser report fails with Selected in-memory season | Keep the failing baseline evidence |
| BENCH B2 | Guard passes six fresh/cached cases, exact previous export, requery and retry | P3 | guard-browser report | Retain the actual browser regression command |
| BENCH B3 | All29 checks passed; exact CI and merged master guard regressions pass | P3 | PR93 master provenance and retained reports | Keep exact artifact identities |
| WIRE W1 | Checks use already-validated metadata plus digest-keyed resident identity | P3 | loadCatalog fast path | Reject before invoking resident selection |
| WIRE W2 | Raw transport/hash validation and fetched-byte header checks remain unchanged | P3 | main.ts diff only cached branch | Preserve both paths' validation semantics |
| WIRE W3 | Fixture is unsigned and reload is handler-driven | P3 | test scope and two-entry fixture | Do not claim offline-shell or user-click-flow proof from this fixture |
| KEEL K1 | No new WASM API or engine normalization logic | P3 | Identical published WASM SHA2e4c8533 | Keep the fix in the browser acquisition boundary |
| KEEL K2 | The same header contract applies to cached and fetched bytes | P3 | Three equalities match existing header gate | Retain consistency across acquisition paths |
| KEEL K3 | No persistence, revision, export or catalog schema changes | P3 | Runtime diff and previous-good export evidence | Preserve saved-data compatibility |
| FORGE F1 | No native or dependency changes | P3 | Four-line frontend guard plus test | Do not reintroduce rejected native prototype |
| FORGE F2 | Error occurs before generation/selection mutation | P3 | guard preceding activateResident | Keep previous active state on rejection |
| FORGE F3 | Metadata values are compared without another package parse or clone | P3 | Resident Dataset metadata | Keep the check small; no memory-saving claim |
| EDGE E1 | Each season, type and source mismatch is covered separately | P3 | Six fixture cases | Retain all three cached regressions |
| EDGE E2 | Exact previous raw package export survives failures | P3 | Byte comparison against immutable source file | Avoid count-only preservation evidence |
| EDGE E3 | A valid retry still selects the905-player second season | P3 | Retry report | Keep recovery in the regression |

Roles reviewed:5; P1:0; P2:0; P3:15.
Verdict:APPROVED for this cached-header guard. PR93 merged as b29ba81c;
exact master artifact passes catalog, viewport and offline checks. Not published.
Top finding: cached and fetched paths must enforce the same catalog context.
Cross-role consensus: WIRE/KEEL/EDGE require rejection before state mutation.

Amendments:
1. Retain reviewed-head and merged-master CI/artifact proof, now completed.
2. Keep the intentionally unsigned/handler-driven fixture scope explicit.
3. Keep resource/mobile gates open and the rejected native memory experiment out
   of the implementation. Published desktop result remains388.7 MiB, above250.
