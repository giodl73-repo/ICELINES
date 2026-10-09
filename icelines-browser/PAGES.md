# Static Pages rollout

The repository serves documentation at
`https://giodl73-repo.github.io/ICELINES/`. Actions publication composes the
documentation from `gh-pages:/` with the browser distribution.
The static WASM workbench lives at
`https://giodl73-repo.github.io/ICELINES/workbench/`.
Rust runs in the browser worker; Pages serves HTML, JavaScript, WASM and packages.

## Current publication method

As verified on 2026-10-08, Pages uses `build_type: workflow`. Publication run
`37854895167`, attempt 2, deployed reviewed source
`696c656bead479bde0048059b551c49aa53856eb`. Its clean retained artifact has
103 files and 75 public packages, with shell identity
`5875fe2396a1a1f19c105355cc597323613e0c10924318b8279b51789d0b445f`.

1. Review and merge the source PR after CI passes. Dispatch
   `gh workflow run browser-pages.yml --ref master -f publish=true`.
   The workflow rebuilds/verifies the exact source and retains the distribution.
2. The workflow reads current `gh-pages` documentation and stages the workbench
   under `/workbench/`, retaining a preservation inventory. It publishes the
   composed site through the `github-pages` environment. Master must be allowed
   by that environment's deployment branch rules.
3. Verify the deployment conclusion and actual served identity/MIME. Download
   its retained artifact and preservation inventory, then run
   `verify-browser-deployment.py` as shown below. Separately verify browser
   update/reload, saved-data compatibility and live adapters.
4. A documentation-only change on `gh-pages` does not itself publish the Actions
   site. Dispatch a composed publication to carry that documentation forward.
   Do not publish a docs-only artifact that removes the workbench.

The first theme publication attempt was rejected because the old environment
allowed only `gh-pages`. Switching Pages to workflow mode added a master rule;
the subsequent failed-job-only rerun deployed the already verified artifact.
Both master and gh-pages rules currently exist; the workflows dispatch on master.

```powershell
python scripts/verify-browser-deployment.py <artifact-dir> https://giodl73-repo.github.io/ICELINES/workbench/ <full-source-commit> <shell-sha256> --preservation <pages-preservation.json> --output <evidence.json>
```

This proves static bytes and MIME types, including preservation of publicly
served documentation files. It does not prove browser CORS, saved storage,
offline reopening or device/resource acceptance. The publisher marker and
TypeScript declaration files are excluded from runtime HTTP checks.
The verifier uses six concurrent reads, a 20-second socket timeout per request
and bounded response bodies; it does not impose a whole-run wall-clock deadline.

## Historical branch publication method (through 2026-10-06)

User decision on 2026-10-04: keep the existing branch publishing setup and put
built static files under `workbench/`. No Pages source migration or environment
branch-policy change is required for this method. A code-review PR and a static
publication are separate operations; the implementation was merged in PR #74.

1. Require a successful reviewed master build and repository CI. Download its
   retained `icelines-browser-<commit>` artifact and run
   `scripts/verify-browser-artifact.py` with the exact source commit and shell hash.
2. Read the current gh-pages commit into a clean isolated checkout. Compose a
   fresh output using `scripts/stage-browser-pages.py`, preserving documentation
   and placing the verified artifact under workbench. Never publish the browser
   as the site root.
3. Review only the workbench subtree and, if missing, the root `.nojekyll` marker.
   Verify staged Git blob bytes match the artifact; Windows checkout line-ending
   conversion must not invalidate shell hashes. Existing documentation Git blobs
   must remain unchanged.
4. After publication authorization, commit and push normally to gh-pages. A
   non-fast-forward rejection means reread the current docs and recompose; do
   not force-push over another publisher. GitHub's branch Pages workflow publishes
   this branch. An Actions push using GITHUB_TOKEN does not itself trigger a branch
   Pages build; future automation must account for that documented limitation.
5. Verify the Pages deployment, exact HTTPS shell/worker/WASM MIME and identity,
   root docs, deep-link reload, saved-data recovery and live adapters separately.
   Native HTTP probes do not establish browser access.

## Historical first validation publication

Source: `d1bbc239a01749e6cbe0f5ca6a4fdcea5f67ae07`.
Retained master build: Actions run `37224191804`; repository CI also passed.
Publication commit: `1b3b9dfcc6aab2af0e591018ab024e23db2fd996` on gh-pages.
Pages run `37239443772` succeeded. All 102 staged browser files match retained
bytes; all 95 existing documentation files were preserved. HTTPS verification
matched 24 shell/worker/manifest files and the original Git root index.
Root index SHA-256: `83e77bbb606d5375e57dd1eeb893b6db7e2245e21e83e2293148440ad0334c23`.
Earlier `cb48...` evidence hashes the CRLF Windows checkout, not the served Git blob.

Actual deployed-browser checks loaded the catalog, ran skater/goalie queries,
opened player details and restored an explicitly saved season after reload.
Direct stats and schedule refresh both failed with TypeError: Failed to fetch;
existing stats remained available. Initial native diagnostics returned HTTP 403. Controlled follow-up requests
with a browser User-Agent returned 200 JSON for bios, summary and schedule, but
omitted Access-Control-Allow-Origin for the GitHub Pages origin. The stats summary
request with NHL’s own website Origin returned that allowed-origin header. This
confirms a CORS blocker for our browser acquisition. At that stage, relay hosting and ownership had not yet been selected.
This publication does not close live/device/full peak-memory/offline/update/rollback
release acceptance.

## Storage and rollback

Storage and service-worker scope are deployment-path specific. Local preview
saves do not automatically transfer to the public path; use package export/import.
Keep downloaded previous-good artifacts beyond Actions retention. The current
`browser-rollback.yml` workflow selects a successful master artifact by run ID,
full source commit and shell hash, verifies it without running its code, composes
it with current gh-pages documentation and publishes that composed site. For a
forward restore, select the current known-good artifact through the same workflow;
another Rust rebuild is unnecessary. Do not reset the entire documentation branch
to an old commit. Verify saved-data compatibility and update/reload behavior
before claiming rollback acceptance. Remote rollback has not been exercised.

The Actions publication path is now active. Its retained-master artifact guard
and verification remain required; a prepared workflow is not proof that a
rollback rehearsal passed. See the current acceptance ledger before claiming
first-release completion.

## Historical relay-enabled validation deployment

A fresh GitHub Pages API check on 2026-10-06 confirmed `build_type: legacy`,
source `gh-pages:/`. The branch publication procedure above remains active.
The relay-enabled update used source
`4ae78a8c340bd3eda676633b8775f0cf18078a28` (merged PR #75), browser run
`37253045085`, and Pages commit
`ae53f8bc2fc44b4f955d0b161429cdac70bdb73c`.
Pages run `37550820648` succeeded. Its shell identity is
`4f7f1e6de9be3772720538c7464af640dc27730dbd060c360b1bad04c7a0568b`.
All 102 staged browser files matched the retained artifact, 95 documentation
files were preserved, and 24 served shell/worker/manifest files matched their
recorded hashes. Root documentation retained the Git/HTTPS identity above.

The user selected Cloudflare Free, with their account as operating owner.
The `icelines-relay` Worker supplies bounded NHL stats and schedule routes;
see `icelines-relay/README.md`. Worker version
`7f92df73-2907-42c8-b498-6e01d2a64e85` corrects upstream pagination ordering
for the deployed PR #75 browser. Actual deployed-browser checks loaded all 940
skaters, found all eight 100-point leaders, and recovered explicitly saved live
data after reload. PR #79 adds stricter browser completeness guards; those guards
are not part of the recorded PR #75 static deployment.

Verification commands take positional arguments:

```powershell
python scripts/verify-browser-artifact.py <artifact-dir> <full-source-commit> <shell-sha256>
python scripts/stage-browser-pages.py <pages-baseline> <artifact-dir> <new-output>
```

Use the successful merged master artifact for publication. Retain the staging
preservation inventory outside the site. The branch procedure above records
the 2026-10-06 deployment; subsequent publication follows the current Actions
procedure at the start of this guide.

Evidence is under `context/waves/2026-10-03-browser-wasm/evidence/`, including
`deployed-relay-publication-20261006.json`,
`pagination-relay-runtime-20261006.json`, and
`deployed-mobile-viewport-evidence-20261006.json`. Desktop Chromium at 360px
verified contained table scrolling, player dialog dismissal/focus restoration,
goalie queries and refresh cancellation preserving prior results. This does not
establish physical-phone performance. Actual deployed offline reopening, remote
rollback, full peak-memory and import/decompression stress acceptance remain open.
Export buttons were invoked, but downloaded bytes have not yet been verified.
