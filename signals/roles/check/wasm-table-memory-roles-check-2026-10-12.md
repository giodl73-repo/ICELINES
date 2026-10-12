---
skill: roles-check
topic: wasm-table-memory
date: 2026-10-12
roles_used: [bench, forge, wire, keel, broadcast]
p1_count: 3
verdict: NEEDS-WORK
---

# Table ownership and memory attribution review

Artifact: `player-table.ts`, its integration with `main.ts`, regression tests,
opt-in CDP diagnostics, memory phase scoping and dated evidence. Inventory:
all 13 Markdown files under `.roles/`. BENCH covers meaningful regression and
measurement tests; FORGE covers ownership and retained memory; WIRE covers
untrusted presentation data and diagnostic boundaries; KEEL covers shared-engine
convergence and acceptance claims; broadcast covers semantic browser interaction.
Selected verification lenses were read. This is one agent applying five lenses,
not independent reviewers or an independent release approval. Native-only Rust
and HTMX constraints do not override the approved static WASM surface.

## BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| B1 | Matching-build control and prototype still exceed 250 MiB | P1 open | MEMORY-ATTRIBUTION.md | Keep resource acceptance open; no forced-GC waiver |
| B2 | Different retained and Windows-built WASM bytes confound attribution | P2 amended | Normal-run comparison | Use the clean 4f13fd70 Windows control with identical WASM hash and the same runner; disclose single-run variance |
| B3 | Stale player callbacks, short-result retention, goalie columns, null/zero values and results beyond the pool have direct assertions | P3 verified | player-table.test.mjs | Keep these cases plus actual offline browser interaction; the DOM double alone is not browser acceptance |

## FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| F1 | Prototype retained 19,140 post-GC DOM nodes by pooling spare rows | P2 amended | player-table.ts | Trim to the current result size; clear every visible row's callback/text, including rows outside the reusable pool |
| F2 | Reusable nodes are bounded at 1,024 while result display stays complete | P3 verified | render | Distinguish pool retention from result limits; do not truncate Rust query output |
| F3 | Unload/recovery releases the pool; kind transitions rebuild shape; query clearing erases the old DTO callback before reuse | P3 verified | clear and main integration | Preserve this ownership matrix; no Rust lifetimes or engine allocations changed |

## WIRE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| W1 | Debugger setup, forced collection and post-GC restart must not count as normal-budget evidence | P2 amended | memory-scope tests and diagnostics | Start diagnostic phase before attaching; exclude every subsequent phase from the normal comparison |
| W2 | Imported names are assigned to Text nodes; HTML-looking names remain text | P3 verified | player-table.test.mjs | Keep text-only insertion; do not introduce innerHTML |
| W3 | Worker inspector attaches only to the exact benchmark worker URL and reads the initialized binding module | P3 verified | memory-diagnostics.mjs | Fail on zero/multiple matching targets or evaluation errors; detach sessions and keep it opt-in |

## KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| K1 | Public previous-version rollback and forward restore remain unrehearsed | P1 open | ACCEPTANCE.md | Preserve pending publication approval; local artifacts and this renderer change do not satisfy that gate |
| K2 | Ranking, filtering, pace and detail computation still run through shared Rust WASM | P3 verified | main.ts / PlayerTable | Restrict this cache to presentation nodes; no second domain calculation or data persistence tier |
| K3 | Prototype measurements and offline pass identify 6851783d, not the later pool correction | P3 constraint | Evidence identities | Measure and identify the corrected artifact separately; keep rejected experiments distinguishable |

## broadcast

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| U1 | Real phone picker, touch and latency acceptance remain unmeasured | P1 open | ACCEPTANCE.md | Use a physical browser/device; desktop headless measurements cannot close this |
| U2 | Header scope, semantic rows, player buttons and missing-value dashes are preserved | P3 verified | PlayerTable render | Keep complete table semantics and current-player accessible text during reuse |
| U3 | Query clears old rows, errors retain existing recovery copy, and load/unload context remains explicit | P3 verified | main.ts | Preserve loading/empty/error status and generation checks; do not render stale async results |

## Synthesis

Five roles applied; three P1 release blockers remain, three P2 issues amended,
nine P3 findings/constraints. Full first-release verdict: **NEEDS-WORK**.
The renderer and diagnostic changes are reviewable separately, subject to
corrected-artifact browser evidence and required CI. This verdict does not demand
new permission for local work or imply approval to alter the public deployment.

Top finding: total browser memory still exceeds the provisional budget. BENCH,
FORGE and KEEL agree that a modest single-run reduction cannot close the gate.

Amendments: release spare DOM rows and scrub all callbacks; compare identical
WASM builds with explicit baseline/variance limitations; keep diagnostic and
post-intervention samples outside normal acceptance. Record corrected-candidate
measurements and offline reopening before describing its resulting behavior.

Corrected-artifact validation: clean 4d90de8b distribution passed actual WASM
verification, all 75 package loads with at most eight resident windows, 905/332
archive rows, 20 warm queries and normal engine restart. Post-GC DOM count was
6,108, close to the control's 6,103. The normal run peaked at 573.44 MiB private
allocations and still fails the budget. Offline saved/unsaved restoration,
fresh query and player detail passed after process close/server stop; all 140
unit tests passed. Required PR CI remains separate. No significant total-memory
or latency improvement and no full release approval is claimed.
