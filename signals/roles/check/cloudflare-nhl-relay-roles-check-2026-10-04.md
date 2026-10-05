---
skill: roles-check
topic: cloudflare-nhl-relay
date: 2026-10-04
roles_used: 5
p1_count: 1
verdict: NEEDS-WORK
---

# Cloudflare NHL relay review

Scope: relay code/configuration and browser acquisition routing. Standard-depth
review by one agent applying five installed lenses; no independent approvals.
KEEL covers shared-engine boundaries, WIRE acquisition reliability, BENCH test
evidence, FORGE runtime/resource limits, and EDGE adversarial failure paths.
Other hockey scoring, native UI and visual-design lenses are not affected.

Local verification: 128 browser tests plus 6 relay tests passed; distribution
verification passed with 75 public packages. Real relay GETs returned 200 JSON
and the exact Pages CORS origin from all four routes. Full acquisition and Rust
WASM normalization/query/schedule projection succeeded in 22 requests, producing
a 924779-byte season package, eight 100-point leaders and 49 schedule games.
These were command-line network probes with an Origin header, not actual browser
CORS enforcement or a deployed UI refresh. The existing public UI still uses
the previous build until the integration is reviewed and published.

| ID | Finding | Severity | Section | Recommendation |
| --- | --- | --- | --- | --- |
| K1 | Relay contains no normalization, hockey scoring or repository state. | P3 | worker.mjs / acquisition.ts | Retain browser-owned Rust engine and byte-only server boundary. |
| K2 | Both stats and schedule route to the same fixed public relay. | P3 | acquisition.ts | Keep endpoint expectations in acquisition tests. |
| K3 | No catalog import, private package or saved browser record is routed through the relay. | P3 | acquisition.ts / README | Preserve explicit local persistence and catalog download paths. |
| W1 | Actual deployed Pages-origin refresh and persistence behavior remain unverified for this build. | P1 | publication acceptance | Publish the reviewed build, refresh stats and schedule in the real browser, and verify save policy and failure preservation. |
| W2 | Numeric and HTTP-date Retry-After values are retained, exposed through CORS and covered by mocks. | P3 | worker.mjs / worker.test.mjs | Keep retry scheduling in the existing bounded browser network layer. |
| W3 | Public JSON is passed through; browser parsing and Rust validation reject drift before repository replacement. | P3 | acquisition.ts / WASM engine | Do not add lossy normalization or partial-success envelopes on the relay. |
| B1 | Updated source has local proof but its reviewed CI artifact has not yet been built. | P2 | browser-pages.yml | Obtain successful PR/main CI and publish the retained reviewed artifact. |
| B2 | Timeout, oversized body, origin, unknown/duplicate params, invalid dates, redirects and rate-limit errors have deterministic tests. | P3 | worker.test.mjs | Keep live NHL probes separate from CI tests. |
| B3 | A real Workers failure escaped Node mocks because edge fetch lacks redirect:error. | P3 | worker.mjs / live evidence | Retain manual redirect handling and rejection test; maintain a deployed pilot probe for runtime changes. |
| F1 | Free-plan CPU headroom at peak request sizes has not been measured on the deployed runtime. | P2 | runtime acceptance | Measure CPU/error behavior before claiming production resource acceptance; do not upgrade billing automatically. |
| F2 | Response bytes and wall time are bounded to 2 MiB and 15 seconds including body. | P3 | worker.mjs | Keep cancellation and oversize tests; the browser retains its shared foreground deadline. |
| F3 | The Workers rate-limit binding is present and deployment fails closed if it is absent. | P3 | wrangler.jsonc / worker.mjs | Treat the 120/minute IP limit as approximate per-location protection, not an account-wide budget. |
| E1 | Arbitrary hosts, reports, query keys, duplicated parameters and redirect targets cannot select upstream destinations. | P3 | upstreamURL / worker.test.mjs | Preserve fixed HTTPS host construction and one upstream fetch per request. |
| E2 | CORS is not authentication; clients outside browsers can call these public NHL endpoints. | P3 | README / worker.mjs | Keep the public-data-only contract and Free account quota visible to the operating owner. |
| E3 | Rate limiting can affect users sharing an IP, but existing complete memory data survives failed refreshes. | P3 | README / existing controllers | Preserve the prior complete repository and expose retry timing instead of saving partial data. |

Roles reviewed: 5. P1 blockers: 1; P2 issues: 2; P3 notes: 12.
Verdict: NEEDS-WORK for production acceptance; the pilot and merge candidate have
concrete local/server-side proof. Top finding: W1. WIRE and BENCH agree that
server-side Origin-header probes cannot substitute for actual browser acceptance.

Amendments:
1. Complete actual Pages refresh, query, failure-retention and explicit-save
   checks after reviewed static publication (W1).
2. Keep relay tests in CI and use successful reviewed source/artifact for the
   browser publication (B1).
3. Measure deployed CPU under the largest permitted response and representative
   concurrency before closing the existing resource acceptance gate (F1).

This review does not revise the earlier browser release's unrelated mobile,
offline update/rollback, and full peak-memory acceptance findings.
