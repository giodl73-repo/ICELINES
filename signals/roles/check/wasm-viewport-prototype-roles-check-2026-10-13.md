---
skill: roles-check
topic: wasm-viewport-prototype
date: 2026-10-13
roles_used: [bench, forge, keel, wire, edge]
p1_count: 2
verdict: NEEDS-WORK
---

# Viewport renderer and validation evidence review

Code/measurement review through installed BENCH (test strength), FORGE (ownership
and allocations), KEEL (surface parity), WIRE (offline/data boundaries), and EDGE
(adversarial navigation) lenses. One agent applies these lenses; this is not an
independent review. Scope: local candidate application 83da9b55, permanent browser
regression, viewport-aware memory runner, and the retained reports. Public e0741daa
remains unchanged. Native storage sizing is a separate diagnostic example, not a
new engine representation.

| Role / ID | Finding | Severity | Evidence / recommendation |
|---|---|---|---|
| BENCH B1 | Whole-browser 250 MiB target still fails | P1 open | Final private peaks 442.02/446.23 MiB; retain failed flags and continue optimization |
| BENCH B2 | Mounted row counts cannot stand in for complete query results | P2 amended | Runner checks full result count, query summary and inert completion; full-fixture warm threshold, all 75 packages, both archive imports and 20 queries remain |
| BENCH B3 | Gains need matching binary, flags and owned-process scope | P3 verified | Windows WASM hashes identical; original sampler/launch/network observation, no forced GC; control547.10, final candidate442.02/446.23. Baselines vary and are retained |
| FORGE F1 | UI optimization must not duplicate hockey semantics or alter Rust ownership | P3 verified | No engine/library changes; only presentation window/rank arithmetic. Native sizing example is diagnostic and explicitly not WASM memory |
| FORGE F2 | Complete DTOs must remain available outside the viewport | P3 verified | Full 933-player JSON and all-mode names match; 77-goalie export/fallback parity. Main query export still uses original complete result |
| FORGE F3 | Recycled nodes must not retain obsolete records or callbacks | P3 verified | Existing PlayerTable lifecycle retained; window render replaces callbacks/text, clears failure rows, and actual last-player detail/keyboard/failure retry checks pass |
| KEEL K1 | Row windows must preserve globally ordered ranks across surfaces | P3 verified | Every one of 933 ranks/names compared to shared-engine export, global offset test covers skater/goalie columns |
| KEEL K2 | Local candidate evidence is distinct from deployed release readiness | P3 verified | Candidate is unmerged/unpublished; e0741daa public publication/rollback evidence is not relabeled as viewport evidence |
| KEEL K3 | New UI module must be included in the same offline shell | P3 verified | Exact candidate verifier105 files/75 packages, 25 shell assets; actual offline process-close restoration and new query/detail pass |
| WIRE W1 | Presentation optimization must not silently persist or fetch extra data | P3 verified | Acquisition/storage/library/domain code unchanged; fixture-only browser checks use owned fresh profiles, saved933 versus unsaved second season |
| WIRE W2 | Offline proof requires executing the saved application, not only manifests | P3 verified | Server stopped, browser process closed, offline navigator/probe failure, saved933 restored with42 mounted rows and fresh835 query/full names/detail |
| WIRE W3 | Existing release rollback safety cannot be inferred for a new deployment | P3 qualified | Prior e0741daa public transitions remain separately proven; no viewport deployment or rollback claim |
| EDGE E1 | Scroll anchoring and focus-induced transient height changes can move the page | P2 amended | Stronger failed reports retained; root anchoring disabled while virtual, height preserved during row replacement; exact-window traversal, 120 forward/reverse steps and terminal boundaries pass |
| EDGE E2 | Browser Find, printing and assistive-technology access need complete rows | P3 qualified | Explicit Show all players option restores full native table; labels describe use, complete exports preserved. Real screen-reader and print rendering are not independently verified |
| EDGE E3 | Desktop narrow viewport cannot close Android acceptance | P1 open | 390px layout check is desktop Chromium only; physical phone remains deferred at user's request |

Five roles, 15 findings: two P1 full-goal gaps open, two P2 issues amended,
and eleven P3 findings verified or explicitly qualified. Full-goal verdict:
**NEEDS-WORK**. Local candidate verdict: **APPROVED-WITH-CONDITIONS** for a
bounded optimization review, not full first-release acceptance. BENCH and KEEL
agree that a roughly 18–19% observed saving does not meet the original 250 MiB
target. EDGE and WIRE require distinct physical-device/deployed evidence.

Amendments applied:
1. Validate complete result counts and full-fixture query expectations, not the
   mounted row count; retain the original workload and OS accounting.
2. Preserve page height while recycling focused rows, disable virtual-table
   anchoring, retain terminal rows, and test exact scroll windows plus both Tab
   boundaries. Preserve the failed probes rather than erasing them.
3. Separate candidate/public provenance, complete-table fallback, bounded offline
   proof, and remaining resource/Android gates in both ledgers.

Before promotion: required CI and review of the new interaction behavior; keep
resource/device gates explicitly open. The Show all option is a fallback whose
actual screen-reader/print experience still needs user-facing validation.
