# Static Pages rollout

The repository serves documentation at
`https://giodl73-repo.github.io/ICELINES/` from `gh-pages:/`.
The static WASM workbench lives at
`https://giodl73-repo.github.io/ICELINES/workbench/`.
Rust runs in the browser worker; Pages serves HTML, JavaScript, WASM and packages.

## Current publication method

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

## First validation publication

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
confirms a CORS blocker for our browser acquisition. A relay host and operating
owner must now be selected and upstream access tested before relay implementation.
This publication does not close live/device/full peak-memory/offline/update/rollback
release acceptance.

## Storage and rollback

Storage and service-worker scope are deployment-path specific. Local preview
saves do not automatically transfer to the public path; use package export/import.
Keep downloaded previous-good artifacts beyond Actions retention. For subsequent
updates, restore verified prior workbench bytes alongside the current docs in a
new normal gh-pages commit. Do not reset the entire documentation branch to an
old commit. Verify saved-data compatibility and update/reload behavior before
claiming rollback acceptance. Remote rollback has not been exercised.

The merged browser-pages and browser-rollback workflows provide an alternative
Actions-based publication path. That path would need a deliberate Pages source
migration and a master environment allowance; it is not the current default.
Its retained-master artifact guard and verification remain useful, but the
Actions rollback workflow is not proof of branch-method rollback.
