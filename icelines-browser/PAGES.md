# Pages rollout

The repository already serves documentation at
`https://giodl73-repo.github.io/ICELINES/` from the `gh-pages` branch.
The browser belongs at `https://giodl73-repo.github.io/ICELINES/workbench/`.
Do not deploy the standalone browser distribution as the site root.

The prepared workflow builds the browser on PR/master and retains its preview
artifact. A manual master dispatch with `publish=true` checks out the existing
`gh-pages` site, composes it with the browser under `workbench/`, verifies every
copied file's hash, and uploads that combined site. `.git` is excluded; a previous
workbench subtree is replaced, while documentation remains byte-identical.
The preservation inventory is retained as a separate artifact.

## Before first deployment

1. Land the workflow and dependencies through the repository's review process.
   The workflow is currently local and cannot be dispatched on GitHub yet.
2. Complete local artifact review and preparation checks before a validation
   deployment. Exact deployed-origin live/offline/update checks require an HTTPS
   deployment and therefore follow it. A validation deployment needs deliberate
   approval and does not establish release acceptance. Build/test success alone
   does not approve publication.
3. Inspect the retained browser artifact under the representative
   `/ICELINES/workbench/` subpath. Verify worker/WASM/catalog loading, deep-link
   reload, saved-data recovery and root documentation links.
4. Record the current `gh-pages` commit and preserve the existing site. Configure
   repository Pages Source to **GitHub Actions** deliberately: the API observation
   on 2026-10-04 reported legacy branch mode. The workflow does not silently change
   this setting. Coordinate existing documentation publishers with this change;
   `gh-pages` remains the documentation input, not the Actions deployment output.
5. Configure the `github-pages` environment review rules, dispatch on master with
   publish enabled, and verify both root docs and workbench on the exact HTTPS
   origin. Record the deployed build, source commit and preservation inventory.
6. Test stats and schedule separately from the deployed origin. If direct access
   fails, choose relay hosting and operating ownership before implementation, per
   the user's decision. Complete live refresh, deployed offline/update, physical
   mobile/resource and remote rollback checks before accepting the first release.

Browser storage and service-worker scope are deployment-path specific. Saves
made in a local preview or at the site root do not automatically move into the
new workbench path. Users transfer those datasets through package export/import.

## Rollback preparation

Keep a downloaded previous-good `icelines-browser-<commit>` artifact beyond the
30-day Actions retention when it is the chosen recovery build. Also preserve
the deployed composed site and its preservation inventory. Rebuilding an old
commit is not proof of the same published bytes.

For a rollback rehearsal, download the previous-good browser artifact into a
new local directory and compose it with the current documentation baseline:

```powershell
gh run download <previous-good-run-id> --name icelines-browser-<commit> --dir target/rollback-browser
python scripts/stage-browser-pages.py target/pages-baseline target/rollback-browser target/rollback-site
```

The staging output must be new. Verify the old browser's manifest and binary
identity, preserve current documentation, and test saved-data compatibility and
update/reload behavior before deploying that exact composed artifact. A reviewed
rollback dispatch/deployment and its browser acceptance are still release gates;
the normal publish workflow rebuilds master. The prepared
`browser-rollback.yml` workflow restores retained bytes instead: dispatch on
master with the successful source run ID, full source commit, and reviewed shell
build SHA-256. It accepts only successful same-repository master browser builds,
verifies clean source identity and shell/worker/WASM/package integrity without
executing artifact code, and composes it with current documentation. The same
github-pages environment review applies; publication and rollback share one
concurrency group. It has not run remotely. Actual deployment and saved-data
compatibility evidence remain required before rollback is proven.
