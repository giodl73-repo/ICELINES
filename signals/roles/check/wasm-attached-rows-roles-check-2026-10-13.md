---
skill: roles-check
topic: wasm-attached-rows
date: 2026-10-13
roles_used: [bench, glass, forge, keel, edge]
p1_count: 2
verdict: NEEDS-WORK
---

# Attached player rows review

Scope: PlayerTable, runQuery lifecycle, regression tests, opt-in real-browser
offline checks and matching Windows memory measurements. All 13 installed role
files were inventoried. BENCH covers evidence, GLASS presentation/accessibility,
FORGE ownership, KEEL domain boundaries and EDGE pending/error transitions.
One agent applied these lenses; this is not independent multi-agent review.

| Role / ID | Finding | Severity | Evidence / disposition |
|---|---|---|---|
| BENCH B1 | The retained 250 MiB whole-browser limit still fails | P1 open | Full original workload remains above target; no waiver |
| BENCH B2 | Node identity alone did not prove layout retention; runQuery still cleared the body | P2 amended | Actual MutationObserver caught 835 reinsertions; caller now suspends rather than clears; corrected offline test observes zero added common rows |
| BENCH B3 | Candidate/control must use identical WASM, full packages and original process counters | P3 verified | Clean artifacts share Windows WASM hash; 75 packages, both imports, 20 queries and restart; no forced GC or debugger intervention |
| GLASS G1 | Pending rows must not look current or remain interactive | P2 amended | visibility:hidden plus inert preserves layout while hiding stale rows; actual pending-state assertion |
| GLASS G2 | Reused buttons must not silently change the focused player's identity | P2 amended | Identity guard blurs a focused reused button when its player changes; regression case |
| GLASS G3 | Table semantics, all results, precision and safe text must remain intact | P3 verified | Existing goalie/skater, null/zero, text safety and >1024-row tests retained; no pagination or CSS redesign |
| FORGE F1 | Removed rows must release player callbacks and names | P3 verified | Tail rows erased, handlers nulled, nodes removed and pool trimmed |
| FORGE F2 | Unload, failure and column transitions must still release/reset owned nodes | P3 verified | clear() remains; same-column growth only appends new rows; goalie transitions rebuild shape |
| FORGE F3 | There is no unsafe Rust or dependency change to obtain the saving | P3 verified | Surface-only TypeScript and opt-in acceptance harness changes |
| KEEL K1 | Projection, sorting and filter semantics stay in the shared engine | P3 verified | Worker/query/export inputs and outputs unchanged; presentation follows returned order |
| KEEL K2 | Saving, recovery bytes and eight-window residency remain intact | P3 verified | Full workload bounds and process-close offline restoration; no local schema change |
| KEEL K3 | Local candidate identity must not be confused with public Pages or Linux CI | P3 verified | Measurements identify runtime 653c876b and Windows WASM; candidate not deployed |
| EDGE E1 | A failed current query must clear hidden retained rows and allow retry | P3 verified | Actual offline malformed query empties table, then valid retry restores 835 rows |
| EDGE E2 | Superseded responses must not expose stale pending results | P3 verified | Existing querySequence guard retained; only current query failure clears; render unhides only after current response check |
| EDGE E3 | Physical Android acceptance is still deferred by the user | P1 open | Desktop tests do not close the phone gate |

Five lenses, 15 findings: two P1 release gates open, three P2 issues amended,
ten P3 findings verified. Full-release verdict: **NEEDS-WORK**. Scoped renderer
patch: **APPROVED-WITH-CONDITIONS**, subject to required CI and bounded memory
claims. BENCH and GLASS agree that proving row retention must not sacrifice
pending/failure visibility boundaries. The first ineffective implementation is
recorded separately from corrected measurements.
