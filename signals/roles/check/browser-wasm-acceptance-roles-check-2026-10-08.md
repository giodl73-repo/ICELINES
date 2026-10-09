---
skill: roles-check
topic: browser-wasm-acceptance
date: 2026-10-08
roles_used: 5
p1_count: 4
p2_count: 4
p3_count: 7
verdict: NEEDS-WORK
---

# Browser WASM acceptance reconciliation review

Artifacts: ACCEPTANCE.md, PAGES.md, updated plan/VTRACE status, deployment
verifier and its tests, live static-byte report and prepared rollback selection.
All 12 installed specialist roles and ROLE.md were inventoried. Selected KEEL
for rollout/data compatibility, WIRE for HTTP integrity, FORGE for resource
ownership boundaries, BENCH for evidence/test scope, and broadcast for browser
acceptance. Other lenses have no changed domain formula, identity model, terminal
rendering or visual design in this follow-up. This is one agent applying installed
lenses, not independent reviewers or human release approval.

P1 findings below block full plan acceptance, not merging this evidence/tooling
update. The four P2 findings were amended during this review. P3 entries are
constraints to retain and scoped follow-up work.

## KEEL

| # | Finding | Severity | Reference | Recommendation / disposition |
|---|---|---|---|---|
| K1 | Remote rollback compatibility remains unverified despite available artifacts. | P1 | ACCEPTANCE rollback row | Execute approved public rollback and forward restore; compare saved bytes and current docs. Prepared local composition is insufficient. |
| K2 | The runbook and plan described the superseded legacy publisher. | P2 | PAGES publication method | Amended to the observed Actions source, master rule and successful retry. Preserve dated branch-method evidence as history. |
| K3 | Static composition proves file preservation, not native/browser semantic convergence. | P3 | deployment verifier scope | Retain native/WASM parity corpus and per-surface evidence; no full-dashboard promotion from this check. |

## WIRE

| # | Finding | Severity | Reference | Recommendation / disposition |
|---|---|---|---|---|
| W1 | An arbitrary remote HTTP URL could previously receive a deployment-verification result. | P2 | verify-browser-deployment.py URL boundary | Amended: remote targets require HTTPS; localhost HTTP remains available to deterministic tests. |
| W2 | Passing bytes cannot establish NHL browser CORS or schema behavior. | P3 | report scope; prior relay evidence | Keep live source evidence separate, with unknown observation times preserved. |
| W3 | The verifier has six concurrent reads, bounded payloads and 20-second socket timeouts, but no whole-run wall-clock deadline. | P3 | verify_file / executor | Document the bound accurately; do not describe this as the app's 120-second foreground-refresh policy. |

## FORGE

| # | Finding | Severity | Reference | Recommendation / disposition |
|---|---|---|---|---|
| F1 | Input-byte budgets and WASM high-water measurements do not prove total browser staging/peak memory. | P1 | ACCEPTANCE resource row | Measure attributed processes and baseline on isolated representative hardware. |
| F2 | The verifier does not run downloaded artifact code; reviewed commit/manifest/package integrity are prerequisites. | P3 | artifact.verify before HTTP jobs | Retain that separation; no hockey formula or worker ownership changes belong in this tool. |
| F3 | Rollback selection depends on expiring Actions artifacts. | P3 | 30-day retention; prepared selection | Keep verified prior/current artifacts outside the transient workflow retention window. |

## BENCH

| # | Finding | Severity | Reference | Recommendation / disposition |
|---|---|---|---|---|
| B1 | Actual deployed offline close/reopen and a new saved query are still missing. | P1 | ACCEPTANCE offline row | Use real browser network blocking; local server-stopped evidence cannot close the deployed-origin gate. |
| B2 | New network-boundary tests initially were not included in the checked browser build. | P2 | scripts/build-browser.mjs | Amended: run test_browser_deployment.py in the checked build. Local selected suite: 17 passing tests. |
| B3 | Initial static-byte reports lacked capture time and explicit transport. | P2 | report metadata | Amended and re-executed: UTC capture time, https transport, 194 matches and no failures recorded. |

## broadcast

| # | Finding | Severity | Reference | Recommendation / disposition |
|---|---|---|---|---|
| BR1 | Desktop viewport results do not prove physical phone/touch/picker/latency acceptance. | P1 | ACCEPTANCE mobile row | Record a real device/browser and the required interactions and measurements. |
| BR2 | A saved-data update walkthrough covers selected states, not the entire loading/stale/partial/unavailable/assistive matrix. | P3 | dated browser observations | Retain scope; full responsive and assistive acceptance needs separate captures. |
| BR3 | Native HTMX/no-build constraints do not apply to the explicitly requested WASM browser surface. | P3 | plan surface boundary | Keep native and static-browser requirements separate; appearance preference is not hidden query truth. |

## Synthesis

Roles reviewed: 5. P1 gates: 4; P2 issues: 4 (amended); P3 notes: 7.
Verdict: NEEDS-WORK for full plan acceptance. The evidence/tooling update is
reviewable; remote CI remains its merge gate. Top finding: real deployed offline
and hardware acceptance cannot be inferred from verified static files.
BENCH, WIRE and broadcast agree on that evidence boundary. KEEL and FORGE agree
that prepared artifacts do not prove rollback or complete memory behavior.

Amendments:
1. Correct publisher/runbook/plan/VTRACE status and separate historical evidence
   from current first-release acceptance. Applied.
2. Refuse insecure remote verification, timestamp results, and wire deterministic
   network-refusal tests into checked builds. Applied and locally verified.
3. Keep offline/rollback/device/whole-browser-resource gates open with exact next
   evidence, prepared rollback identities, and current tooling limitations. Applied;
   the acceptance actions themselves remain incomplete.
