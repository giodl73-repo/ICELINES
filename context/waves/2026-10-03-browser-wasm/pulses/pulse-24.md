# Browser WASM pulse 24 — preserve the existing Pages site

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-06 preparation. Previous turn
was progress: distributed-WASM/native-loader parity gate. Goal remains active.

## External state and finding

Read-only GitHub API inspection of the actual configured remote,
`giodl73-repo/ICELINES`, reported a built public Pages site at
`https://giodl73-repo.github.io/ICELINES/`, legacy publishing from `gh-pages:/`.
The browser workflow is not yet present in the remote workflow list. The
`gh-pages` branch commit was `79dfba2eeef4aa478955d53558bbaef83f205016` and
contains root documentation, assets, teams, plans, specs and search.

The local browser workflow originally uploaded its standalone distribution as
the whole Pages site. That would omit existing documentation. It now checks
out the existing site on an authorized manual publication and stages the browser
under `workbench/`, preserving existing site files. Pages source configuration
is not changed by this work; legacy-to-Actions migration is an explicit rollout
step. No remote mutation, commit push, workflow dispatch or deployment occurred.

## Change and verification

`stage-browser-pages.py` composes a fresh output from the existing site and
browser distribution. It excludes Git metadata and previous workbench files,
refuses symlinks/overlapping inputs/existing outputs, copies each file and
verifies hashes. It preserves the root and adds `.nojekyll`. The publication
workflow retains its preservation inventory. New test paths trigger the workflow,
and four staging tests run in the checked build pipeline.

`python -m unittest discover -s scripts/tests -p test_browser_pages.py` passed
four cases: docs preservation/workbench replacement/Git exclusion, existing
output refusal, overlapping output refusal, and missing baseline refusal.

A shallow checkout of the real `gh-pages` branch into
`target/browser-pages-baseline-24` was staged with the current distribution to
`target/browser-pages-site-24`. All 95 existing files were copied and hash
verified; 101 browser files were added. The root index digest was unchanged:
`cb48f1f72ac9ae2c75a4362051da61b2260caaffb4946ccf11d366e3258b1628`.
Inventory: `target/browser-pages-preservation-24.json`.

`icelines-browser/PAGES.md` records the exact production workbench URL,
configuration migration, preview/deployment checks, storage-path consequence,
artifact retention and rollback preparation. A saved artifact is distinguished
from an exercised rollback; master rebuild publication is not artifact restore.

## Remaining gates

Actual project-subpath browser proof, GitHub Linux execution, Pages source
migration, coordinated documentation publisher, deployment and rollback remain
open. Live browser access, offline reopening and mobile/resource acceptance
also remain open. This closes an identified destructive-layout risk, not release
acceptance. No site replacement or working public browser is claimed.
