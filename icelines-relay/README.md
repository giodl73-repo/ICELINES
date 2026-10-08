# IceLines NHL relay

The pilot targets Workers Free in the user's Cloudflare account at
`https://icelines-relay.giodl73.workers.dev`. The relay was deployed and its four
upstream routes validated on 2026-10-04. This code forwards only skater bios,
skater summary, goalie summary and a calendar-valid schedule date. It is
not an arbitrary URL proxy. Stats pages always request ascending `playerId`
ordering, including requests from older clients without a sort parameter. Only
the exact canonical sort is accepted when supplied. Rust normalization and queries stay in the browser.

Public GET requests omit upstream cookies, authorization and browser Origin.
The only browser origin granted CORS is `https://giodl73-repo.github.io`.
CORS is not authentication; clients outside browsers can still call these public
routes. No private package, local catalog or saved browser data is sent here.

Each request allows one upstream fetch, no redirects, 15 seconds including body,
2 MiB maximum body and no relay cache. The runtime rate-limit binding allows
120 requests per minute per client IP at each Cloudflare location. It is an
approximate abuse limit, not a global budget; shared networks share this limit.
The Free account request allowance is shared by all Workers and may exhaust.
No billing upgrade is part of this pilot. Check dashboard CPU and error metrics
after deployed-origin validation; passing local mocks is not upstream proof.

Install the locked tool with `npm ci --ignore-scripts`, run `npm test`, then
authorize your account with `npx wrangler login`. Review the OAuth permission
screen yourself. Never put tokens in source or send passwords in chat.
Use `npx wrangler login --scopes account:read user:read workers_scripts:write`;
the legacy `workers:write` scope was insufficient for deployments. Wrangler
also requires background access to renew credentials. `npm run deploy` deploys
both the code and required rate-limit binding. Deploy
fails closed when that binding is missing. Account ID is a public identifier.

Live route checks returned 200 and the exact Pages CORS origin for skater bios,
skater summary, goalie summary and the 2026-10-04 schedule. Worker version
`abaa29ab-ba1f-41cb-a0de-b54ccef19a9b` contains the tested implementation.
`node probe.mjs` repeats first-page/header checks. After building the browser,
`node probe-engine.mjs` exercises live pagination through browser acquisition
and Rust WASM; the validation produced 22 requests, a 924779-byte package,
eight `p >= 100` leaders and 49 projected schedule games. These command-line
checks are not deployed-browser acceptance. Browser routing changes require
review, CI, static publication and an actual Pages-origin refresh check.

Workers rejects `redirect: 'error'`; the relay uses `manual` and rejects all
non-success statuses without following redirects. Browser-side requests still
use `error`. The dashboard's runtime Errors card does not count every handled
HTTP 502; inspect request status and warning events when diagnosing failures.

On rollback,
restore the prior browser build before reverting or disabling this Worker.
Authentication and CI deployment secrets are separate from the public endpoint.

## Pagination correction (2026-10-06)

Actual Pages-origin refreshes exposed unsorted offset pagination: a 940-row
summary response sequence contained only 924 unique players. Earlier route
smokes and eight-leader probes did not establish full report completeness.
Direct NHL requests with ascending player ID returned 940 unique bios, 940
unique summaries and 98 unique goalies in strict order. The relay injects that
ordering for existing clients; new browser acquisition also requests it and
rejects duplicate/unordered IDs or disagreeing bios/summary coverage before
building a replacement package. Live mid-acquisition mutations can still cause
a refusal; retry preserves the previous good dataset.


The correction is deployed as Worker version
`7f92df73-2907-42c8-b498-6e01d2a64e85`. Full relay acquisition plus actual Rust
WASM passed (22 requests, 924767 bytes, eight leaders, 49 games); the existing
Pages client refreshed all 940 skaters and restored Pastrnak among eight
100-point leaders. New browser guards require the follow-up PR and publication.
