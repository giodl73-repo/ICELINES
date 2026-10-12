---
skill: roles-check
topic: wasm-dependency-audit
date: 2026-10-12
roles_used: [forge, wire, bench, keel, edge]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Dependency audit repair

Artifact: Cargo.lock and the release advisory ledger. All 13 Markdown files
under .roles/ were inventoried. Selected lens.verify sections were read:
FORGE for dependency soundness, WIRE for network dependency boundaries, BENCH
for verification, KEEL for surface convergence and EDGE for failure cases.
This is one agent applying five lenses, not independent reviewers.

The lockfile moves quinn-proto 0.11.15 to 0.11.18, event-listener 5.4.1 to
5.4.2 and lru 0.18.1 to 0.18.2. Quinn's random-number dependency changes are
resolver consequences. No manifest, application source or audit policy changes.

| Role / ID | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| FORGE F1 | Quinn has five vulnerability findings in the old lockfile | P2 amended | Cargo.lock | Pin the compatible 0.11.18 advisory fix |
| FORGE F2 | event-listener has an unsound StackSlot Send/Sync implementation | P2 amended | Cargo.lock | Use the published 5.4.2 fix rather than assume safe call sites |
| FORGE F3 | lru has a panic-safety memory corruption finding | P2 amended | Cargo.lock | Pin 0.18.2; retain native TUI compilation |
| WIRE W1 | Default native and WASM graphs do not activate quinn-proto | P3 constraint | Feature graph | Do not claim HTTP/3 runtime testing |
| WIRE W2 | The update does not change NHL endpoints, DTOs or retries | P3 verified | Source scope | Preserve fixture-based validation; no live upstream CI |
| WIRE W3 | Registry access is needed to resolve fixes | P3 constraint | Validation | Distinguish dependency retrieval from live NHL testing |
| BENCH B1 | Old PR86 CI failed on a real audit finding | P3 constraint | CI | Land the fix and rebase PR86; rerunning the old lockfile is insufficient |
| BENCH B2 | Compilation alone cannot prove every optional dependency path | P3 constraint | Native check | Require existing CI and fresh audit; avoid an all-features claim |
| BENCH B3 | WASM uses a pinned Rust/binding toolchain | P3 constraint | Browser build | Run the repository build and distribution verifier |
| KEEL K1 | No shared model, query or persistence code changes | P3 verified | Source scope | Keep the fix separate from device acceptance evidence |
| KEEL K2 | Lockfile transitive changes extend beyond three version lines | P3 verified | Cargo.lock | Review resolver changes; avoid unrelated upgrades |
| KEEL K3 | Merging a dependency fix does not publish Pages | P3 constraint | Deployment | Keep merge and public deployment evidence distinct |
| EDGE E1 | Existing ledger described a warning no longer reported | P2 amended | release-checklist.md | Remove the stale anyhow entry; retain the remaining async-std entry |
| EDGE E2 | A registry package absent from the activated graph is not a workspace test target | P3 constraint | Validation | Check application targets; do not infer runtime failure from cargo selection errors |
| EDGE E3 | Memory, phone and public rollback gates remain open | P3 constraint | First-release scope | Do not use green dependency CI as full WASM acceptance |

## Synthesis

Five roles applied; zero unresolved P1 findings, four P2 findings amended,
eleven P3 constraints/verified findings. Verdict: **APPROVED-WITH-CONDITIONS**.
Condition: the fresh audit, native CLI compilation, pinned browser build and
required PR checks must pass before merging. FORGE and EDGE agree that patched
versions are preferable to advisory exceptions.

Amendments:
1. Upgrade all three affected dependencies to their compatible advisory fixes.
2. Refresh the warning ledger without weakening the audit configuration.
3. Validate native and pinned WASM targets, while keeping later first-release
   gates and optional HTTP/3 coverage outside this patch's claims.

